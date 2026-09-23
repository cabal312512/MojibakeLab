use crate::streaming;
use mojibake_core::{
    codec::{self, Codec},
    CaseAnalysis, RecoveryCandidate,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub const SAMPLE_LIMIT: usize = 32 * 1024;
pub const PASTE_LIMIT: usize = 32 * 1024;

// ref: cadf64b6-4b48-41a8-8c48-f3014691058c / f
fn preview_codec(head: &[u8]) -> Option<Codec> {
    if let Some((encoding, _)) = codec::detect_bom(head) {
        return Some(encoding);
    }
    match mojibake_core::evidence::null_pattern(head) {
        "utf16le" => return Some(Codec::Utf16Le),
        "utf16be" => return Some(Codec::Utf16Be),
        _ => {}
    }
    match std::str::from_utf8(head) {
        Ok(_) => Some(Codec::Utf8),
        Err(error) if error.error_len().is_none() => Some(Codec::Utf8),
        _ => Codec::parse(mojibake_core::evidence::detector_label_prefix(head)),
    }
}

/// Trim only a cutoff sequence that the following source bytes actually finish.
/// A valid shortened prefix alone is insufficient: it could hide a real bad byte.
fn trim_incomplete_sample(head: &mut Vec<u8>, lookahead: &[u8], encoding: Codec) {
    let bom_length = codec::detect_bom(head).map_or(0, |(_, length)| length);
    let raw = &head[bom_length..];
    if !codec::decode(raw, encoding).had_errors {
        return;
    }
    let mut extended = raw.to_vec();
    extended.extend_from_slice(lookahead);
    let completed = (1..=lookahead.len())
        .any(|length| !codec::decode(&extended[..raw.len() + length], encoding).had_errors);
    if !completed {
        return;
    }
    // Supported codecs need at most four bytes per encoded character.
    if let Some(trim) = (1..=3.min(raw.len()))
        .find(|&trim| !codec::decode(&raw[..raw.len() - trim], encoding).had_errors)
    {
        head.truncate(head.len() - trim);
    }
}

#[derive(Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    size: u64,
    modified: Option<SystemTime>,
    head_hash: Vec<u8>,
    head_length: usize,
    pub bom_length: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HexRow {
    offset: u64,
    hex: String,
    ascii: String,
}

fn io_error(error: std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => "permission_denied".into(),
        std::io::ErrorKind::NotFound => "file_not_found".into(),
        std::io::ErrorKind::AlreadyExists => "output_exists".into(),
        _ => error.to_string(),
    }
}

