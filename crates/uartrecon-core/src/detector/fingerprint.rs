//! Fingerprinting perangkat: bootloader, OS, shell, vendor/SoC.
//!
//! Setiap hasil memiliki kategori kepercayaan:
//! - [`Confidence::Detected`] — bukti kuat (pola khas + konteks).
//! - [`Confidence::Probable`] — ada indikasi tapi tidak konklusif.
//! - [`Confidence::Unknown`] — tidak ada bukti.
//!
//! **Penting:** kita tidak mengklaim SoC hanya berdasarkan satu string.
//! Setiap pola memiliki bobot, dan total bobot menentukan kategori.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Tingkat kepercayaan hasil deteksi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// Bukti kuat.
    Detected,
    /// Kemungkinan besar.
    Probable,
    /// Tidak diketahui.
    Unknown,
}

impl Confidence {
    /// Membangun dari skor 0..100 memakai ambang tetap.
    pub fn from_score(score: u32) -> Self {
        if score >= 60 {
            Confidence::Detected
        } else if score >= 25 {
            Confidence::Probable
        } else {
            Confidence::Unknown
        }
    }

    /// Label manusiawi.
    pub fn label(self) -> &'static str {
        match self {
            Confidence::Detected => "Detected",
            Confidence::Probable => "Probable",
            Confidence::Unknown => "Unknown",
        }
    }
}

/// Satu pola deteksi dengan bobot.
struct WeightedPattern {
    /// Nama/ID pola.
    name: &'static str,
    /// Regex yang dicari (case-insensitive bila `ci`).
    pattern: &'static str,
    /// Bobot kontribusi (0..100).
    weight: u32,
    /// Apakah case-insensitive.
    case_insensitive: bool,
}

/// Hasil deteksi satu kategori (mis. bootloader).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    /// Nama yang terdeteksi (mis. `U-Boot`).
    pub name: String,
    /// Skor 0..100.
    pub score: u32,
    /// Kategori kepercayaan.
    pub confidence: Confidence,
    /// Nama-nama pola yang cocok.
    pub matched: Vec<String>,
}

impl Detection {
    fn unknown() -> Self {
        Detection {
            name: "Unknown".to_string(),
            score: 0,
            confidence: Confidence::Unknown,
            matched: Vec::new(),
        }
    }
}

/// Hasil fingerprinting lengkap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fingerprint {
    /// Bootloader (U-Boot, CFE, Barebox, ...).
    pub bootloader: Detection,
    /// OS (Linux, OpenWrt, Android, ...).
    pub os: Detection,
    /// Shell (BusyBox, bash, ...).
    pub shell: Detection,
    /// Vendor/SoC (hanya bila bukti kuat).
    pub vendor: Detection,
    /// Ukuran RAM yang terdeteksi (byte), bila ada.
    pub ram_bytes: Option<u64>,
    /// Ukuran storage yang terdeteksi (byte), bila ada.
    pub storage_bytes: Option<u64>,
    /// Indikator jenis storage (NAND/MMC/SD/NOR).
    pub storage_kind: Option<String>,
}

impl Fingerprint {
    /// Ringkasan teks multi-baris untuk ditampilkan.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "BOOTLOADER\n  {} ({}, {}/100)\n",
            self.bootloader.name,
            self.bootloader.confidence.label(),
            self.bootloader.score
        ));
        s.push_str(&format!(
            "OS\n  {} ({}, {}/100)\n",
            self.os.name,
            self.os.confidence.label(),
            self.os.score
        ));
        s.push_str(&format!(
            "SHELL\n  {} ({}, {}/100)\n",
            self.shell.name,
            self.shell.confidence.label(),
            self.shell.score
        ));
        if self.vendor.confidence != Confidence::Unknown {
            s.push_str(&format!(
                "VENDOR\n  {} ({}, {}/100)\n",
                self.vendor.name,
                self.vendor.confidence.label(),
                self.vendor.score
            ));
        }
        if let Some(ram) = self.ram_bytes {
            s.push_str(&format!("RAM\n  {}\n", crate::util::human_size(ram)));
        }
        if let Some(storage) = self.storage_bytes {
            let kind = self.storage_kind.as_deref().unwrap_or("storage");
            s.push_str(&format!(
                "STORAGE\n  {} {}\n",
                kind,
                crate::util::human_size(storage)
            ));
        }
        s
    }
}

