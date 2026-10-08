//! Recovery: generate script shell untuk backup & restore partisi via SD card.
//!
//! ## Mengapa via SD card?
//!
//! Transfer data besar via UART sangat lambat (~0.2 KB/s) dan rawan corrupt.
//! STB embedded umumnya punya port USB yang bisa baca SD card/flashdisk
//! (auto-mount ke `/var/mntt/usba1` pada ZTE B700V5). Jadi:
//!
//! ```text
//! UART  -> kirim perintah (kecil)
//! SD    -> transfer data partisi (cepat & andal)
//! ```
//!
//! Modul ini menghasilkan script yang **read-only terhadap flash** (hanya baca
//! partisi, tulis ke SD), lalu script restore terpisah yang WAJIB dikonfirmasi.

use serde::{Deserialize, Serialize};

use crate::safety::{self, Criticality};

/// Lokasi mount SD card umum pada STB embedded.
pub const SD_MOUNT_CANDIDATES: &[&str] = &[
    "/var/mntt/usba1",
    "/var/mnt/usba1",
    "/mnt/usb",
    "/mnt/sd",
    "/media/usb0",
    "/tmp/usb",
];

/// Membuat script backup partisi kritis ke SD card.
///
/// Script ini **read-only terhadap flash** - hanya `dd if=/dev/mtdN` (baca) dan
/// tulis ke SD. Aman dijalankan.
pub fn backup_script(parts: &[safety::PartitionRule], out_subdir: &str) -> String {
    let mut s = String::new();
    s.push_str("#!/bin/sh\n");
    s.push_str("# UARTRecon backup script (READ-ONLY terhadap flash)\n");
    s.push_str("# Partisi hanya DIBACA (dd if=/dev/mtdN), hasil ditulis ke SD card.\n");
    s.push_str("set -e\n\n");

    s.push_str("# --- Temukan mount point SD card ---\n");
    s.push_str("SD=\"\"\n");
    for m in SD_MOUNT_CANDIDATES {
        s.push_str(&format!(
            "if [ -z \"$SD\" ] && [ -d \"{m}\" ] && mount | grep -q \"{m}\"; then SD=\"{m}\"; fi\n"
        ));
    }
    // Fallback: cari /var/mntt/usb* apa saja.
    s.push_str("if [ -z \"$SD\" ]; then SD=$(ls -d /var/mntt/usb* 2>/dev/null | head -1); fi\n");
    s.push_str("if [ -z \"$SD\" ]; then echo \"ERROR: SD card tidak terdeteksi\"; exit 1; fi\n");
    s.push_str(&format!(
        "OUT=\"$SD/{out_subdir}\"\nmkdir -p \"$OUT\"\necho \"Backup ke: $OUT\"\n\n"
    ));

    s.push_str("# --- Backup tiap partisi ---\n");
    for p in parts {
        let Some(mtd) = p.mtd else { continue };
        s.push_str(&format!("echo \"[*] {} (mtd{mtd})...\"\n", p.name));
        s.push_str(&format!(
            "dd if=/dev/mtd{mtd} of=\"$OUT/mtd{mtd}_{}.bin\" bs=64k 2>/dev/null\n",
            p.name
        ));
        s.push_str(&format!(
            "md5sum \"$OUT/mtd{mtd}_{}.bin\" | awk '{{print $1}}' > \"$OUT/mtd{mtd}_{}.md5\"\n",
            p.name, p.name
        ));
        s.push_str(&format!(
            "echo \"    -> mtd{mtd}_{}.bin ($(cat \"$OUT/mtd{mtd}_{}.md5\"))\"\n\n",
            p.name, p.name
        ));
    }

    s.push_str("# --- Manifest ---\n");
    s.push_str("echo \"--- MANIFEST ---\" > \"$OUT/MANIFEST.txt\"\n");
    s.push_str("date >> \"$OUT/MANIFEST.txt\" 2>/dev/null || true\n");
    s.push_str(&format!(
        "echo \"device={}\" >> \"$OUT/MANIFEST.txt\"\n",
        "{DEVICE}"
    ));
    s.push_str("cat /proc/mtd >> \"$OUT/MANIFEST.txt\"\n");
    s.push_str("cat \"$OUT\"/mtd*.md5 >> \"$OUT/MANIFEST.txt\" 2>/dev/null || true\n");
    s.push_str("sync\n");
    s.push_str("echo \"[+] Backup selesai. Cabut SD & colok ke PC.\"\n");
    s
}