pub fn analyze(path: &Path) -> Result<(CaseAnalysis, SourceFile), String> {
    let path = path.canonicalize().map_err(io_error)?;
    let mut file = File::open(&path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() {
        return Err("not_a_file".into());
    }
    let mut head = Vec::with_capacity(SAMPLE_LIMIT);
    (&mut file)
        .take(SAMPLE_LIMIT as u64)
        .read_to_end(&mut head)
        .map_err(io_error)?;
    if head.starts_with(&[0xFF, 0xFE, 0, 0]) || head.starts_with(&[0, 0, 0xFE, 0xFF]) {
        return Err("unsupported_encoding".into());
    }
    let sampled = metadata.len() > head.len() as u64;
    let bom = codec::detect_bom(&head);
    let source = SourceFile {
        path: path.clone(),
        size: metadata.len(),
        modified: metadata.modified().ok(),
        head_hash: Sha256::digest(&head).to_vec(),
        head_length: head.len(),
        bom_length: bom.map(|(_, size)| size).unwrap_or(0),
    };
    // The digest above covers the actual source prefix, including bytes omitted
    // from a character-aligned preview. Lookahead is never discarded on export.
    if sampled {
        if let Some(encoding) = preview_codec(&head) {
            let mut lookahead = Vec::with_capacity(4);
            (&mut file)
                .take(4)
                .read_to_end(&mut lookahead)
                .map_err(io_error)?;
            trim_incomplete_sample(&mut head, &lookahead, encoding);
        }
    }
    let name = path.file_name().map(|s| s.to_string_lossy().to_string());
    let mut analysis = mojibake_core::analyze_bytes(&head, name);
    analysis.file_size = Some(metadata.len());
    analysis.evidence.sampled = sampled;
    if sampled {
        analysis.warnings.push("sampled_file".into());
        // Independently inspect middle/tail, never concatenate disjoint bytes into a text.
        for offset in [metadata.len() / 2, metadata.len().saturating_sub(4096)] {
            file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
            let mut sample = vec![0; 4096];
            let n = file.read(&mut sample).map_err(io_error)?;
            sample.truncate(n);
            if sample.starts_with(b"\x89PNG") || sample.starts_with(b"PK\x03\x04") {
                analysis.warnings.push("heterogeneous_samples".into());
            }
            // A UTF-8 middle sample can begin/end mid-codepoint. Ignore only up to 3
            // boundary bytes and retain any invalid interior as mixed-encoding evidence.
            if analysis.evidence.valid_utf8 == Some(true)
                && preview_codec(&head) == Some(Codec::Utf8)
            {
                let start = sample
                    .iter()
                    .take(3)
                    .take_while(|b| **b & 0xc0 == 0x80)
                    .count();
                if let Err(error) = std::str::from_utf8(&sample[start..]) {
                    let reaches_eof = offset + n as u64 == metadata.len();
                    if error.error_len().is_some() || reaches_eof {
                        analysis.warnings.push("heterogeneous_samples".into());
                    }
                }
            }
        }
        analysis.warnings.sort();
        analysis.warnings.dedup();
    }
    Ok((analysis, source))
}

impl SourceFile {
    fn verified_open(&self) -> Result<File, String> {
        let mut file = File::open(&self.path).map_err(io_error)?;
        self.check_metadata(&file)?;
        let mut head = vec![0; self.head_length];
        file.read_exact(&mut head).map_err(|_| "source_changed")?;
        if Sha256::digest(&head).as_slice() != self.head_hash {
            return Err("source_changed".into());
        }
        file.seek(SeekFrom::Start(0)).map_err(io_error)?;
        Ok(file)
    }
    fn check_metadata(&self, file: &File) -> Result<(), String> {
        let now = file.metadata().map_err(io_error)?;
        if now.len() != self.size || now.modified().ok() != self.modified {
            return Err("source_changed".into());
        }
        Ok(())
    }
}

pub fn read_hex(source: &SourceFile, offset: u64) -> Result<Vec<HexRow>, String> {
    let mut file = source.verified_open()?;
    file.seek(SeekFrom::Start(offset.min(source.size)))
        .map_err(io_error)?;
    let mut bytes = vec![0; 4096];
    let n = file.read(&mut bytes).map_err(io_error)?;
    bytes.truncate(n);
    Ok(bytes
        .chunks(16)
        .enumerate()
        .map(|(i, chunk)| HexRow {
            offset: offset + i as u64 * 16,
            hex: chunk
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" "),
            ascii: chunk
                .iter()
                .map(|b| {
                    if (32..127).contains(b) {
                        char::from(*b)
                    } else {
                        '.'
                    }
                })
                .collect(),
        })
        .collect())
}

