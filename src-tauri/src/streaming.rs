//! Stateful, bounded export. Each decoder owns its partial multibyte sequence.
use encoding_rs::{CoderResult, Decoder};
use mojibake_core::{
    codec::{self, Codec},
    TransformationStep,
};
use std::io::{Read, Write};

enum Stage {
    Decode {
        decoder: Option<Decoder>,
        encoding: Codec,
        pending: Vec<u8>,
    },
    Encode(Codec),
}

enum Chunk {
    Bytes(Vec<u8>),
    Text(String),
}

impl Stage {
    fn apply(&mut self, input: Chunk, last: bool) -> Result<Chunk, String> {
        match (self, input) {
            (Stage::Decode { decoder: None, .. }, Chunk::Bytes(bytes)) => {
                Ok(Chunk::Text(bytes.into_iter().map(char::from).collect()))
            }
            (
                Stage::Decode {
                    decoder: Some(decoder),
                    encoding,
                    pending,
                },
                Chunk::Bytes(bytes),
            ) => {
                pending.extend_from_slice(&bytes);
                let mut text = String::with_capacity(bytes.len().saturating_mul(3) + 32);
                let mut position = 0;
                loop {
                    text.reserve(bytes.len().saturating_mul(3) + 32);
                    let (status, read, errors) =
                        decoder.decode_to_string(&bytes[position..], &mut text, last);
                    if errors {
                        return Err("export_loss_detected".into());
                    }
                    position += read;
                    if status == CoderResult::InputEmpty {
                        break;
                    }
                }
                // Check canonicalization too, not only malformed sequences. A
                // decoder may consume partial bytes without emitting text yet.
                let restored =
                    codec::encode(&text, *encoding).map_err(|_| "export_loss_detected")?;
                if !pending.starts_with(&restored) {
                    return Err("export_loss_detected".into());
                }
                pending.drain(..restored.len());
                if (last && !pending.is_empty()) || pending.len() > 16 {
                    return Err("export_loss_detected".into());
                }
                Ok(Chunk::Text(text))
            }
            (Stage::Encode(encoding), Chunk::Text(text)) => {
                let bytes = codec::encode(&text, *encoding).map_err(|_| "export_loss_detected")?;
                if codec::decode(&bytes, *encoding).text != text {
                    return Err("export_loss_detected".into());
                }
                Ok(Chunk::Bytes(bytes))
            }
            _ => Err("invalid_transformation".into()),
        }
    }
}

pub fn convert<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    steps: &[TransformationStep],
    skip_bom: usize,
) -> Result<u64, String> {
    let mut stages = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let encoding = Codec::parse(&step.encoding).ok_or("unsupported_encoding")?;
        let expect_decode = index % 2 == 0;
        if (step.operation == "decode") != expect_decode {
            return Err("invalid_transformation".into());
        }
        stages.push(if expect_decode {
            Stage::Decode {
                decoder: encoding
                    .encoding()
                    .map(|enc| enc.new_decoder_without_bom_handling()),
                encoding,
                pending: Vec::new(),
            }
        } else {
            Stage::Encode(encoding)
        });
    }
    if stages.is_empty() || stages.len() % 2 == 0 {
        return Err("invalid_transformation".into());
    }
    let mut bom = vec![0; skip_bom];
    reader.read_exact(&mut bom).map_err(|_| "source_changed")?;
    let mut buffer = [0; 16 * 1024];
    let mut written = 0u64;
    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        let last = count == 0;
        let mut chunk = Chunk::Bytes(buffer[..count].to_vec());
        for stage in &mut stages {
            chunk = stage.apply(chunk, last)?;
        }
        match chunk {
            Chunk::Text(text) => {
                writer
                    .write_all(text.as_bytes())
                    .map_err(|e| e.to_string())?;
                written += text.len() as u64;
            }
            Chunk::Bytes(_) => return Err("invalid_transformation".into()),
        }
        if last {
            break;
        }
    }
    writer.flush().map_err(|e| e.to_string())?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn steps(labels: &[(&str, &str)]) -> Vec<TransformationStep> {
        labels
            .iter()
            .map(|(operation, encoding)| {
                serde_json::from_value(serde_json::json!({
                    "operation":operation,"encoding":encoding,
                    "inputType":if *operation=="decode" {"bytes"} else {"text"},
                    "outputType":if *operation=="decode" {"text"} else {"bytes"},
                    "inputPreview":"","outputPreview":"","inputSize":0,"outputSize":0,
                    "lossy":false,"replacementCount":0
                }))
                .expect("step fixture")
            })
            .collect()
    }
    struct TinyReader<R>(R);
    impl<R: Read> Read for TinyReader<R> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(&mut out[..1])
        }
    }
    #[test]
    fn every_byte_boundary_utf16_and_surrogates() {
        let text = "Hello 中文 🧪\r\n日本語";
        for encoding in [Codec::Utf16Le, Codec::Utf16Be, Codec::Utf8, Codec::Gb18030] {
            let bytes = codec::encode(text, encoding).expect("fixture encodes");
            let mut output = Vec::new();
            convert(
                TinyReader(Cursor::new(bytes)),
                &mut output,
                &steps(&[("decode", encoding.name())]),
                0,
            )
            .expect("stream");
            assert_eq!(output, text.as_bytes());
        }
    }
    #[test]
    fn repair_pair_carries_incomplete_utf8_between_chunks() {
        let damaged = "ä¸­æ–‡".repeat(10000);
        let mut output = Vec::new();
        convert(
            TinyReader(Cursor::new(damaged.as_bytes())),
            &mut output,
            &steps(&[
                ("decode", "UTF-8"),
                ("encode", "Windows-1252"),
                ("decode", "UTF-8"),
            ]),
            0,
        )
        .expect("repair");
        assert_eq!(output, "中文".repeat(10000).as_bytes());
    }
    #[test]
    fn invalid_tail_is_not_silently_replaced() {
        let mut output = Vec::new();
        let result = convert(
            Cursor::new(b"valid\xe4\xb8"),
            &mut output,
            &steps(&[("decode", "UTF-8")]),
            0,
        );
        assert_eq!(result, Err("export_loss_detected".into()));
    }
    #[test]
    fn literal_latin1_differs_from_cp1252() {
        let mut output = Vec::new();
        convert(
            Cursor::new([0x80]),
            &mut output,
            &steps(&[("decode", "ISO-8859-1")]),
            0,
        )
        .expect("latin1");
        assert_eq!(output, "\u{80}".as_bytes());
    }
    #[test]
    fn encodable_but_non_roundtrip_yen_is_rejected() {
        let mut output = Vec::new();
        let result = convert(
            Cursor::new("¥".as_bytes()),
            &mut output,
            &steps(&[
                ("decode", "UTF-8"),
                ("encode", "Shift_JIS"),
                ("decode", "UTF-8"),
            ]),
            0,
        );
        assert_eq!(result, Err("export_loss_detected".into()));
    }
}
