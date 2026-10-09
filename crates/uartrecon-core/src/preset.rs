//! Preset device: simpan/load konfigurasi koneksi untuk perangkat tertentu.
//!
//! Berguna kalau sering ngulik device yang sama (mis. STB ZTE B700V5 selalu
//! 115200 8N1). Preset disimpan sebagai TOML di direktori config.
//!
//! ```text
//! ~/.config/uartrecon/presets/<nama>.toml
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Preset koneksi untuk sebuah device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    /// Nama preset.
    pub name: String,
    /// Deskripsi (opsional).
    #[serde(default)]
    pub description: String,
    /// Nama port (opsional - bisa beda tiap koneksi).
    #[serde(default)]
    pub port: Option<String>,
    /// Baudrate.
    pub baudrate: u32,
    /// Format (mis. `8N1`).
    #[serde(default = "default_format")]
    pub format: String,
    /// Command U-Boot favorit (opsional).
    #[serde(default)]
    pub uboot_commands: Vec<String>,
    /// Command shell favorit (opsional).
    #[serde(default)]
    pub shell_commands: Vec<String>,
}

fn default_format() -> String {
    "8N1".to_string()
}

impl Preset {
    /// Membuat preset baru.
    pub fn new(name: impl Into<String>, baudrate: u32, format: impl Into<String>) -> Self {
        Preset {
            name: name.into(),
            description: String::new(),
            port: None,
            baudrate,
            format: format.into(),
            uboot_commands: Vec::new(),
            shell_commands: Vec::new(),
        }
    }

    /// Simpan preset ke file TOML.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Muat preset dari file TOML.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

/// Direktori preset default.
pub fn presets_dir() -> PathBuf {
    crate::config::config_dir().join("presets")
}

/// Path file preset berdasarkan nama.
pub fn preset_path(name: &str) -> PathBuf {
    // Sanitasi nama agar tidak keluar direktori.
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    presets_dir().join(format!("{safe}.toml"))
}

/// Simpan preset berdasarkan nama.
pub fn save(preset: &Preset) -> Result<PathBuf> {
    let path = preset_path(&preset.name);
    preset.save(&path)?;
    Ok(path)
}

/// Muat preset berdasarkan nama.
pub fn load(name: &str) -> Result<Preset> {
    let path = preset_path(name);
    if !path.exists() {
        return Err(Error::Other(format!("preset '{name}' tidak ditemukan")));
    }
    Preset::load(&path)
}

/// Daftar semua preset yang tersimpan.
pub fn list() -> Result<Vec<Preset>> {
    let dir = presets_dir();
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml")
            && let Ok(p) = Preset::load(&path)
        {
            out.push(p);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Hapus preset berdasarkan nama.
pub fn remove(name: &str) -> Result<bool> {
    let path = preset_path(name);
    if path.exists() {
        std::fs::remove_file(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_roundtrip() {
        let mut p = Preset::new("stb-b700", 115_200, "8N1");
        p.description = "ZTE B700V5".to_string();
        p.uboot_commands = vec!["setenv system norm".to_string()];
        let text = toml::to_string_pretty(&p).unwrap();
        let back: Preset = toml::from_str(&text).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn preset_path_sanitasi() {
        let p = preset_path("../../etc/passwd");
        assert!(p.to_string_lossy().contains("presets"));
        assert!(!p.to_string_lossy().contains(".."));
    }

    #[test]
    fn default_format_ok() {
        let p = Preset::new("x", 9600, "8N1");
        assert_eq!(p.format, "8N1");
        assert!(p.uboot_commands.is_empty());
    }
}
