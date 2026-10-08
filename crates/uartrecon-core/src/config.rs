//! Konfigurasi persisten UARTRecon.
//!
//! Disimpan sebagai TOML di direktori config pengguna
//! (`~/.config/uartrecon/config.toml` di Linux/macOS,
//! `%APPDATA%\uartrecon\config.toml` di Windows).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Konfigurasi aplikasi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Baudrate default.
    pub default_baudrate: u32,
    /// Format default (mis. `8N1`).
    pub default_format: String,
    /// Daftar baudrate untuk scanning.
    pub scan_baudrates: Vec<u32>,
    /// Durasi baca per kandidat (milidetik).
    pub scan_duration_ms: u64,
    /// Direktori sesi.
    pub sessions_dir: String,
    /// Direktori capture.
    pub captures_dir: String,
    /// Direktori profiles.
    pub profiles_dir: String,
    /// Panjang minimum string untuk ekstraksi.
    pub min_string_len: usize,
    /// Ukuran blok entropi.
    pub entropy_block_size: usize,
    /// Apakah menampilkan warna.
    pub color: bool,
    /// Jumlah hasil maksimum untuk pencarian.
    pub max_search_results: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_baudrate: 115_200,
            default_format: "8N1".to_string(),
            scan_baudrates: vec![
                9600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600,
            ],
            scan_duration_ms: 700,
            sessions_dir: "sessions".to_string(),
            captures_dir: "captures".to_string(),
            profiles_dir: "profiles".to_string(),
            min_string_len: 4,
            entropy_block_size: 1024,
            color: true,
            max_search_results: 1000,
        }
    }
}

impl Config {
    /// Memuat dari path tertentu, atau default bila file tidak ada.
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "config tidak valid, memakai default");
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    /// Menyimpan ke path tertentu (membuat parent dir bila perlu).
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Direktori config pengguna (cross-platform).
pub fn config_dir() -> PathBuf {
    // Linux/macOS: $XDG_CONFIG_HOME/uartrecon atau ~/.config/uartrecon
    // Windows: %APPDATA%\uartrecon
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("uartrecon");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("uartrecon");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("uartrecon");
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(profile).join(".config").join("uartrecon");
    }
    PathBuf::from(".uartrecon")
}

/// Path file config default.
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Memuat config dari lokasi default.
pub fn load() -> Config {
    Config::load_or_default(config_path())
}

/// Menyimpan config ke lokasi default.
pub fn save(config: &Config) -> Result<PathBuf> {
    let path = config_path();
    config.save(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sensible() {
        let c = Config::default();
        assert_eq!(c.default_baudrate, 115_200);
        assert_eq!(c.default_format, "8N1");
        assert!(!c.scan_baudrates.is_empty());
    }

    #[test]
    fn roundtrip_toml() {
        let c = Config::default();
        let text = toml::to_string_pretty(&c).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn load_or_default_file_hilang() {
        let c = Config::load_or_default("tidak_ada_file_xyz.toml");
        assert_eq!(c, Config::default());
    }

    #[test]
    fn save_dan_load() {
        let dir = std::env::temp_dir().join(format!("uartrecon_cfg_{}", crate::util::now_unix()));
        let path = dir.join("config.toml");
        let c = Config {
            default_baudrate: 57_600,
            ..Config::default()
        };
        c.save(&path).unwrap();
        assert!(path.exists());
        let loaded = Config::load_or_default(&path);
        assert_eq!(loaded.default_baudrate, 57_600);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_dir_tidak_kosong() {
        let dir = config_dir();
        assert!(!dir.as_os_str().is_empty());
    }
}
