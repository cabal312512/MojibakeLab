# Third-party software

Mojibake Lab is MIT licensed. It uses the following principal open-source dependencies. The exact resolved versions and full transitive dependency list are recorded in `Cargo.lock` and `package-lock.json`.

| Component | License | Source |
| --- | --- | --- |
| Tauri, Wry, Tao | MIT / Apache-2.0 | https://github.com/tauri-apps |
| encoding_rs | MIT / Apache-2.0 / BSD-3-Clause | https://github.com/hsivonen/encoding_rs |
| chardetng | MIT / Apache-2.0 | https://github.com/hsivonen/chardetng |
| serde / serde_json | MIT / Apache-2.0 | https://github.com/serde-rs |
| unicode-script | MIT / Apache-2.0 | https://github.com/unicode-rs/unicode-script |
| sha2 | MIT / Apache-2.0 | https://github.com/RustCrypto/hashes |
| React | MIT | https://github.com/facebook/react |
| Zustand | MIT | https://github.com/pmndrs/zustand |
| Lucide icons | ISC | https://github.com/lucide-icons/lucide |
| Vite / TypeScript / Vitest | MIT / Apache-2.0 / MIT | https://github.com/vitejs/vite |

Microsoft Edge WebView2 is a system runtime with Microsoft's own license. Windows portable builds use the installed runtime and do not download or install it. The MSVC/Windows SDK files under `.tools` are local build prerequisites under Microsoft's terms and must **not** be published in the source repository or redistributed as part of a source archive. Runtime redistributable DLLs are distributed only where Microsoft's redistributable license permits it.

Run `node scripts/collect-licenses.mjs` after a dependency change to regenerate `THIRD_PARTY_LICENSES.txt`. This generated file is included in portable distributions and excluded from the source repository. `scripts/build.ps1` generates and includes it automatically.
