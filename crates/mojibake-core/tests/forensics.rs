use mojibake_core::{
    analyze_bytes, analyze_sample, analyze_text,
    codec::{self, Codec, CODECS},
    search::{self, DataState},
};
use std::collections::HashSet;

#[test]
fn codec_round_trips_supported_repertoires() {
    let cases = [
        (Codec::Utf8, "中文 / 日本語 / 한국어 / café / 🧪"),
        (Codec::Utf16Le, "中文 / 🧪"),
        (Codec::Utf16Be, "中文 / 🧪"),
        (Codec::Gbk, "中文测试，离线工具。"),
        (Codec::Gb18030, "中文𠀀🧪"),
        (Codec::Big5, "繁體中文測試"),
        (Codec::ShiftJis, "日本語のテスト"),
        (Codec::EucJp, "日本語のテスト"),
        (Codec::EucKr, "한국어 테스트"),
        (Codec::Windows1252, "café — €25"),
        (Codec::Latin1, "café\u{0080}"),
    ];
    for (codec, original) in cases {
        let encoded = codec::encode(original, codec).expect(codec.name());
        let decoded = codec::decode(&encoded, codec);
        assert_eq!(decoded.text, original, "{}", codec.name());
        assert!(
            decoded.round_trip && !decoded.had_errors,
            "{}",
            codec.name()
        );
    }
}

#[test]
fn strict_encoders_never_insert_numeric_references() {
    for codec in [
        Codec::Gbk,
        Codec::Big5,
        Codec::ShiftJis,
        Codec::EucJp,
        Codec::EucKr,
        Codec::Windows1252,
        Codec::Latin1,
    ] {
        assert!(codec::encode("🧪", codec).is_err(), "{}", codec.name());
    }
    assert_eq!(
        codec::encode("&#129514;", Codec::Windows1252).unwrap(),
        b"&#129514;"
    );
}

#[test]
fn latin1_is_not_windows1252() {
    assert_eq!(codec::decode(&[0x80], Codec::Latin1).text, "\u{0080}");
    assert_eq!(codec::decode(&[0x80], Codec::Windows1252).text, "€");
    assert_eq!(codec::encode("€", Codec::Windows1252).unwrap(), vec![0x80]);
    assert!(codec::encode("€", Codec::Latin1).is_err());
}

#[test]
fn gbk_decoder_has_gb18030_superset_semantics_but_encoder_does_not() {
    let bytes = codec::encode("🧪", Codec::Gb18030).unwrap();
    assert_eq!(bytes.len(), 4);
    let gbk_decoded = codec::decode(&bytes, Codec::Gbk);
    assert_eq!(gbk_decoded.text, "🧪");
    assert!(!gbk_decoded.had_errors);
    assert!(!gbk_decoded.round_trip);
    assert!(codec::encode("🧪", Codec::Gbk).is_err());
    assert!(codec::decode(&bytes, Codec::Gb18030).round_trip);
}

#[test]
fn utf32_bom_is_not_claimed_as_utf16() {
    for (bytes, name) in [
        (&[0xFF, 0xFE, 0, 0, 0x41, 0, 0, 0][..], "UTF-32LE"),
        (&[0, 0, 0xFE, 0xFF, 0, 0, 0, 0x41][..], "UTF-32BE"),
    ] {
        assert!(codec::detect_bom(bytes).is_none());
        let case = analyze_bytes(bytes, None);
        assert_eq!(case.evidence.bom.as_deref(), Some(name));
        assert!(case.warnings.contains(&"unsupported_encoding".into()));
        assert!(case.candidates.is_empty());
    }
}

#[test]
fn utf16_encoder_has_explicit_endianness_and_no_implicit_bom() {
    assert_eq!(
        codec::encode("A🧪", Codec::Utf16Le).unwrap(),
        vec![0x41, 0, 0x3E, 0xD8, 0xEA, 0xDD]
    );
    assert_eq!(codec::encode("A", Codec::Utf16Be).unwrap(), vec![0, 0x41]);
    assert!(codec::decode(&[0, 0xD8], Codec::Utf16Le).had_errors);
    assert!(codec::decode(&[0x41], Codec::Utf16Le).had_errors);
}

