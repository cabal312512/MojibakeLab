fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let license = root.join("LICENSE");
    let third_party = root.join("THIRD_PARTY_LICENSES.txt");
    println!("cargo:rerun-if-changed={}", license.display());
    println!("cargo:rerun-if-changed={}", third_party.display());
    let mut notices = std::fs::read_to_string(license).expect("Missing MIT license");
    match std::fs::read_to_string(third_party) {
        Ok(text) => {
            notices.push_str("\n\nTHIRD-PARTY LICENSES\n\n");
            notices.push_str(&text);
        }
        Err(_) if std::env::var("PROFILE").as_deref() == Ok("release") => {
            panic!("Run cargo fetch --locked and node scripts/collect-licenses.mjs before a release build");
        }
        Err(_) => notices.push_str(
            "\nDevelopment build: run node scripts/collect-licenses.mjs for dependency notices.\n",
        ),
    }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out.join("licenses.txt"), notices).unwrap();
    tauri_build::build();
}
