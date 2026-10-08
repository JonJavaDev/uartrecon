//! Device profiles: definisi pola untuk jenis perangkat tertentu.
//!
//! Profile dapat dibangun ke dalam binary atau dimuat dari file TOML di
//! direktori `profiles/`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Definisi sebuah profile perangkat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Nama profile (mis. `stb`).
    pub name: String,
    /// Deskripsi.
    pub description: String,
    /// Pola bootloader yang diharapkan.
    #[serde(default)]
    pub bootloader_patterns: Vec<String>,
    /// Pola OS yang diharapkan.
    #[serde(default)]
    pub os_patterns: Vec<String>,
    /// Pola storage.
    #[serde(default)]
    pub storage_patterns: Vec<String>,
    /// Baudrate yang disarankan.
    #[serde(default)]
    pub suggested_baudrates: Vec<u32>,
}

impl Profile {
    /// Profile generic (default).
    pub fn generic() -> Self {
        Profile {
            name: "generic".to_string(),
            description: "Profil generik untuk perangkat embedded tak dikenal".to_string(),
            bootloader_patterns: vec!["U-Boot".into(), "CFE".into(), "Barebox".into()],
            os_patterns: vec!["Linux".into(), "BusyBox".into()],
            storage_patterns: vec!["NAND".into(), "MMC".into(), "NOR".into()],
            suggested_baudrates: vec![115_200, 57_600, 38_400, 9_600],
        }
    }

    /// Profile embedded Linux umum.
    pub fn embedded_linux() -> Self {
        Profile {
            name: "embedded_linux".to_string(),
            description: "Perangkat Embedded Linux umum".to_string(),
            bootloader_patterns: vec!["U-Boot".into(), "Barebox".into()],
            os_patterns: vec!["Linux version".into(), "BusyBox".into()],
            storage_patterns: vec!["NAND".into(), "MMC".into()],
            suggested_baudrates: vec![115_200, 57_600],
        }
    }

    /// Profile STB (set-top box).
    pub fn stb() -> Self {
        Profile {
            name: "stb".to_string(),
            description: "Set-top box / receiver".to_string(),
            bootloader_patterns: vec!["U-Boot".into(), "CFE".into()],
            os_patterns: vec!["Linux".into(), "BusyBox".into()],
            storage_patterns: vec!["NAND".into(), "MMC".into(), "SquashFS".into()],
            suggested_baudrates: vec![115_200, 57_600, 38_400],
        }
    }

    /// Semua profile built-in.
    pub fn builtin() -> Vec<Profile> {
        vec![
            Profile::generic(),
            Profile::embedded_linux(),
            Profile::stb(),
        ]
    }

    /// Mencari profile built-in berdasarkan nama.
    pub fn find_builtin(name: &str) -> Option<Profile> {
        Profile::builtin()
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

/// Memuat profile dari file TOML.
pub fn load_from_toml(path: impl AsRef<Path>) -> Result<Profile> {
    let content = std::fs::read_to_string(path)?;
    let profile: Profile = toml::from_str(&content)?;
    Ok(profile)
}

/// Memuat semua profile `.toml` dari sebuah direktori.
pub fn load_dir(dir: impl AsRef<Path>) -> Result<Vec<Profile>> {
    let mut out = Vec::new();
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            match load_from_toml(&path) {
                Ok(p) => out.push(p),
                Err(e) => {
                    tracing::warn!(file = %path.display(), error = %e, "gagal memuat profile")
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_tersedia() {
        assert_eq!(Profile::builtin().len(), 3);
        assert!(Profile::find_builtin("stb").is_some());
        assert!(Profile::find_builtin("STB").is_some());
        assert!(Profile::find_builtin("nope").is_none());
    }

    #[test]
    fn profile_roundtrip_toml() {
        let p = Profile::stb();
        let toml_str = toml::to_string(&p).unwrap();
        let back: Profile = toml::from_str(&toml_str).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn load_toml_file() {
        let dir =
            std::env::temp_dir().join(format!("uartrecon_profile_{}", crate::util::now_unix()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.toml");
        let p = Profile::generic();
        std::fs::write(&path, toml::to_string(&p).unwrap()).unwrap();
        let loaded = load_from_toml(&path).unwrap();
        assert_eq!(loaded.name, "generic");

        let all = load_dir(&dir).unwrap();
        assert_eq!(all.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
