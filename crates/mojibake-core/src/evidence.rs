use crate::codec::{detect_bom, Codec};
use crate::EncodingEvidence;
use std::collections::BTreeMap;
use unicode_script::{Script, UnicodeScript};

#[derive(Clone, Debug, Default)]
pub(crate) struct Metrics {
    pub count: usize,
    pub replacements: usize,
    pub controls: usize,
    pub ascii: usize,
    pub whitespace: usize,
    pub suspicious: usize,
    pub scripts: BTreeMap<String, usize>,
}

impl Metrics {
    pub fn ratio(&self, count: usize) -> f64 {
        count as f64 / self.count.max(1) as f64
    }
    pub fn clean(&self) -> bool {
        self.replacements == 0 && self.controls == 0 && self.suspicious == 0
    }
}

pub(crate) fn metrics(text: &str) -> Metrics {
    let mut result = Metrics::default();
    let mut previous = None;
    for ch in text.chars() {
        result.count += 1;
        result.replacements += usize::from(ch == '\u{FFFD}');
        result.controls += usize::from(is_control(ch));
        result.ascii += usize::from(ch.is_ascii());
        result.whitespace += usize::from(ch.is_whitespace());
        // A letter such as â is legitimate on its own. Only adjacent patterns
        // characteristic of UTF-8 decoded as a single-byte code page count.
        if let Some(first) = previous {
            if matches!(
                first,
                'Ã' | 'Â'
                    | 'â'
                    | 'ð'
                    | 'ä'
                    | 'å'
                    | 'æ'
                    | 'ç'
                    | 'è'
                    | 'é'
                    | 'ê'
                    | 'ë'
                    | 'ì'
                    | 'ï'
            ) && (matches!(ch as u32, 0x80..=0xBF) || "€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ".contains(ch))
            {
                result.suspicious += 1;
            }
        }
        if matches!(ch as u32, 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD) {
            result.suspicious += 1;
        }
        let name = match ch.script() {
            Script::Han => "Han",
            Script::Hiragana => "Hiragana",
            Script::Katakana => "Katakana",
            Script::Hangul => "Hangul",
            Script::Latin => "Latin",
            Script::Cyrillic => "Cyrillic",
            Script::Arabic => "Arabic",
            Script::Greek => "Greek",
            Script::Hebrew => "Hebrew",
            Script::Common | Script::Inherited => "Common",
            _ => "Other",
        };
        *result.scripts.entry(name.into()).or_default() += 1;
        previous = Some(ch);
    }
    // Short recurring sequences are weak evidence, never a language verdict.
    for pattern in [
        "浣犲",
        "犲ソ",
        "鏂囧",
        "娴嬭",
        "璇曟",
        "锟斤拷",
        "鈥",
        "馃",
        "縺",
        "繧",
        "繝",
    ] {
        result.suspicious += text.matches(pattern).count();
    }
    result
}

pub(crate) fn is_control(ch: char) -> bool {
    ch.is_control() && !matches!(ch, '\t' | '\r' | '\n' | '\u{000C}')
}

pub fn hex_preview(bytes: &[u8], limit: usize) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(limit.min(bytes.len()) * 3);
    for (index, byte) in bytes.iter().take(limit).enumerate() {
        if index != 0 {
            result.push(' ');
        }
        let _ = write!(result, "{byte:02X}");
    }
    if bytes.len() > limit {
        result.push_str(" …");
    }
    result
}

pub fn text_preview(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut result: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}

pub fn null_pattern(bytes: &[u8]) -> &'static str {
    if !bytes.contains(&0) {
        return "none";
    }
    let mut even = 0;
    let mut odd = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == 0 {
            if index % 2 == 0 {
                even += 1;
            } else {
                odd += 1;
            }
        }
    }
    let pairs = (bytes.len() / 2).max(1) as f64;
    if odd as f64 / pairs >= 0.30 && even as f64 / pairs < 0.10 {
        "utf16le"
    } else if even as f64 / pairs >= 0.30 && odd as f64 / pairs < 0.10 {
        "utf16be"
    } else {
        "irregular"
    }
}

pub fn is_binary(bytes: &[u8]) -> bool {
    if crate::codec::unsupported_bom(bytes).is_some() {
        return true;
    }
    let signatures: &[&[u8]] = &[
        b"\x89PNG\r\n\x1a\n",
        b"PK\x03\x04",
        b"PK\x05\x06",
        b"MZ",
        b"%PDF-",
        b"\x7fELF",
        b"\xff\xd8\xff",
        b"GIF87a",
        b"GIF89a",
        b"\x1f\x8b",
        b"\xd0\xcf\x11\xe0",
        b"SQLite format 3\0",
    ];
    if signatures
        .iter()
        .any(|signature| bytes.starts_with(signature))
    {
        return true;
    }
    if detect_bom(bytes).is_some() || matches!(null_pattern(bytes), "utf16le" | "utf16be") {
        return false;
    }
    let length = bytes.len().max(1) as f64;
    let zeros = bytes.iter().filter(|&&byte| byte == 0).count() as f64 / length;
    let controls = bytes
        .iter()
        .filter(|&&byte| byte < 32 && !matches!(byte, 9 | 10 | 12 | 13))
        .count() as f64
        / length;
    zeros > 0.03 || controls > 0.20
}

pub fn detector_label(bytes: &[u8]) -> &'static str {
    detect_label(bytes, true)
}

/// A prefix of a larger file is not end-of-stream. Let the detector retain a
/// trailing multibyte lead as pending input instead of scoring it malformed.
pub fn detector_label_prefix(bytes: &[u8]) -> &'static str {
    detect_label(bytes, false)
}

fn detect_label(bytes: &[u8], complete: bool) -> &'static str {
    if let Some((codec, _)) = detect_bom(bytes) {
        return codec.name();
    }
    match null_pattern(bytes) {
        "utf16le" => return Codec::Utf16Le.name(),
        "utf16be" => return Codec::Utf16Be.name(),
        _ => {}
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, complete);
    detector.guess(None, true).name()
}

pub(crate) fn build_evidence(text: &str, bytes: Option<&[u8]>, sampled: bool) -> EncodingEvidence {
    let values = metrics(text);
    let script_distribution = values
        .scripts
        .iter()
        .map(|(name, count)| (name.clone(), values.ratio(*count)))
        .collect();
    EncodingEvidence {
        bom: bytes
            .and_then(|raw| {
                crate::codec::unsupported_bom(raw)
                    .or_else(|| detect_bom(raw).map(|(codec, _)| codec.name()))
            })
            .map(String::from),
        valid_utf8: bytes.map(|raw| std::str::from_utf8(raw).is_ok()),
        null_byte_pattern: bytes.map(null_pattern).unwrap_or("unavailable").into(),
        replacement_count: values.replacements,
        control_ratio: values.ratio(values.controls),
        ascii_ratio: values.ratio(values.ascii),
        whitespace_ratio: values.ratio(values.whitespace),
        line_count: if text.is_empty() {
            0
        } else {
            text.split('\n').count()
        },
        script_distribution,
        detector_suggestion: bytes
            .filter(|raw| !raw.is_empty())
            .map(|raw| detector_label(raw).into()),
        hex_prefix: bytes.map(|raw| hex_preview(raw, 256)).unwrap_or_default(),
        sampled,
        sample_size: bytes.map_or(text.len(), <[u8]>::len),
        binary: bytes.is_some_and(is_binary),
    }
}