#[test]
fn intermediate_bom_is_not_silently_stripped() {
    let bytes = vec![0xEF, 0xBB, 0xBF, 0x41];
    assert_eq!(codec::decode(&bytes, Codec::Utf8).text, "\u{FEFF}A");
    let case = analyze_bytes(&bytes, None);
    assert_eq!(case.evidence.bom.as_deref(), Some("UTF-8"));
    assert_eq!(case.candidates[0].full_text, "A");
}

#[test]
fn clean_unicode_is_preserved() {
    for original in [
        "中文测试文件，Mojibake Lab 2026。",
        "日本語とEnglish / 한국어 / café / 🧪",
        "ASCII only\r\nkey=value\t42",
        "João, São Paulo, âme, encyclopædia: 2 questions?",
        "é e\u{0301} — normalization must stay unchanged",
    ] {
        assert_eq!(analyze_text(original).candidates[0].full_text, original);
        assert_eq!(
            analyze_bytes(original.as_bytes(), None).candidates[0].full_text,
            original
        );
    }
}

#[test]
fn windows1252_utf8_mojibake_recovers_and_explains_direction() {
    let original = "中文测试文件";
    let damaged = codec::decode(original.as_bytes(), Codec::Windows1252).text;
    let case = analyze_text(&damaged);
    let best = &case.candidates[0];
    assert_eq!(
        best.full_text,
        original,
        "candidates: {:?}",
        case.candidates
            .iter()
            .map(|c| (&c.full_text, c.score))
            .collect::<Vec<_>>()
    );
    assert!(best.reversible && !best.lossy);
    assert_eq!(best.depth, 1);
    assert_eq!(best.transformations[0].operation, "encode");
    assert_eq!(best.transformations[0].encoding, "Windows-1252");
    assert_eq!(best.transformations[1].operation, "decode");
    assert_eq!(best.transformations[1].encoding, "UTF-8");
    assert!(case.evidence.valid_utf8.is_none());
    assert!(case.evidence.hex_prefix.is_empty());
}

#[test]
fn gbk_mojibake_reconstruction_is_in_candidates() {
    let original = "中文测试";
    let damaged = codec::decode(original.as_bytes(), Codec::Gbk);
    assert!(damaged.round_trip, "fixture must not have lost bytes");
    let case = analyze_text(&damaged.text);
    assert_eq!(case.candidates[0].full_text, original);
    assert!(case
        .candidates
        .iter()
        .any(|candidate| candidate.full_text == original));
}

