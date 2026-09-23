use mojibake_core::codec::{decode, encode, Codec};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    fs::create_dir_all(&root)?;
    let chinese = "中文测试文件\nMojibake Lab 2026\n";
    let cp1252 = decode(chinese.as_bytes(), Codec::Windows1252).text;
    let gbk = decode("中文测试".as_bytes(), Codec::Gbk).text;
    let double = decode(cp1252.as_bytes(), Codec::Windows1252).text;
    let mut utf8_bom = vec![0xEF, 0xBB, 0xBF];
    utf8_bom.extend_from_slice(chinese.as_bytes());
    let fixtures: Vec<(&str, Vec<u8>)> = vec![
        ("utf8.txt", chinese.as_bytes().to_vec()),
        ("utf8_bom.txt", utf8_bom),
        ("utf16le.txt", encode(chinese, Codec::Utf16Le)?),
        ("utf16be.txt", encode(chinese, Codec::Utf16Be)?),
        (
            "gb18030.txt",
            encode("中文𠀀🧪\nMojibake Lab", Codec::Gb18030)?,
        ),
        (
            "big5.txt",
            encode("繁體中文測試\nMojibake Lab", Codec::Big5)?,
        ),
        (
            "shift_jis.txt",
            encode("日本語のテスト\nMojibake Lab", Codec::ShiftJis)?,
        ),
        (
            "windows1252.txt",
            encode("café — €25\nMojibake Lab", Codec::Windows1252)?,
        ),
        ("utf8_as_windows1252.txt", cp1252.into_bytes()),
        ("utf8_as_gbk.txt", gbk.into_bytes()),
        ("double_mojibake.txt", double.into_bytes()),
        (
            "replacement_loss.txt",
            "中�文：信息已丢失\n".as_bytes().to_vec(),
        ),
        (
            "mixed_language.txt",
            "中文 / 日本語 / 한국어 / café / 🧪\n".as_bytes().to_vec(),
        ),
        ("ascii_only.txt", b"key=value\ncount=42\n".to_vec()),
    ];
    for (name, bytes) in fixtures {
        fs::write(root.join(name), bytes)?;
    }
    println!("Fixtures written to {}", root.display());
    Ok(())
}
