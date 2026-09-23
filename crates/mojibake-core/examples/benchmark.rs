use mojibake_core::{
    analyze_bytes, analyze_sample, analyze_text,
    codec::{self, Codec},
};

fn main() {
    for id in ["cp1252", "gbk", "double", "utf16", "loss", "clean"] {
        let case = analyze_sample(id).expect("built-in sample");
        println!(
            "{id}: {} ms, {} explored, top {:?}",
            case.elapsed_ms,
            case.explored_states,
            case.candidates
                .first()
                .map(|candidate| &candidate.full_text)
        );
    }
    let clean = "文".repeat(11_000);
    let damaged = codec::decode("中文测试文件\n".repeat(1000).as_bytes(), Codec::Windows1252).text;
    for (name, text) in [("32KiB clean", clean), ("32KiB damaged", damaged)] {
        let case = analyze_text(&text);
        println!(
            "{name}: {} ms, {} explored, {} candidates",
            case.elapsed_ms,
            case.explored_states,
            case.candidates.len()
        );
    }
    let utf16_prefix = "A".repeat((mojibake_core::MAX_SAMPLE_BYTES - 2) / 2);
    for codec in [Codec::Utf16Le, Codec::Utf16Be] {
        let bytes = codec::encode(&utf16_prefix, codec).expect("UTF-16 prefix");
        let case = analyze_bytes(&bytes, None);
        println!(
            "32766-byte {} ASCII prefix: {} ms, {} explored, {} candidates",
            codec.name(),
            case.elapsed_ms,
            case.explored_states,
            case.candidates.len()
        );
    }
}
