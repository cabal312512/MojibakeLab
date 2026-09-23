//! The byte/text boundary. No encoder is allowed to substitute numeric references.
use encoding_rs::{EncoderResult, Encoding};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Codec {
    Utf8,
    Utf16Le,
    Utf16Be,
    Gbk,
    Gb18030,
    Big5,
    ShiftJis,
    EucJp,
    EucKr,
    Windows1252,
    Latin1,
}

pub const CODECS: [Codec; 11] = [
    Codec::Utf8,
    Codec::Windows1252,
    Codec::Latin1,
    Codec::Gbk,
    Codec::Gb18030,
    Codec::Big5,
    Codec::ShiftJis,
    Codec::EucJp,
    Codec::EucKr,
    Codec::Utf16Le,
    Codec::Utf16Be,
];

// ref: cadf64b6-4b48-41a8-8c48-f3014691058c / c
impl Codec {
    pub fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16LE",
            Self::Utf16Be => "UTF-16BE",
            Self::Gbk => "GBK",
            Self::Gb18030 => "GB18030",
            Self::Big5 => "Big5",
            Self::ShiftJis => "Shift_JIS",
            Self::EucJp => "EUC-JP",
            Self::EucKr => "EUC-KR",
            Self::Windows1252 => "Windows-1252",
            Self::Latin1 => "ISO-8859-1",
        }
    }

    pub fn parse(label: &str) -> Option<Self> {
        match label.to_ascii_lowercase().replace(['_', ' '], "-").as_str() {
            "utf-8" | "utf8" | "utf-8-bom" => Some(Self::Utf8),
            "utf-16le" | "utf-16-le" | "utf16le" => Some(Self::Utf16Le),
            "utf-16be" | "utf-16-be" | "utf16be" => Some(Self::Utf16Be),
            "gbk" | "gb2312" => Some(Self::Gbk),
            "gb18030" => Some(Self::Gb18030),
            "big5" | "big-5" => Some(Self::Big5),
            "shift-jis" | "sjis" => Some(Self::ShiftJis),
            "euc-jp" => Some(Self::EucJp),
            "euc-kr" => Some(Self::EucKr),
            "windows-1252" | "cp1252" => Some(Self::Windows1252),
            "iso-8859-1" | "latin-1" | "latin1" => Some(Self::Latin1),
            _ => None,
        }
    }

    /// Latin-1 has deliberately literal semantics, unlike WHATWG label resolution.
    /// UTF-16 encoders must be implemented separately from encoding_rs.
    pub fn encoding(self) -> Option<&'static Encoding> {
        Some(match self {
            Self::Utf8 => encoding_rs::UTF_8,
            Self::Utf16Le => encoding_rs::UTF_16LE,
            Self::Utf16Be => encoding_rs::UTF_16BE,
            Self::Gbk => encoding_rs::GBK,
            Self::Gb18030 => encoding_rs::GB18030,
            Self::Big5 => encoding_rs::BIG5,
            Self::ShiftJis => encoding_rs::SHIFT_JIS,
            Self::EucJp => encoding_rs::EUC_JP,
            Self::EucKr => encoding_rs::EUC_KR,
            Self::Windows1252 => encoding_rs::WINDOWS_1252,
            Self::Latin1 => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Decoded {
    pub text: String,
    pub had_errors: bool,
    /// U+FFFD characters in the output, including any already in the evidence.
    pub replacement_count: usize,
    pub round_trip: bool,
}

pub fn encode(text: &str, codec: Codec) -> Result<Vec<u8>, String> {
    match codec {
        Codec::Utf8 => Ok(text.as_bytes().to_vec()),
        Codec::Utf16Le | Codec::Utf16Be => {
            let mut bytes = Vec::with_capacity(text.len() * 2);
            for unit in text.encode_utf16() {
                bytes.extend(if codec == Codec::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            Ok(bytes)
        }
        Codec::Latin1 => text
            .chars()
            .map(|ch| {
                u8::try_from(ch as u32)
                    .map_err(|_| format!("unmappable:{}:U+{:04X}", codec.name(), ch as u32))
            })
            .collect(),
        _ => {
            let encoding = codec
                .encoding()
                .ok_or_else(|| "unsupported_encoding".to_string())?;
            let mut encoder = encoding.new_encoder();
            let capacity = encoder
                .max_buffer_length_from_utf8_without_replacement(text.len())
                .ok_or_else(|| "input_too_large".to_string())?;
            let mut output = Vec::with_capacity(capacity);
            let (result, _) =
                encoder.encode_from_utf8_to_vec_without_replacement(text, &mut output, true);
            match result {
                EncoderResult::InputEmpty => Ok(output),
                EncoderResult::Unmappable(ch) => {
                    Err(format!("unmappable:{}:U+{:04X}", codec.name(), ch as u32))
                }
                EncoderResult::OutputFull => Err("encoding_buffer_full".into()),
            }
        }
    }
}

pub fn decode(bytes: &[u8], codec: Codec) -> Decoded {
    let (text, had_errors) = if codec == Codec::Latin1 {
        (bytes.iter().map(|&byte| char::from(byte)).collect(), false)
    } else if let Some(encoding) = codec.encoding() {
        let (text, errors) = encoding.decode_without_bom_handling(bytes);
        (text.into_owned(), errors)
    } else {
        (String::new(), true)
    };
    let replacement_count = text.chars().filter(|&ch| ch == '\u{FFFD}').count();
    let round_trip = !had_errors && encode(&text, codec).is_ok_and(|encoded| encoded == bytes);
    Decoded {
        text,
        had_errors,
        replacement_count,
        round_trip,
    }
}

/// BOM removal belongs only at the original evidence boundary. Intermediate
/// transformations must preserve a U+FEFF character like any other character.
pub fn detect_bom(bytes: &[u8]) -> Option<(Codec, usize)> {
    if unsupported_bom(bytes).is_some() {
        None
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        Some((Codec::Utf8, 3))
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        Some((Codec::Utf16Le, 2))
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        Some((Codec::Utf16Be, 2))
    } else {
        None
    }
}

/// UTF-32LE shares its first two BOM bytes with UTF-16LE. Do not invent a
/// supported encoding merely because that shorter prefix matches.
pub fn unsupported_bom(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xFE, 0, 0]) {
        Some("UTF-32LE")
    } else if bytes.starts_with(&[0, 0, 0xFE, 0xFF]) {
        Some("UTF-32BE")
    } else {
        None
    }
}
