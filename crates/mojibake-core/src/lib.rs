//! Deterministic offline encoding forensics. All I/O belongs to the caller.
pub mod codec;
pub mod evidence;
mod models;
pub mod scoring;
pub mod search;

use codec::Codec;
pub use models::*;
use std::time::Instant;

pub const MAX_SAMPLE_BYTES: usize = 32 * 1024;

/// Analyze a bounded byte prefix. The caller owns any additional head/middle/
/// tail sampling and must replay a chosen path over the complete file on export.
pub fn analyze_bytes(bytes: &[u8], file_name: Option<String>) -> CaseAnalysis {
    let start = Instant::now();
    let full_size = bytes.len();
    let sampled = full_size > MAX_SAMPLE_BYTES;
    let bytes = &bytes[..full_size.min(MAX_SAMPLE_BYTES)];
    let bom = codec::detect_bom(bytes);
    let raw = &bytes[bom.map_or(0, |(_, count)| count)..];
    let suggestion = Codec::parse(evidence::detector_label(bytes));
    let initial_codec = bom.map_or_else(
        || {
            if matches!(suggestion, Some(Codec::Utf16Le | Codec::Utf16Be)) {
                suggestion.unwrap_or(Codec::Utf8)
            } else if std::str::from_utf8(raw).is_ok() {
                Codec::Utf8
            } else {
                suggestion.unwrap_or(Codec::Windows1252)
            }
        },
        |(codec, _)| codec,
    );
    let original_text = codec::decode(raw, initial_codec).text;
    let evidence = evidence::build_evidence(&original_text, Some(bytes), sampled);
    let utf16_pattern = match evidence.null_byte_pattern.as_str() {
        "utf16le" => Some(Codec::Utf16Le),
        "utf16be" => Some(Codec::Utf16Be),
        _ => None,
    };
    let (candidates, explored_states) = if evidence.binary {
        (Vec::new(), 0)
    } else {
        search::search(
            &original_text,
            Some(raw),
            bom.map(|(codec, _)| codec),
            suggestion,
            utf16_pattern,
        )
    };
    let warnings = case_warnings(&original_text, &evidence, &candidates, false);
    CaseAnalysis {
        case_id: format!("file-{:016x}", search::stable_hash(bytes)),
        source_type: "file".into(),
        file_name,
        file_size: Some(full_size as u64),
        original_text,
        evidence,
        candidates,
        warnings,
        explored_states,
        elapsed_ms: start.elapsed().as_millis() as u64,
    }
}

pub fn analyze_text(text: &str) -> CaseAnalysis {
    let start = Instant::now();
    let mut end = text.len().min(MAX_SAMPLE_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let sampled = end < text.len();
    let text = &text[..end];
    let evidence = evidence::build_evidence(text, None, sampled);
    let (candidates, explored_states) = search::search(text, None, None, None, None);
    let warnings = case_warnings(text, &evidence, &candidates, true);
    CaseAnalysis {
        case_id: format!("paste-{:016x}", search::stable_hash(text.as_bytes())),
        source_type: "paste".into(),
        file_name: None,
        file_size: None,
        original_text: text.into(),
        evidence,
        candidates,
        warnings,
        explored_states,
        elapsed_ms: start.elapsed().as_millis() as u64,
    }
}

fn case_warnings(
    text: &str,
    evidence: &EncodingEvidence,
    candidates: &[RecoveryCandidate],
    paste: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if paste {
        warnings.push("original_bytes_unavailable".into());
    }
    if evidence.binary {
        if evidence
            .bom
            .as_deref()
            .is_some_and(|bom| bom.starts_with("UTF-32"))
        {
            warnings.push("unsupported_encoding".into());
        } else {
            warnings.push("binary_file".into());
        }
        return warnings;
    }
    if text.is_empty() {
        warnings.push("empty_input".into());
    }
    if evidence.sampled {
        warnings.push("sampled_input".into());
    }
    if evidence.replacement_count > 0 {
        warnings.push("information_loss".into());
    }
    if candidates
        .first()
        .is_some_and(|candidate| candidate.full_text == text && evidence::metrics(text).clean())
    {
        warnings.push("no_obvious_corruption".into());
    }
    if candidates.len() > 1 && candidates[0].score - candidates[1].score < 4.0 {
        warnings.push("ambiguous_result".into());
    }
    warnings
}

/// Samples are generated locally from known originals using the same explicit
/// byte/text boundaries as real input. No assets are downloaded at runtime.
pub fn analyze_sample(id: &str) -> Result<CaseAnalysis, String> {
    let chinese = "中文测试：你好，世界。\nMojibake Lab — 离线编码取证。";
    let mut analysis = match id {
        "cp1252" => {
            analyze_text(&codec::decode("中文测试文件".as_bytes(), Codec::Windows1252).text)
        }
        "gbk" => analyze_text(&codec::decode("中文测试".as_bytes(), Codec::Gbk).text),
        "double" => {
            let once = codec::decode("中文测试文件".as_bytes(), Codec::Windows1252).text;
            let twice = codec::decode(once.as_bytes(), Codec::Windows1252).text;
            analyze_text(&twice)
        }
        "utf16" => analyze_bytes(
            &codec::encode(chinese, Codec::Utf16Le)?,
            Some("utf16-no-bom.txt".into()),
        ),
        "loss" => analyze_text(
            "记录 2026-09-23\n用户：中�文 / 状态：�失\nOriginal bytes are no longer available.",
        ),
        "clean" => analyze_bytes(chinese.as_bytes(), Some("already-utf8.txt".into())),
        _ => return Err("unknown_sample".into()),
    };
    analysis.source_type = "sample".into();
    Ok(analysis)
}
