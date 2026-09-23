use std::path::{Path, PathBuf};

pub fn resolve() -> Result<PathBuf, &'static str> {
    if let Some(path) = std::env::var_os("MOJIBAKE_DATA_DIR").filter(|p| !p.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let exe = std::env::current_exe().map_err(|_| "Cannot locate executable")?;
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    location(std::env::consts::OS, &exe, home.as_deref(), xdg.as_deref())
}

fn location(
    os: &str,
    exe: &Path,
    home: Option<&Path>,
    xdg: Option<&Path>,
) -> Result<PathBuf, &'static str> {
    match os {
        "windows" => exe
            .parent()
            .map(|p| p.join("runtime-data"))
            .ok_or("Missing executable folder"),
        "macos" => home
            .map(|p| p.join("Library/Application Support/MojibakeLab"))
            .ok_or("Missing home folder"),
        _ => xdg
            .filter(|p| p.is_absolute())
            .map(|p| p.join("mojibake-lab"))
            .or_else(|| home.map(|p| p.join(".local/share/mojibake-lab")))
            .ok_or("Missing user data folder"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_and_mounted_bundles_use_writable_user_data() {
        let home = std::env::temp_dir().join("mojibake-home");
        let exe = home.join("readonly-bundle/bin/MojibakeLab");
        let xdg = home.join("xdg");
        assert_eq!(
            location("windows", &exe, Some(&home), Some(&xdg)).unwrap(),
            exe.parent().unwrap().join("runtime-data")
        );
        assert_eq!(
            location("macos", &exe, Some(&home), None).unwrap(),
            home.join("Library/Application Support/MojibakeLab")
        );
        assert_eq!(
            location("linux", &exe, Some(&home), Some(&xdg)).unwrap(),
            xdg.join("mojibake-lab")
        );
        assert_eq!(
            location("linux", &exe, Some(&home), Some(Path::new("relative"))).unwrap(),
            home.join(".local/share/mojibake-lab")
        );
        assert!(location("macos", &exe, None, None).is_err());
    }
}