/// Membuat script restore dari SD card ke partisi.
///
/// **BERBAHAYA** - script ini menulis ke flash. Wajib dikonfirmasi user.
pub fn restore_script(parts: &[safety::PartitionRule], in_subdir: &str) -> String {
    let mut s = String::new();
    s.push_str("#!/bin/sh\n");
    s.push_str("# !!! UARTRecon RESTORE script (MENULIS KE FLASH) !!!\n");
    s.push_str("# Hanya jalankan bila Anda benar-benar yakin. Butuh konfirmasi.\n");
    s.push_str("set -e\n\n");

    s.push_str("echo \"==================================================\"\n");
    s.push_str("echo \"  PERINGATAN: script ini akan MENULIS ke flash!\"\n");
    s.push_str("echo \"  Menulis ke partisi boot/kernel salah = BRICKED.\"\n");
    s.push_str("echo \"==================================================\"\n");
    s.push_str("read -p \"Ketik 'RESTORE' untuk lanjut: \" ans\n");
    s.push_str("[ \"$ans\" = \"RESTORE\" ] || { echo \"Dibatalkan.\"; exit 1; }\n\n");

    s.push_str("SD=\"\"\n");
    for m in SD_MOUNT_CANDIDATES {
        s.push_str(&format!(
            "if [ -z \"$SD\" ] && [ -d \"{m}\" ] && mount | grep -q \"{m}\"; then SD=\"{m}\"; fi\n"
        ));
    }
    s.push_str("if [ -z \"$SD\" ]; then SD=$(ls -d /var/mntt/usb* 2>/dev/null | head -1); fi\n");
    s.push_str(&format!(
        "IN=\"$SD/{in_subdir}\"\n[ -d \"$IN\" ] || {{ echo \"ERROR: $IN tidak ada\"; exit 1; }}\n\n"
    ));

    for p in parts {
        let Some(mtd) = p.mtd else { continue };
        // Bootloader (Critical) tidak pernah masuk restore otomatis.
        if p.criticality == Criticality::Critical {
            s.push_str(&format!(
                "# SKIP mtd{mtd} ({}): CRITICAL - bootloader TIDAK boleh ditulis otomatis.\n\n",
                p.name
            ));
            continue;
        }
        s.push_str(&format!("echo \"[*] restore {} (mtd{mtd})...\"\n", p.name));
        s.push_str(&format!(
            "if [ -f \"$IN/mtd{mtd}_{}.bin\" ]; then\n",
            p.name
        ));
        s.push_str(&format!(
            "  flash_erase /dev/mtd{mtd} 0 0\n  dd if=\"$IN/mtd{mtd}_{}.bin\" of=/dev/mtd{mtd} bs=64k 2>/dev/null\n",
            p.name
        ));
        s.push_str(&format!("  echo \"    -> mtd{mtd} {} OK\"\nelse\n  echo \"    [!] file mtd{mtd}_{}.bin tidak ada, skip\"\nfi\n\n", p.name, p.name));
    }

    s.push_str("sync\necho \"[+] Restore selesai. Reboot untuk menerapkan.\"\n");
    s
}

/// Rencana recovery (untuk ditampilkan & disimpan).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// Nama device.
    pub device: String,
    /// Subdirektori backup di SD.
    pub subdir: String,
    /// Daftar partisi yang dibackup (semua kritis).
    pub parts: Vec<safety::PartitionRule>,
    /// Script backup (read-only).
    pub backup_script: String,
    /// Script restore (berbahaya).
    pub restore_script: String,
}

impl RecoveryPlan {
    /// Membuat rencana recovery untuk device.
    pub fn new(device: impl Into<String>, subdir: impl Into<String>) -> Self {
        let device = device.into();
        let subdir = subdir.into();
        // Backup semua partisi kritis + conf (recoverable penting).
        let parts: Vec<_> = safety::b700v5_rules()
            .into_iter()
            .filter(|r| {
                matches!(
                    r.criticality,
                    Criticality::Critical | Criticality::High | Criticality::Medium
                )
            })
            .collect();

        let backup_script = backup_script(&parts, &subdir).replace("{DEVICE}", &device);
        let restore_script = restore_script(&parts, &subdir);

        RecoveryPlan {
            device,
            subdir,
            parts,
            backup_script,
            restore_script,
        }
    }

    /// Ringkasan teks.
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("DEVICE: {}\n", self.device));
        s.push_str(&format!("SD subdir: {}\n\n", self.subdir));
        s.push_str("Partisi yang akan dibackup:\n");
        for p in &self.parts {
            s.push_str(&format!(
                "  mtd{:<2} {:<10} [{}]\n",
                p.mtd.map(|m| m.to_string()).unwrap_or_else(|| "?".into()),
                p.name,
                p.criticality.label()
            ));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_script_readonly() {
        let parts = safety::required_backups();
        let script = backup_script(&parts, "uartrecon_backup");
        // Harus ada dd if=/dev/mtd (baca).
        assert!(script.contains("dd if=/dev/mtd1"));
        // TIDAK boleh ada flash_erase / nand write (tulis).
        assert!(!script.contains("flash_erase"));
        assert!(!script.contains("nand write"));
        // Harus ada verifikasi md5.
        assert!(script.contains("md5sum"));
        // Harus cari mount SD.
        assert!(script.contains("/var/mntt/usba1"));
    }

    #[test]
    fn restore_script_menulis_dan_konfirmasi() {
        let parts = safety::required_backups();
        let script = restore_script(&parts, "uartrecon_backup");
        assert!(script.contains("flash_erase"));
        assert!(script.contains("RESTORE"));
        // Bootloader (mtd1) harus di-SKIP.
        assert!(script.contains("# SKIP mtd1"));
        assert!(!script.contains("flash_erase /dev/mtd1 "));
    }

    #[test]
    fn restore_tidak_pernah_menulis_bootloader() {
        let parts = safety::b700v5_rules();
        let script = restore_script(&parts, "x");
        // Pastikan tidak ada perintah tulis ke mtd1.
        assert!(!script.contains("of=/dev/mtd1 "));
        assert!(!script.contains("flash_erase /dev/mtd1 "));
    }

    #[test]
    fn recovery_plan_lengkap() {
        let plan = RecoveryPlan::new("B700V5S1", "uartrecon_backup");
        assert!(plan.parts.iter().any(|p| p.name == "boot"));
        assert!(plan.backup_script.contains("mtd1_boot.bin"));
        assert!(plan.summary().contains("B700V5S1"));
    }
}