fn match_patterns(text: &str, patterns: &[WeightedPattern]) -> (u32, Vec<String>) {
    let mut score = 0u32;
    let mut matched = Vec::new();
    for p in patterns {
        let re = if p.case_insensitive {
            Regex::new(&format!("(?i){}", p.pattern))
        } else {
            Regex::new(p.pattern)
        };
        if let Ok(re) = re
            && re.is_match(text)
        {
            score = score.saturating_add(p.weight);
            matched.push(p.name.to_string());
        }
    }
    (score.min(100), matched)
}

fn detection_from(name: &str, text: &str, patterns: &[WeightedPattern]) -> Detection {
    let (score, matched) = match_patterns(text, patterns);
    if matched.is_empty() {
        return Detection::unknown();
    }
    Detection {
        name: name.to_string(),
        score,
        confidence: Confidence::from_score(score),
        matched,
    }
}

/// Pola bootloader.
const BOOTLOADER_PATTERNS: &[WeightedPattern] = &[
    WeightedPattern {
        name: "U-Boot banner",
        pattern: r"U-Boot",
        weight: 55,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "U-Boot prompt",
        pattern: r"Hit any key",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "U-Boot env",
        pattern: r"\b(bootcmd|bootargs)\b",
        weight: 25,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Das U-Boot",
        pattern: r"Das U-Boot",
        weight: 40,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "CFE version banner",
        pattern: r"CFE version",
        weight: 60,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "CFE",
        pattern: r"\bCFE\b",
        weight: 45,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Barebox",
        pattern: r"barebox",
        weight: 45,
        case_insensitive: true,
    },
];

/// Pola OS.
const OS_PATTERNS: &[WeightedPattern] = &[
    WeightedPattern {
        name: "Linux version",
        pattern: r"Linux version",
        weight: 45,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "kernel message",
        pattern: r"(?i)\bstarting kernel\b|\[\s*\d+\.\d+\]\s+\w+",
        weight: 25,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "OpenWrt",
        pattern: r"OpenWrt",
        weight: 40,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Android",
        pattern: r"Android",
        weight: 35,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "init/login",
        pattern: r"(?m)^\s*(login:|root@|init:)",
        weight: 20,
        case_insensitive: false,
    },
];

/// Pola shell.
const SHELL_PATTERNS: &[WeightedPattern] = &[
    WeightedPattern {
        name: "BusyBox",
        pattern: r"BusyBox",
        weight: 55,
        case_insensitive: true,
    },
    WeightedPattern {
        name: "root shell",
        pattern: r"root@",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "sh prompt",
        pattern: r"(?m)[#\$]\s*$",
        weight: 15,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "login prompt",
        pattern: r"login:",
        weight: 25,
        case_insensitive: false,
    },
];

/// Pola vendor/SoC. Bobotnya kecil karena satu string saja tidak cukup.
const VENDOR_PATTERNS: &[WeightedPattern] = &[
    WeightedPattern {
        name: "Broadcom",
        pattern: r"(?i)broadcom|bcm\d",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "MediaTek",
        pattern: r"(?i)mediatek|mt\d{4}",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Realtek",
        pattern: r"(?i)realtek|rtl\d{4}",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Amlogic",
        pattern: r"(?i)amlogic|s905|s912",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Rockchip",
        pattern: r"(?i)rockchip|rk\d{4}",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "Allwinner",
        pattern: r"(?i)allwinner|sun\d(i|x)",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "HiSilicon",
        pattern: r"(?i)hisilicon|hi3\d{3}",
        weight: 30,
        case_insensitive: false,
    },
    WeightedPattern {
        name: "MStar",
        pattern: r"(?i)mstar|mst\d",
        weight: 30,
        case_insensitive: false,
    },
];