/// create_new also rejects aliases/hard links to the source and any existing destination.
pub fn save(
    path: &Path,
    source: Option<&SourceFile>,
    candidate: &RecoveryCandidate,
    sampled: bool,
) -> Result<u64, String> {
    let input = source.map(SourceFile::verified_open).transpose()?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io_error)?;
    let result = (|| {
        let written = if sampled {
            let mut input = input.ok_or("source_unavailable")?;
            let source = source.ok_or("source_unavailable")?;
            let bytes = streaming::convert(
                &mut input,
                &mut output,
                &candidate.transformations,
                source.bom_length,
            )?;
            source.check_metadata(&input)?;
            bytes
        } else {
            output
                .write_all(candidate.full_text.as_bytes())
                .map_err(io_error)?;
            candidate.full_text.len() as u64
        };
        output.sync_all().map_err(io_error)?;
        Ok(written)
    })();
    drop(output);
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_cutoffs_preserve_complete_characters_for_supported_codecs() {
        for encoding in codec::CODECS {
            let text = if encoding == Codec::Latin1 || encoding == Codec::Windows1252 {
                "A\u{e9}123\r\n"
            } else if matches!(
                encoding,
                Codec::Utf8 | Codec::Utf16Le | Codec::Utf16Be | Codec::Gb18030
            ) {
                "A中文🧪123\r\n"
            } else {
                "A中文123\r\n"
            };
            let bytes = codec::encode(text, encoding).expect("encodable fixture");
            for end in 1..bytes.len() {
                let mut head = bytes[..end].to_vec();
                trim_incomplete_sample(
                    &mut head,
                    &bytes[end..(end + 4).min(bytes.len())],
                    encoding,
                );
                assert!(
                    !codec::decode(&head, encoding).had_errors,
                    "{} at {end}",
                    encoding.name()
                );
                assert!(bytes.starts_with(&head));
                assert!(end - head.len() <= 3);
            }
        }
    }
    #[test]
    fn preview_does_not_hide_a_real_malformed_suffix_or_interior() {
        for (head, tail) in [
            (b"valid\xff".as_slice(), b"more".as_slice()),
            (b"valid\xe4\xb8".as_slice(), b"!tail".as_slice()),
            (b"bad\xfftail\xe4\xb8".as_slice(), b"\xadend".as_slice()),
        ] {
            let mut preview = head.to_vec();
            trim_incomplete_sample(&mut preview, tail, Codec::Utf8);
            assert_eq!(preview, head);
        }
    }
    #[test]
    fn detected_legacy_samples_end_on_real_character_boundaries() {
        for (encoding, text) in [
            (Codec::Gbk, "中文文件编码测试。你好，世界。\r\n"),
            (Codec::Gb18030, "中文文件编码测试🧪。你好，世界。\r\n"),
            (Codec::Big5, "中文測試檔案，早安。世界您好。\r\n"),
            (
                Codec::ShiftJis,
                "日本語の文字コードをテストします。こんにちは。\r\n",
            ),
            (
                Codec::EucJp,
                "日本語の文字コードをテストします。こんにちは。\r\n",
            ),
            (Codec::EucKr, "한글 인코딩 테스트입니다. 안녕하세요.\r\n"),
        ] {
            let body = codec::encode(&text.repeat(3000), encoding).expect("fixture");
            let bytes = (0..4)
                .find_map(|prefix| {
                    let mut bytes = vec![b'A'; prefix];
                    bytes.extend_from_slice(&body);
                    codec::decode(&bytes[..SAMPLE_LIMIT], encoding)
                        .had_errors
                        .then_some(bytes)
                })
                .expect("fixture cuts a character");
            let mut head = bytes[..SAMPLE_LIMIT].to_vec();
            let suggested = preview_codec(&head).expect("supported detector suggestion");
            trim_incomplete_sample(&mut head, &bytes[SAMPLE_LIMIT..SAMPLE_LIMIT + 4], suggested);
            assert!(
                !codec::decode(&head, encoding).had_errors,
                "{} suggested {}",
                encoding.name(),
                suggested.name()
            );
        }
    }
    #[test]
    fn utf8_bom_sample_boundary_does_not_invent_information_loss() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("utf8-bom.txt");
        let text = format!("{}中文", "A".repeat(SAMPLE_LIMIT - 4));
        let mut bytes = b"\xef\xbb\xbf".to_vec();
        bytes.extend_from_slice(text.as_bytes());
        fs::write(&path, &bytes).expect("source");
        let (case, source) = analyze(&path).expect("analysis");
        assert!(case.evidence.sampled);
        assert_eq!(case.evidence.valid_utf8, Some(true));
        assert_eq!(case.evidence.replacement_count, 0);
        assert!(!case
            .warnings
            .iter()
            .any(|warning| warning == "information_loss"));
        let candidate = case
            .candidates
            .iter()
            .find(|candidate| {
                candidate.transformations.len() == 1
                    && candidate.transformations[0].encoding == "UTF-8"
            })
            .expect("UTF-8 candidate");
        let output = dir.path().join("out.txt");
        save(&output, Some(&source), candidate, true).expect("full export");
        assert_eq!(fs::read(&output).expect("output"), text.as_bytes());
    }
    #[test]
    fn unmarked_utf16_surrogate_cutoff_is_only_a_preview_boundary() {
        let dir = tempfile::tempdir().expect("temp");
        for encoding in [Codec::Utf16Le, Codec::Utf16Be] {
            let path = dir.path().join(format!("{}.txt", encoding.name()));
            let text = format!("{}🧪", "A".repeat((SAMPLE_LIMIT - 2) / 2));
            fs::write(&path, codec::encode(&text, encoding).expect("fixture")).expect("source");
            let (case, source) = analyze(&path).expect("analysis");
            assert!(case.evidence.sampled);
            assert_eq!(case.evidence.replacement_count, 0, "{}", encoding.name());
            assert!(
                !case.warnings.iter().any(|warning| warning == "heterogeneous_samples"),
                "{} must not be checked as UTF-8 merely because its ASCII prefix is byte-valid UTF-8",
                encoding.name()
            );
            let candidate = case
                .candidates
                .iter()
                .find(|candidate| {
                    candidate.transformations.len() == 1
                        && candidate.transformations[0].encoding == encoding.name()
                })
                .expect("UTF-16 candidate");
            let output = dir.path().join(format!("{}.out.txt", encoding.name()));
            save(&output, Some(&source), candidate, true).expect("full export");
            assert_eq!(fs::read(&output).expect("output"), text.as_bytes());
        }
    }
    #[test]
    fn malformed_real_eof_is_not_trimmed_as_a_sample() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("incomplete.txt");
        fs::write(&path, b"\xef\xbb\xbfvalid\xe4\xb8").expect("source");
        let (case, _) = analyze(&path).expect("analysis");
        assert!(!case.evidence.sampled);
        assert_eq!(case.evidence.replacement_count, 1);
        assert!(case.original_text.ends_with('\u{fffd}'));
    }
    #[test]
    fn never_overwrites_source_or_existing_output() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("source.txt");
        fs::write(&path, b"Hello").expect("source");
        let (case, source) = analyze(&path).expect("analysis");
        assert_eq!(
            save(&path, Some(&source), &case.candidates[0], false),
            Err("output_exists".into())
        );
        assert_eq!(fs::read(&path).expect("read"), b"Hello");
    }
    #[test]
    fn changed_source_is_rejected() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("source.txt");
        fs::write(&path, b"Hello").expect("source");
        let (case, source) = analyze(&path).expect("analysis");
        fs::write(&path, b"Changed").expect("change");
        assert_eq!(
            save(
                &dir.path().join("out.txt"),
                Some(&source),
                &case.candidates[0],
                false
            ),
            Err("source_changed".into())
        );
    }
    #[test]
    fn large_file_is_sampled_and_saved_in_full() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("large.txt");
        let text = "Hello 中文\r\n".repeat(20000);
        fs::write(&path, text.as_bytes()).expect("source");
        let (case, source) = analyze(&path).expect("analysis");
        assert!(case.evidence.sampled);
        assert!(case.original_text.len() <= SAMPLE_LIMIT * 3);
        let out = dir.path().join("out.txt");
        save(&out, Some(&source), &case.candidates[0], true).expect("save");
        assert_eq!(fs::read(&out).expect("output"), text.as_bytes());
    }
    #[test]
    fn late_conversion_failure_removes_partial_output() {
        let dir = tempfile::tempdir().expect("temp");
        for (index, suffix) in [b"\xff".as_slice(), b"\xe4\xb8".as_slice()]
            .iter()
            .enumerate()
        {
            let path = dir.path().join(format!("mixed-{index}.txt"));
            let mut input = b"Hello\n".repeat(10000);
            input.extend_from_slice(suffix);
            fs::write(&path, input).expect("source");
            let (case, source) = analyze(&path).expect("analysis");
            assert!(case
                .warnings
                .iter()
                .any(|warning| warning == "heterogeneous_samples"));
            let utf8 = case
                .candidates
                .iter()
                .find(|c| {
                    c.transformations
                        .first()
                        .is_some_and(|s| s.encoding == "UTF-8")
                })
                .expect("utf8");
            let out = dir.path().join(format!("out-{index}.txt"));
            assert_eq!(
                save(&out, Some(&source), utf8, true),
                Err("export_loss_detected".into())
            );
            assert!(!out.exists());
        }
    }
}
