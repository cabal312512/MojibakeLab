# Mojibake Lab — implementation design

The desktop shell is Tauri 2; a standalone Rust crate owns all encoding logic and file access. React 19 only renders bounded evidence, candidates and transformation paths. No service, AI, account, telemetry or remote assets are used.

## Encoding contract

`Bytes(Vec<u8>)` and `Text(String)` are separate states. Every operation is either Text → encode → Bytes or Bytes → decode → Text. UTF-16 encoding and literal ISO-8859-1 semantics need explicit wrappers: encoding_rs does not provide UTF-16 encoders, and its Latin-1 labels resolve to Windows-1252. Use explicit no-BOM-handling decoders; detect and strip a BOM only at the evidence boundary, recording it. Unmappable encodes are rejected from reversible search, never accepted as encoding_rs numeric character references. Decodes report replacements and byte-for-byte round trips.

## Bounded recovery

Start with raw decodes for a file or the unchanged Unicode text for a paste. Expand up to three repair pairs (encode a suspected wrong decoder, decode an original encoding), retaining a beam of twelve states. Cap input sampling, candidates and previews. Deduplicate exact text states during search, group equivalent final candidates with alternative paths, preserve original Unicode without normalization. Record every actual byte/text transition and preview. Never assert an inferred historical error as an observed fact.

## Ranking and uncertainty

Central weights combine printable text, control and replacement penalties, weak script coherence, characteristic mojibake sequences, round-trip preservation and a depth cost. Valid clean UTF-8 and unchanged clean paste receive a preservation preference. Detector output is a suggestion. Scores are heuristic points, never percentages. A reversible path is not proof of semantic correctness; existing replacement characters remain a loss warning. Question marks alone are not proof of loss.

## Files and export

Rust reads bytes; React never loads entire source files. Sample evidence is explicitly distinguished from full-file validation. Large files use head/middle/tail samples and bounded previews; export replays a selected chain using incremental codecs and refuses newly discovered loss. Create output files with create_new; never overwrite any existing file or original. Partial output is removed on conversion failure. Copy is limited to available preview when a case is sampled and must say so.

## Graph

Each candidate carries typed transformation steps with input/output previews, encoding, operation, sizes, replacement count and loss flags. A source node plus alternating byte/text nodes reconstructs the observed recovery path. Candidate selection changes the active path; alternate candidate branches remain visible in subdued form.

## Implementation sequence

1. Scaffold workspace, local toolchain and desktop raw-byte commands.
2. Codec wrappers and round-trip unit tests.
3. Evidence metrics, detector and binary protection.
4. Typed transformation engine and bounded search.
5. Fixtures and ranking/loss/limits regression tests.
6. Compact light-only text tool, localized controls, graph and candidate inspector.
7. Compare, hex, copy and safe streaming export.
8. Integration tests, native build, screenshots, release docs and CI.

The five principal risks are ambiguous rankings (conservative baselines and visible alternatives), silent codec loss (strict wrappers and round trips), search explosion (beam/depth/input limits), chunk boundaries (stateful streaming decoders), and packaging side effects (project-local toolchains, caches and portable runtime data). Validate each subsystem before integration.

References: https://docs.rs/encoding_rs/latest/encoding_rs/ and https://v2.tauri.app/start/prerequisites/ .