/// Menangkap ukuran RAM dari pola umum (mis. `DRAM: 512 MiB`).
fn detect_ram(text: &str) -> Option<u64> {
    let re = Regex::new(r"(?i)(?:DRAM|Memory|RAM)[:\s]+(\d+)\s*(MiB|MB|GiB|GB|KiB|KB|B)").ok()?;
    let caps = re.captures(text)?;
    let value: u64 = caps.get(1)?.as_str().parse().ok()?;
    let unit = caps.get(2)?.as_str().to_ascii_lowercase();
    let multiplier: u64 = match unit.as_str() {
        "gib" | "gb" => 1024 * 1024 * 1024,
        "mib" | "mb" => 1024 * 1024,
        "kib" | "kb" => 1024,
        _ => 1,
    };
    Some(value * multiplier)
}

/// Menangkap ukuran & jenis storage dari pola umum (mis. `NAND: 256 MiB`).
fn detect_storage(text: &str) -> (Option<String>, Option<u64>) {
    let re = Regex::new(r"(?i)(NAND|MMC|eMMC|NOR|SD)[:\s]+(\d+)\s*(MiB|MB|GiB|GB|KiB|KB|B)").ok();
    if let Some(re) = re
        && let Some(caps) = re.captures(text)
    {
        let kind = caps.get(1).map(|m| m.as_str().to_string());
        let value: Option<u64> = caps.get(2).and_then(|m| m.as_str().parse().ok());
        let unit = caps
            .get(3)
            .map(|m| m.as_str().to_ascii_lowercase())
            .unwrap_or_default();
        let multiplier: u64 = match unit.as_str() {
            "gib" | "gb" => 1024 * 1024 * 1024,
            "mib" | "mb" => 1024 * 1024,
            "kib" | "kb" => 1024,
            _ => 1,
        };
        return (kind, value.map(|v| v * multiplier));
    }
    (None, None)
}

/// Melakukan fingerprinting terhadap data (biasanya isi bootlog).
pub fn fingerprint(data: &[u8]) -> Fingerprint {
    let text = String::from_utf8_lossy(data);
    let (storage_kind, storage_bytes) = detect_storage(&text);
    Fingerprint {
        bootloader: detection_from("U-Boot", &text, BOOTLOADER_PATTERNS),
        os: detection_from("Embedded Linux", &text, OS_PATTERNS),
        shell: detection_from("BusyBox", &text, SHELL_PATTERNS),
        vendor: detection_from("Vendor", &text, VENDOR_PATTERNS),
        ram_bytes: detect_ram(&text),
        storage_bytes,
        storage_kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UBOOT_LOG: &[u8] = b"U-Boot 2021.10 (Jan 01 2021)\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nHit any key to stop autoboot\r\n";

    #[test]
    fn deteksi_uboot_kuat() {
        let fp = fingerprint(UBOOT_LOG);
        assert_eq!(fp.bootloader.confidence, Confidence::Detected);
        assert!(fp.bootloader.score >= 60);
    }

    #[test]
    fn deteksi_linux_busybox() {
        let log = b"Linux version 5.10.0 (gcc)\r\nBusyBox v1.35.0\r\nlogin: root\r\n";
        let fp = fingerprint(log);
        assert_eq!(fp.os.confidence, Confidence::Detected);
        assert_eq!(fp.shell.name, "BusyBox");
        assert!(fp.shell.score >= 55);
    }

    #[test]
    fn ram_dan_storage_terdeteksi() {
        let fp = fingerprint(UBOOT_LOG);
        assert_eq!(fp.ram_bytes, Some(512 * 1024 * 1024));
        assert_eq!(fp.storage_bytes, Some(256 * 1024 * 1024));
        assert_eq!(fp.storage_kind.as_deref(), Some("NAND"));
    }

    #[test]
    fn vendor_tidak_diklaim_dari_satu_string() {
        // Hanya ada "Broadcom" tanpa konteks -> hanya probable.
        let fp = fingerprint(b"Broadcom\r\n");
        assert_ne!(fp.vendor.confidence, Confidence::Detected);
    }

    #[test]
    fn data_kosong_unknown() {
        let fp = fingerprint(b"");
        assert_eq!(fp.bootloader.confidence, Confidence::Unknown);
        assert_eq!(fp.os.confidence, Confidence::Unknown);
        assert_eq!(fp.ram_bytes, None);
    }

    #[test]
    fn cfe_terdeteksi() {
        let fp = fingerprint(b"CFE version 1.0.37\r\n");
        assert_eq!(fp.bootloader.confidence, Confidence::Detected);
    }
}
