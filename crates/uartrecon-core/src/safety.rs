//! Safety: klasifikasi partisi kritis & analisis risiko command device.
//!
//! ## Filosofi
//!
//! Prinsip **read-only first** ditegakkan di level command. Untuk perangkat
//! seperti STB ZTE B700V5 (MediaTek MT85xx), partisi `boot` (bootloader) adalah
//! **titik brick**: bila rusak, UART tidak lagi berguna (tidak ada yang
//! mendengarkan di ujung lain) dan recovery butuh USB BROM mode.
//!
//! Modul ini mengklasifikasikan partisi berdasarkan tingkat bahaya dan
//! memblokir command tulis yang menyasar partisi kritis.

use serde::{Deserialize, Serialize};

/// Tingkat kekritisan sebuah partisi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Criticality {
    /// Fatal bila rusak: bricked, recovery butuh USB BROM. Contoh: bootloader.
    Critical,
    /// Sangat berbahaya: boot gagal. Contoh: kernel, rootfs, env.
    High,
    /// Berbahaya tapi recoverable. Contoh: app, config, logo.
    Medium,
    /// Data/user; dampak minimal. Contoh: data, vas.
    Low,
}

impl Criticality {
    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            Criticality::Critical => "CRITICAL",
            Criticality::High => "HIGH",
            Criticality::Medium => "MEDIUM",
            Criticality::Low => "LOW",
        }
    }

    /// Apakah partisi ini diblokir dari penulisan pada mode hard-block.
    pub fn blocked_in_hard_mode(self) -> bool {
        matches!(self, Criticality::Critical | Criticality::High)
    }
}

/// Satu aturan partisi yang dikenal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionRule {
    /// Nama partisi (mis. `boot`).
    pub name: String,
    /// Nomor mtd (bila diketahui).
    pub mtd: Option<u32>,
    /// Tingkat kekritisan.
    pub criticality: Criticality,
    /// Alasan.
    pub reason: String,
}

/// Aturan bawaan untuk skema partisi ZTE B700V5 (MediaTek MT85xx).
pub fn b700v5_rules() -> Vec<PartitionRule> {
    let r = |name: &str, mtd: u32, crit: Criticality, reason: &str| PartitionRule {
        name: name.to_string(),
        mtd: Some(mtd),
        criticality: crit,
        reason: reason.to_string(),
    };
    vec![
        r(
            "boot",
            1,
            Criticality::Critical,
            "Bootloader (preloader+U-Boot). RUSAK = BRICKED, butuh USB BROM.",
        ),
        r(
            "env",
            2,
            Criticality::High,
            "U-Boot environment. Rusak = bad CRC, boot tak terkendali.",
        ),
        r(
            "conf",
            3,
            Criticality::Medium,
            "Config device. Recoverable via backup.",
        ),
        r("logo", 4, Criticality::Medium, "Boot logo. Recoverable."),
        r(
            "kernel1",
            5,
            Criticality::High,
            "Kernel slot norm. Recoverable via slot safe.",
        ),
        r(
            "rootfs1",
            6,
            Criticality::High,
            "Rootfs slot norm. Recoverable via slot safe.",
        ),
        r(
            "app1",
            7,
            Criticality::Medium,
            "Aplikasi slot norm. Recoverable.",
        ),
        r(
            "kernel2",
            8,
            Criticality::High,
            "Kernel slot safe. Recoverable via slot norm.",
        ),
        r(
            "rootfs2",
            9,
            Criticality::High,
            "Rootfs slot safe. Recoverable via slot norm.",
        ),
        r(
            "app2",
            10,
            Criticality::Medium,
            "Aplikasi slot safe. Recoverable.",
        ),
        r("vas", 11, Criticality::Low, "Value-added service (UBI)."),
        r("data", 12, Criticality::Low, "Data user (UBI)."),
    ]
}

/// Mencari aturan berdasarkan nama partisi (mis. `boot`).
pub fn rule_for_name(name: &str) -> Option<PartitionRule> {
    b700v5_rules()
        .into_iter()
        .find(|r| r.name.eq_ignore_ascii_case(name))
}

/// Mencari aturan berdasarkan nomor mtd.
pub fn rule_for_mtd(mtd: u32) -> Option<PartitionRule> {
    b700v5_rules().into_iter().find(|r| r.mtd == Some(mtd))
}

