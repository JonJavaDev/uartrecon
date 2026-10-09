//! Macro: rangkaian command yang bisa dijalankan berurutan.
//!
//! Berguna untuk otomasi: kirim command ke device, tunggu, kirim lagi, dst.
//! Macro disimpan sebagai TOML di direktori config.
//!
//! ```text
//! ~/.config/uartrecon/macros/<nama>.toml
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Satu langkah dalam macro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// Command yang dikirim.
    pub send: String,
    /// Tunggu N milidetik setelah kirim.
    #[serde(default = "default_wait")]
    pub wait_ms: u64,
    /// Tunggu sampai pola ini muncul (opsional). Kalau kosong, pakai `wait_ms`.
    #[serde(default)]
    pub expect: Option<String>,
    /// Timeout untuk `expect` (milidetik).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_wait() -> u64 {
    500
}

fn default_timeout() -> u64 {
    5000
}

/// Sebuah macro (rangkaian command).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Macro {
    /// Nama macro.
    pub name: String,
    /// Deskripsi.
    #[serde(default)]
    pub description: String,
    /// Daftar langkah.
    pub steps: Vec<Step>,
}

impl Macro {
    /// Membuat macro baru dari daftar command sederhana.
    pub fn from_commands(name: impl Into<String>, commands: &[&str]) -> Self {
        Macro {
            name: name.into(),
            description: String::new(),
            steps: commands
                .iter()
                .map(|c| Step {
                    send: c.to_string(),
                    wait_ms: default_wait(),
                    expect: None,
                    timeout_ms: default_timeout(),
                })
                .collect(),
        }
    }

    /// Simpan ke file TOML.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Muat dari file TOML.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

impl Step {
    /// Durasi tunggu efektif (pakai `wait_ms` bila tidak ada `expect`).
    pub fn wait_duration(&self) -> Duration {
        Duration::from_millis(self.wait_ms)
    }

    /// Timeout sebagai Duration.
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

/// Direktori macro default.
pub fn macros_dir() -> PathBuf {
    crate::config::config_dir().join("macros")
}

/// Path file macro berdasarkan nama.
pub fn macro_path(name: &str) -> PathBuf {
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
    macros_dir().join(format!("{safe}.toml"))
}

/// Simpan macro.
pub fn save(m: &Macro) -> Result<PathBuf> {
    let path = macro_path(&m.name);
    m.save(&path)?;
    Ok(path)
}

/// Muat macro.
pub fn load(name: &str) -> Result<Macro> {
    let path = macro_path(name);
    if !path.exists() {
        return Err(Error::Other(format!("macro '{name}' tidak ditemukan")));
    }
    Macro::load(&path)
}

/// Daftar semua macro.
pub fn list() -> Result<Vec<Macro>> {
    let dir = macros_dir();
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml")
            && let Ok(m) = Macro::load(&path)
        {
            out.push(m);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Hapus macro.
pub fn remove(name: &str) -> Result<bool> {
    let path = macro_path(name);
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
    fn macro_dari_commands() {
        let m = Macro::from_commands("info", &["uname -a", "cat /proc/mtd"]);
        assert_eq!(m.steps.len(), 2);
        assert_eq!(m.steps[0].send, "uname -a");
        assert_eq!(m.steps[0].wait_ms, 500);
    }

    #[test]
    fn macro_roundtrip() {
        let m = Macro::from_commands("test", &["ls", "pwd"]);
        let text = toml::to_string_pretty(&m).unwrap();
        let back: Macro = toml::from_str(&text).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn step_durasi() {
        let s = Step {
            send: "x".into(),
            wait_ms: 1000,
            expect: None,
            timeout_ms: 3000,
        };
        assert_eq!(s.wait_duration(), Duration::from_millis(1000));
        assert_eq!(s.timeout_duration(), Duration::from_millis(3000));
    }
}
