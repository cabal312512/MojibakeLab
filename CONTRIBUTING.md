# Contributing

Keep recovery deterministic, bounded and explainable. No network service or model belongs in the runtime application. Byte evidence is immutable, and Unicode text must not be normalized or rewritten as part of a repair.

The Rust core lives in `crates/mojibake-core`. New codec paths need exact round-trip tests, including unmappable characters and malformed sequences. For streaming changes, test every split of a multibyte character and cleanup after late conversion failure. The shell uses `create_new` when saving; preserve its refusal to overwrite existing paths.

UI changes should fit an 820 × 640 window and remain usable at 680 × 520. Keep the interface light, concise, keyboard accessible and fully translated into Simplified Chinese, English and Japanese. Detailed evidence belongs in the inspector rather than the main reading area.

Before a pull request:

```sh
npm run typecheck
npm test
npm run build
cargo fmt --all --check
cargo test --workspace --locked
cargo check --workspace --locked
```

Windows developers can dot-source `scripts/env.ps1` to keep tools, dependencies and temporary data in the checkout. `scripts/test-desktop.ps1` runs the real release application through its WebView2 debugging endpoint; this endpoint is enabled by the test launcher only. Never commit local evidence, runtime profiles, caches or toolchains.

Fixtures are raw evidence. `.gitattributes` deliberately disables text conversion under `tests/fixtures`. Add a reproducible source and expected original when contributing a damaged sample. Use fabricated text, not personal documents.