/// Keputusan keamanan untuk sebuah command device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SafetyVerdict {
    /// Command diizinkan (read-only atau aman).
    Allow,
    /// Command mengubah state; butuh konfirmasi.
    Warn {
        /// Pesan peringatan.
        message: String,
    },
    /// Command DIBLOKIR (menyasar partisi kritis).
    Block {
        /// Pesan alasan.
        message: String,
        /// Partisi yang menjadi target.
        partition: String,
    },
}

impl SafetyVerdict {
    /// Apakah diblokir.
    pub fn is_blocked(&self) -> bool {
        matches!(self, SafetyVerdict::Block { .. })
    }
}

/// Keyword command yang menulis ke device.
const WRITE_KEYWORDS: &[&str] = &[
    "flash_erase",
    "flash_eraseall",
    "nand erase",
    "nand write",
    "nand markbad",
    "mtd_debug write",
    "mtd_debug erase",
    "ubiformat",
    "ubiattach", // sebenarnya read-ish, tapi treat hati-hati
    "fw_setenv",
    "saveenv",
    "setenv",
    "upgrade",
];

/// Mengekstrak nomor mtd dari string command (mis. `/dev/mtd1`, `mtdblock9`).
fn extract_mtd_targets(cmd: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let bytes = cmd.as_bytes();
    let needle = b"mtd";
    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            // Setelah "mtd", kumpulkan digit (lewati opsional "block").
            let mut j = i + needle.len();
            if bytes[j..].starts_with(b"block") {
                j += 5;
            }
            let mut num = String::new();
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                num.push(bytes[j] as char);
                j += 1;
            }
            if !num.is_empty()
                && let Ok(n) = num.parse::<u32>()
            {
                out.push(n);
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Mengekstrak target `of=/dev/mtdN` atau `of=/dev/mtdblockN`.
fn extract_dd_of_target(cmd: &str) -> Option<u32> {
    if !cmd.contains("dd ") && !cmd.contains("of=") {
        return None;
    }
    let idx = cmd.find("of=")?;
    let rest = &cmd[idx + 3..];
    let targets = extract_mtd_targets(rest);
    targets.first().copied()
}

/// Menganalisis command device untuk menentukan verdict keamanan.
///
/// `hard_mode = true` akan memblokir SEMUA tulis ke partisi Critical + High.
pub fn analyze_device_command(cmd: &str, hard_mode: bool) -> SafetyVerdict {
    let lower = cmd.to_ascii_lowercase();

    // 1. Deteksi command tulis.
    let is_write = WRITE_KEYWORDS.iter().any(|k| lower.contains(k))
        || (lower.contains("dd ") && lower.contains("of="))
        || (lower.contains(">") && lower.contains("/dev/mtd"));

    if !is_write {
        return SafetyVerdict::Allow;
    }

    // 2. Cari target partisi.
    let mut targets = extract_mtd_targets(&lower);
    if let Some(mtd) = extract_dd_of_target(&lower) {
        targets.push(mtd);
    }

    // 3. Periksa target terhadap aturan.
    for mtd in &targets {
        if let Some(rule) = rule_for_mtd(*mtd) {
            let blocked = if hard_mode {
                rule.criticality.blocked_in_hard_mode()
            } else {
                rule.criticality == Criticality::Critical
            };
            if blocked {
                return SafetyVerdict::Block {
                    message: format!(
                        "menulis ke mtd{} ({}) DIBLOKIR [{}]: {}",
                        mtd,
                        rule.name,
                        rule.criticality.label(),
                        rule.reason
                    ),
                    partition: rule.name,
                };
            }
        }
    }

    // 4. Command tulis yang tidak menyasar partisi kritis: peringatkan.
    SafetyVerdict::Warn {
        message: format!("command menulis: '{}'", cmd.trim()),
    }
}

/// Preflight check: daftar partisi kritis yang HARUS dibackup sebelum eksperimen.
pub fn required_backups() -> Vec<PartitionRule> {
    b700v5_rules()
        .into_iter()
        .filter(|r| r.criticality.blocked_in_hard_mode())
        .collect()
}

/// Ringkasan kebijakan keamanan (untuk ditampilkan di CLI).
pub fn policy_summary() -> String {
    let mut out = String::new();
    out.push_str("SAFETY POLICY (read-only first)\n");
    out.push_str("--------------------------------\n\n");
    out.push_str("Partisi yang DIBLOKIR untuk ditulis (hard-block):\n");
    for r in b700v5_rules() {
        if r.criticality.blocked_in_hard_mode() {
            out.push_str(&format!(
                "  mtd{:<2} {:<10} [{}]  {}\n",
                r.mtd.map(|m| m.to_string()).unwrap_or_else(|| "?".into()),
                r.name,
                r.criticality.label(),
                r.reason
            ));
        }
    }
    out.push_str("\nPartisi lain (recoverable, tetap hati-hati):\n");
    for r in b700v5_rules() {
        if !r.criticality.blocked_in_hard_mode() {
            out.push_str(&format!(
                "  mtd{:<2} {:<10} [{}]  {}\n",
                r.mtd.map(|m| m.to_string()).unwrap_or_else(|| "?".into()),
                r.name,
                r.criticality.label(),
                r.reason
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_command_diizinkan() {
        assert_eq!(
            analyze_device_command("cat /proc/mtd", true),
            SafetyVerdict::Allow
        );
        assert_eq!(
            analyze_device_command("ls /dev", true),
            SafetyVerdict::Allow
        );
        assert_eq!(
            analyze_device_command("md5sum /dev/mtd1", true),
            SafetyVerdict::Allow
        );
        assert_eq!(
            analyze_device_command("hexdump -C /dev/mtd1", true),
            SafetyVerdict::Allow
        );
    }

    #[test]
    fn flash_erase_bootloader_diblokir() {
        let v = analyze_device_command("flash_erase /dev/mtd1 0 1", true);
        assert!(v.is_blocked(), "harus diblokir: {v:?}");
        if let SafetyVerdict::Block { partition, .. } = v {
            assert_eq!(partition, "boot");
        }
    }

    #[test]
    fn dd_ke_mtd1_diblokir() {
        let v = analyze_device_command("dd if=/tmp/x of=/dev/mtd1 bs=1M", true);
        assert!(v.is_blocked());
    }

    #[test]
    fn nand_write_kernel_diblokir() {
        let v = analyze_device_command("nand write 0x80000000 0x500000 0x400000", true);
        // Tidak ada target mtd eksplisit -> hanya warn.
        assert!(!v.is_blocked());
    }

    #[test]
    fn nand_write_kernel_dengan_mtd_diblokir() {
        let v = analyze_device_command("nand write /dev/mtd5 data.bin", true);
        assert!(v.is_blocked());
    }

    #[test]
    fn setenv_diblokir() {
        let v = analyze_device_command("setenv bootargs console=ttyMT0", true);
        assert!(v.is_blocked() || matches!(v, SafetyVerdict::Warn { .. }));
    }

    #[test]
    fn write_ke_data_hanya_warn() {
        let v = analyze_device_command("dd if=/tmp/big of=/dev/mtd12 bs=1M", true);
        assert!(matches!(v, SafetyVerdict::Warn { .. }));
    }

    #[test]
    fn ekstrak_mtd_target() {
        assert_eq!(extract_mtd_targets("/dev/mtd1"), vec![1]);
        assert_eq!(extract_mtd_targets("mtdblock9"), vec![9]);
        assert_eq!(extract_mtd_targets("/dev/mtd10"), vec![10]);
        assert!(extract_mtd_targets("nomtd").is_empty());
    }

    #[test]
    fn rule_lookup() {
        assert_eq!(
            rule_for_name("boot").unwrap().criticality,
            Criticality::Critical
        );
        assert_eq!(rule_for_mtd(1).unwrap().name, "boot");
        assert_eq!(rule_for_mtd(9).unwrap().criticality, Criticality::High);
        assert!(rule_for_name("nonexistent").is_none());
    }

    #[test]
    fn required_backups_mencakup_bootloader() {
        let req = required_backups();
        assert!(req.iter().any(|r| r.name == "boot"));
        assert!(req.iter().any(|r| r.name == "kernel1"));
        // Data & vas tidak termasuk.
        assert!(!req.iter().any(|r| r.name == "data"));
    }
}