#[test]
fn double_mojibake_is_recovered_within_depth_bound() {
    let original = "中文测试文件";
    let first = codec::decode(original.as_bytes(), Codec::Windows1252).text;
    let second = codec::decode(first.as_bytes(), Codec::Windows1252).text;
    let case = analyze_text(&second);
    assert_eq!(
        case.candidates[0].full_text, original,
        "double mojibake should rank the recovered text first"
    );
    let recovered = case
        .candidates
        .iter()
        .find(|candidate| candidate.full_text == original)
        .unwrap_or_else(|| {
            panic!(
                "candidates: {:?}",
                case.candidates
                    .iter()
                    .map(|c| (&c.full_text, c.score))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(recovered.depth, 2);
    assert!(recovered.reversible);
}

#[test]
fn saved_corruption_fixtures_recover_from_raw_bytes() {
    let short = codec::decode("中文".as_bytes(), Codec::Windows1252).text;
    assert_eq!(
        analyze_bytes(short.as_bytes(), None).candidates[0].full_text,
        "中文"
    );
    let expected = "中文测试文件\nMojibake Lab 2026\n";
    for raw in [
        include_bytes!("../../../tests/fixtures/utf8_as_windows1252.txt").as_slice(),
        include_bytes!("../../../tests/fixtures/double_mojibake.txt").as_slice(),
    ] {
        let case = analyze_bytes(raw, None);
        assert_eq!(
            case.candidates[0].full_text, expected,
            "saved corruption should rank the recovered text first"
        );
        assert!(
            case.candidates
                .iter()
                .any(|candidate| candidate.full_text == expected),
            "candidates: {:?}",
            case.candidates
                .iter()
                .map(|candidate| (&candidate.full_text, candidate.score))
                .collect::<Vec<_>>()
        );
    }
    for (bytes, codec, expected) in [
        (
            include_bytes!("../../../tests/fixtures/big5.txt").as_slice(),
            Codec::Big5,
            "繁體中文測試\nMojibake Lab",
        ),
        (
            include_bytes!("../../../tests/fixtures/shift_jis.txt").as_slice(),
            Codec::ShiftJis,
            "日本語のテスト\nMojibake Lab",
        ),
        (
            include_bytes!("../../../tests/fixtures/gb18030.txt").as_slice(),
            Codec::Gb18030,
            "中文𠀀🧪\nMojibake Lab",
        ),
        (
            include_bytes!("../../../tests/fixtures/windows1252.txt").as_slice(),
            Codec::Windows1252,
            "café — €25\nMojibake Lab",
        ),
    ] {
        let decoded = codec::decode(bytes, codec);
        assert_eq!(decoded.text, expected);
        assert!(decoded.round_trip);
        let analysis = analyze_bytes(bytes, None);
        assert_eq!(
            analysis.candidates[0].full_text,
            expected,
            "clean {} fixture should rank first; detector {:?}",
            codec.name(),
            analysis.evidence.detector_suggestion
        );
    }
}

#[test]
fn existing_replacement_loss_is_never_laundered() {
    for case in [
        analyze_text("中�文"),
        analyze_bytes("中�文".as_bytes(), None),
    ] {
        assert!(case
            .warnings
            .iter()
            .any(|warning| warning == "information_loss"));
        assert!(case
            .candidates
            .iter()
            .all(|candidate| candidate.lossy && !candidate.reversible));
    }
    let literal = analyze_text("Is this a question?");
    assert!(!literal
        .warnings
        .iter()
        .any(|warning| warning == "information_loss"));
    assert!(!literal.candidates[0].lossy);
}

#[test]
fn invalid_sequences_are_explicit_loss() {
    let decoded = codec::decode(&[0xC0, 0xAF], Codec::Utf8);
    assert!(decoded.had_errors && !decoded.round_trip);
    assert_eq!(decoded.replacement_count, 2);
    let (_, step) = search::transform(DataState::Bytes(vec![0xFF]), Codec::Utf8).unwrap();
    assert!(step.lossy);
    assert_eq!(step.input_type, "bytes");
    assert_eq!(step.output_type, "text");
}

#[test]
fn utf16_without_bom_uses_byte_pattern() {
    for codec in [Codec::Utf16Le, Codec::Utf16Be] {
        let original = "Mojibake Lab\n中文测试 2026";
        let bytes = codec::encode(original, codec).unwrap();
        let case = analyze_bytes(&bytes, None);
        assert!(!case.evidence.binary);
        assert_eq!(case.evidence.bom, None);
        assert_eq!(case.candidates[0].full_text, original);
        assert_eq!(case.candidates[0].transformations[0].encoding, codec.name());
    }
}

#[test]
fn clean_large_utf16_prefix_avoids_speculative_search() {
    let original = "A".repeat((mojibake_core::MAX_SAMPLE_BYTES - 2) / 2);
    for codec in [Codec::Utf16Le, Codec::Utf16Be] {
        let bytes = codec::encode(&original, codec).unwrap();
        let case = analyze_bytes(&bytes, None);
        assert_eq!(case.candidates[0].full_text, original);
        assert!(case.candidates[0].reversible);
        // A proven, clean UTF-16 prefix must not fan out from deliberately
        // wrong direct decodes, which can dominate debug-build runtime.
        assert!(
            case.explored_states < 100,
            "{} explored {}",
            codec.name(),
            case.explored_states
        );
        assert!(
            case.candidates.len() > 1,
            "direct alternatives remain inspectable"
        );
    }
}

#[test]
fn strong_utf16_evidence_still_repairs_visible_double_corruption() {
    let original = "Record: 中文测试文件\n";
    let once = codec::decode(original.as_bytes(), Codec::Windows1252).text;
    let twice = codec::decode(once.as_bytes(), Codec::Windows1252).text;
    for codec in [Codec::Utf16Le, Codec::Utf16Be] {
        let bytes = codec::encode(&twice, codec).unwrap();
        let case = analyze_bytes(&bytes, None);
        assert_eq!(case.candidates[0].full_text, original);
        assert_eq!(case.candidates[0].depth, 2);
        assert!(case.candidates[0].reversible);
    }
}

#[test]
fn binary_signatures_never_enter_search() {
    for bytes in [
        b"\x89PNG\r\n\x1a\n".as_slice(),
        b"MZabcdef",
        b"%PDF-1.7",
        b"PK\x03\x04abcd",
        &[1, 0, 0, 0, 0, 2, 0, 0],
    ] {
        let case = analyze_bytes(bytes, None);
        assert!(case.evidence.binary, "{bytes:?}");
        assert!(case.candidates.is_empty());
        assert_eq!(case.explored_states, 0);
    }
}

#[test]
fn search_is_bounded_deduplicated_and_deterministic() {
    let first = analyze_sample("double").unwrap();
    let second = analyze_sample("double").unwrap();
    assert!(first.explored_states <= search::MAX_EXPLORED);
    assert!(first.candidates.len() <= search::MAX_CANDIDATES);
    let texts: HashSet<&str> = first
        .candidates
        .iter()
        .map(|candidate| candidate.full_text.as_str())
        .collect();
    assert_eq!(texts.len(), first.candidates.len());
    for (left, right) in first.candidates.iter().zip(&second.candidates) {
        assert_eq!(left.full_text, right.full_text);
        assert_eq!(left.score, right.score);
        assert_eq!(left.id, right.id);
        assert!(left.depth <= search::MAX_DEPTH);
        assert_eq!(left.transformations.len(), left.depth * 2);
        for step in &left.transformations {
            assert_ne!(step.input_type, step.output_type);
            assert_eq!(step.operation == "encode", step.input_type == "text");
        }
    }
    let ascii = analyze_bytes(b"one ASCII state", None);
    assert!(ascii.candidates[0].alternative_paths > 0);
}

#[test]
fn paste_limit_is_a_unicode_boundary_and_sampled() {
    let text = "文".repeat(mojibake_core::MAX_SAMPLE_BYTES);
    let case = analyze_text(&text);
    assert!(case.evidence.sampled);
    assert!(case.original_text.len() <= mojibake_core::MAX_SAMPLE_BYTES);
    assert!(text.starts_with(&case.original_text));
    assert!(case.warnings.contains(&"sampled_input".into()));
}

#[test]
fn empty_inputs_and_serialized_contract_are_valid() {
    let case = analyze_text("");
    assert!(case.warnings.contains(&"empty_input".into()));
    let value = serde_json::to_value(&case).unwrap();
    assert!(value.get("caseId").is_some());
    assert!(value["evidence"].get("validUtf8").unwrap().is_null());
    assert!(value["candidates"][0].get("scoreBreakdown").is_some());
    assert!(value["candidates"][0].get("fullText").is_some());
    assert!(!analyze_bytes(&[], None).evidence.binary);
    assert!(analyze_sample("invalid-id").is_err());
    for codec in CODECS {
        assert_eq!(Codec::parse(codec.name()), Some(codec));
    }
}
