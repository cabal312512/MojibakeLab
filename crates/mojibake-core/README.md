# Encoding core

This crate performs no file I/O, networking, normalization or rewriting. Its input is either raw bytes or an already decoded Unicode string. All inference is bounded and deterministic; elapsed time is the only nondeterministic field in a serialized analysis.

`analyze_bytes`, `analyze_text` and `analyze_sample` return the camelCase JSON contract consumed by the desktop UI. The 32 KiB input limit is enforced in the core as a second line of defense. Text prefixes end on a Unicode scalar boundary. A caller that samples a raw file must avoid cutting a multibyte sequence and must label the result as sampled. The desktop shell handles head/middle/tail checks and full-file export replay.

## Codec semantics

| Encoding | Behavior |
| --- | --- |
| UTF-8 | Strict encoding; decoding records malformed input. |
| UTF-16 LE / BE | Explicit endianness, custom encoders, no implicit BOM. |
| GBK | WHATWG / encoding_rs decoder accepts the GB18030 superset; encoder emits the GBK repertoire. A four-byte-only character therefore fails the GBK round trip and is correctly attributed to GB18030. |
| GB18030 | encoding_rs WHATWG mapping, including current GB18030 mapping updates. |
| Big5, Shift_JIS, EUC-JP, EUC-KR | encoding_rs WHATWG semantics. Legacy aliases and duplicate mappings can make a conversion non-bijective. |
| Windows-1252 | encoding_rs semantics, distinct from literal Latin-1. |
| ISO-8859-1 | Literal byte U+0000–U+00FF mapping, implemented explicitly. |

Encoders use `encode_from_utf8_to_vec_without_replacement`; unmappable input fails rather than becoming HTML numeric references. Every proposed pair checks exact round trips. Lossless decoding alone is not enough: some legacy sequences decode successfully but do not re-encode to identical bytes. UTF-32 BOMs are reported as unsupported instead of being mistaken for UTF-16LE.

chardetng reports the shared GBK/GB18030 decoder family as GBK. That family clue also supports a GB18030 candidate; its exact round trip distinguishes four-byte characters that the GBK encoder cannot preserve.

BOM detection and removal happen once at the original file boundary. Intermediate U+FEFF characters remain untouched. A `replacementCount` counts U+FFFD in a decoder's output, including pre-existing U+FFFD; `had_errors` separately records newly malformed sequences. Existing replacement loss propagates to all candidates and cannot be hidden by reinterpreting its UTF-8 spelling.

## Search and scoring

The engine first interprets file bytes using supported codecs, or retains the pasted Unicode text unchanged. Each repair step consists of an explicit text → encode → bytes → decode → text pair. Paths have at most three repair pairs; each frontier retains at most twelve states, with hard limits of 1,024 unique states and 4,096 examined decodes. The attempt budget decreases for larger inputs. Already clean Unicode receives one repair layer instead of inventing deeper histories, including UTF-16 supported by a BOM or a strong null-byte pattern. Direct decode alternatives remain inspectable; visible corruption still receives the full search depth. Candidate text equality is exact, without Unicode normalization. A hash accelerates lookup, but an exact equality check resolves collisions.

Binary signatures and strong unexplained control/NUL evidence stop recovery. Lossy or non-round-trippable states stay visible where relevant but are not expanded. Identity operations and duplicate states are not expanded. UTF-16 repair guesses require NUL evidence to avoid manufacturing arbitrary plausible-looking ideographs.

Weights are centralized in `scoring.rs`. Character scripts have deliberately small influence; the engine has no language model. The Unicode-validity bonus comes from actual UTF-8 byte structure or explicit UTF-16 evidence, rather than the fact that every decoded Rust string is Unicode. Unhinted UTF-16 interpretations receive a penalty because arbitrary even byte streams easily resemble printable Han characters. Mojibake markers are adjacent patterns rather than penalties for an isolated legitimate accented letter. Clean text receives a preservation preference. Strict legacy → UTF-8 contractions retain a bounded lookahead preference in the search frontier so that an intermediate stage of double mojibake can survive pruning; this preference does not inflate displayed candidate scores.

Scores are heuristic points, not percentages or proof of a unique history. BOM evidence can produce a score above 100. A reversible path proves byte/text preservation, not semantic correctness. Some short samples remain ambiguous and unsupported historical code-page variants may not be recoverable.

## Validation and fixtures

```sh
cargo test -p mojibake-core
cargo run -p mojibake-core --example generate_fixtures
cargo run -p mojibake-core --example sample_json -- cp1252
```

The generator writes known byte fixtures under the repository's `tests/fixtures/` directory. The JSON example accepts `cp1252`, `gbk`, `double`, `utf16`, `loss` or `clean` and is also used for offline UI demonstration data.

Reference semantics: [encoding_rs API](https://docs.rs/encoding_rs/latest/encoding_rs/), [GBK decoder/encoder distinction](https://docs.rs/encoding_rs/latest/encoding_rs/static.GBK.html), [strict encoder API](https://docs.rs/encoding_rs/latest/encoding_rs/struct.Encoder.html), [chardetng detector](https://docs.rs/chardetng/0.1.17/chardetng/struct.EncodingDetector.html).
