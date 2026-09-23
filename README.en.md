# Mojibake Lab

**Encoding went wrong. Find out where.**

[简体中文](README.md) · English

A small, fully offline desktop tool for investigating broken text encodings. Open a file or paste text, compare recovery candidates, and inspect the byte/text conversions behind each result. Compact light interface, starting in English, with 简体中文 and 日本語 available. Your language choice is remembered.

![Mojibake Lab desktop application](docs/screenshots/workbench.png)

## Run on Windows

Double-click **启动 Mojibake Lab.cmd** in the project root, or run `release/MojibakeLab/MojibakeLab.exe`. The portable build needs no installation. Windows 10/11 must already have Microsoft Edge WebView2 Runtime; the application does not install or download it.

1. Open or drop a plain-text file, paste text into the editor, or choose a bundled sample.
2. Select a candidate. The recovery path stays visible below the text; tabs expose the full path, comparison, evidence, score, and hex view.
3. Check the result, then copy it or save a new UTF-8 file.

`Ctrl/Cmd+O` opens a file. `Ctrl/Cmd+Enter` analyzes text in the input editor.

**Export never overwrites an existing file.** The backend refuses both the original file and any other existing destination, even if selected in the save dialog.

## How recovery works

The UTF-8 bytes for `中文` are `E4 B8 AD E6 96 87`. Decoding those bytes as Windows-1252 produces `ä¸­æ–‡`. Encoding that text back to Windows-1252 bytes, then decoding as UTF-8, recovers the original:

```text
ä¸­æ–‡  ── encode Windows-1252 ──▶  E4 B8 AD E6 96 87
                                      │ decode UTF-8
                                      ▼
                                     中文
```

The tool searches bounded conversion paths, scores the resulting text, and merges duplicate candidates. A recovery path is a testable reconstruction, not proof of what actually happened to the file.

## Support and limits

- **Encodings:** UTF-8 with or without BOM, UTF-16 LE/BE, GBK, GB18030, Big5, Shift_JIS, EUC-JP, EUC-KR, Windows-1252, and strict Latin-1 byte semantics. UTF-16 encoding and Latin-1 have explicit implementations; other mappings follow `encoding_rs` / WHATWG, rather than claiming equivalence to every historical code page or GB18030 revision.
- **Uncertainty:** detector output is a clue. Scores are heuristic points, not recovery probabilities. A reversible conversion does not prove the intended meaning. Script distribution is only weak evidence; clean UTF-8 is preferentially preserved.
- **Information loss:** `�` may indicate that original bytes were already discarded. Lost characters cannot be reconstructed with certainty. An ordinary question mark is not automatically classified as damage.
- **Bounded search:** at most three repair rounds, retaining twelve states per round, with limits on total states, transformations, and output size. Double mojibake is included in the bundled samples.
- **Large files:** files over 32 KiB are analyzed using a character-aligned head sample. Middle and tail samples check UTF-8 validity and common binary markers. Export streams and validates the complete file; newly encountered conversion loss aborts export and removes partial output. Sampling cannot rule out mixed encodings, which are not reliably repaired automatically.
- **Input limits:** pasted text is limited to 32 KiB; open larger input as a file. The hex inspector reads at most 4 KiB per page. Sampled cases offer bounded text for copying; use export for the full file.
- **Scope:** one plain-text file at a time. Common binary signatures and abnormal control bytes are rejected. No PDF, Word, or spreadsheet extraction.

## Offline and portable

No accounts, AI services, telemetry, update checks, or remote fonts. Rust handles file processing locally. Production content security policy restricts network access, and navigation is limited to local application pages.

On Windows, settings, WebView data, and temporary files live in `runtime-data` beside the executable, or under the project root when using the root launcher. The app does not change the system PATH, registry, or default applications. Operating-system and system-WebView logs and file-dialog history are outside its complete control.

The Windows development scripts keep tools, dependencies, downloads, and caches inside `.tools`, `.cache`, `.tmp`, `node_modules`, and `target`. These directories are excluded from source control.

## Development

Tauri 2 · Rust · React 19 · TypeScript · Vite

```text
crates/mojibake-core/   Codecs, evidence, scoring, search, algorithm tests
src-tauri/             Raw file IO, streaming export, desktop commands
src/                   Localized interface, paths, comparison, hex view
tests/fixtures/        Reproducible encoding samples
scripts/               Portable tools, builds, verification, licenses
docs/                  Architecture and desktop screenshots
```

From PowerShell in the project directory:

```powershell
# Initial setup downloads tools into this project only.
powershell -ExecutionPolicy Bypass -File scripts/toolchain.ps1
. ./scripts/env.ps1
npm.cmd ci
npm.cmd run desktop:dev

# Verify and build the portable application.
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

For a standard macOS/Linux development environment, install the official Tauri system prerequisites, then run `npm ci` and `npm run desktop:dev`. The portable Windows toolchain scripts are Windows-specific.

```sh
npm run typecheck
npm test
npm run build
cargo test --workspace --locked
cargo check --workspace --locked
cargo fmt --all --check
npm run desktop:build
```

The repository includes a Windows, Linux, and macOS CI matrix; see GitHub Actions for current build results. **Interactive desktop behavior has been verified locally on Windows only.** Successful CI builds do not replace interactive testing on the other target systems.

## Publishing

Commit source and lockfiles; keep local tools, caches, application data, and build outputs out of Git. Do not redistribute the Microsoft SDK/compiler from `.tools`. For a Windows portable release, archive `release/MojibakeLab`, retain its license files, and exclude `runtime-data`. The current application is unsigned.

[Architecture](docs/ARCHITECTURE.md) · [Third-party notices](THIRD_PARTY_NOTICES.md) · [MIT license](LICENSE)
