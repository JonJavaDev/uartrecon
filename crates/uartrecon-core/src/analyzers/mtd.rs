//! Parser MTD & partisi storage.
//!
//! Mem-parsing output `cat /proc/mtd` dan `cat /proc/partitions` menjadi
//! struktur tabel partisi yang mudah diproses.

use serde::{Deserialize, Serialize};

/// Satu partisi storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Partition {
    /// Nama device (mis. `mtd0`, `mmcblk0p1`).
    pub device: String,
    /// Nama partisi (mis. `bootloader`, `rootfs`).
    pub name: String,
    /// Ukuran dalam byte (bila diketahui).
    pub size_bytes: Option<u64>,
}

/// Tabel partisi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PartitionTable {
    /// Jenis storage (NAND/MMC/SD/NOR), bila terdeteksi.
    pub storage_kind: Option<String>,
    /// Daftar partisi.
    pub partitions: Vec<Partition>,
}

impl PartitionTable {
    /// Apakah kosong.
    pub fn is_empty(&self) -> bool {
        self.partitions.is_empty()
    }

    /// Render sebagai teks tabel.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        if let Some(kind) = &self.storage_kind {
            out.push_str(&format!("STORAGE TYPE: {kind}\n\n"));
        }
        out.push_str(&format!("{:<10} {:<16} {}\n", "Device", "Name", "Size"));
        for p in &self.partitions {
            let size = p
                .size_bytes
                .map(crate::util::human_size)
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!("{:<10} {:<16} {}\n", p.device, p.name, size));
        }
        out
    }
}

/// Mem-parsing output `cat /proc/mtd`.
///
/// Contoh input:
/// ```text
/// dev:    size   erasesize  name
/// mtd0: 00040000 00020000 "bootloader"
/// mtd1: 00300000 00020000 "kernel"
/// ```
pub fn parse_proc_mtd(data: &[u8]) -> PartitionTable {
    let text = String::from_utf8_lossy(data);
    let re = regex::Regex::new(r#"(mtd\d+):\s+([0-9a-fA-F]+)\s+[0-9a-fA-F]+\s+"([^"]*)""#)
        .expect("regex MTD valid");
    let mut partitions = Vec::new();
    for caps in re.captures_iter(&text) {
        let device = caps[1].to_string();
        let size_hex = &caps[2];
        let name = caps[3].to_string();
        let size_bytes = u64::from_str_radix(size_hex, 16).ok();
        partitions.push(Partition {
            device,
            name,
            size_bytes,
        });
    }
    PartitionTable {
        storage_kind: if partitions.is_empty() {
            None
        } else {
            Some("NAND".to_string())
        },
        partitions,
    }
}

/// Mem-parsing output `cat /proc/partitions`.
///
/// Contoh input:
/// ```text
/// major minor  #blocks  name
///   179        0    7634944 mmcblk0
///   179        1      65536 mmcblk0p1
/// ```
pub fn parse_proc_partitions(data: &[u8]) -> PartitionTable {
    let text = String::from_utf8_lossy(data);
    let re = regex::Regex::new(r"(?m)^\s*\d+\s+\d+\s+(\d+)\s+(\S+)\s*$").expect("regex valid");
    let mut partitions = Vec::new();
    for caps in re.captures_iter(&text) {
        let blocks: u64 = caps[1].parse().unwrap_or(0);
        let device = caps[2].to_string();
        // Blok Linux umumnya 1024 byte.
        let size_bytes = Some(blocks * 1024);
        let name = device.clone();
        partitions.push(Partition {
            device,
            name,
            size_bytes,
        });
    }
    let storage_kind = if partitions.iter().any(|p| p.device.starts_with("mmcblk")) {
        Some("MMC".to_string())
    } else if partitions.iter().any(|p| p.device.starts_with("sd")) {
        Some("SD".to_string())
    } else if partitions.is_empty() {
        None
    } else {
        Some("block".to_string())
    };
    PartitionTable {
        storage_kind,
        partitions,
    }
}

/// Mem-parsing gabungan (coba MTD dulu, lalu partitions).
pub fn parse_any(data: &[u8]) -> PartitionTable {
    let mtd = parse_proc_mtd(data);
    if !mtd.is_empty() {
        return mtd;
    }
    parse_proc_partitions(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROC_MTD: &[u8] = b"dev:    size   erasesize  name\nmtd0: 00040000 00020000 \"bootloader\"\nmtd1: 00300000 00020000 \"kernel\"\nmtd2: 02000000 00020000 \"rootfs\"\nmtd3: 01000000 00020000 \"userdata\"\n";

    const PROC_PARTITIONS: &[u8] = b"major minor  #blocks  name\n 179        0    7634944 mmcblk0\n 179        1      65536 mmcblk0p1\n 179        2    2000000 mmcblk0p2\n";

    #[test]
    fn parse_mtd_benar() {
        let t = parse_proc_mtd(PROC_MTD);
        assert_eq!(t.partitions.len(), 4);
        assert_eq!(t.partitions[0].device, "mtd0");
        assert_eq!(t.partitions[0].name, "bootloader");
        assert_eq!(t.partitions[0].size_bytes, Some(0x0004_0000));
        assert_eq!(t.storage_kind.as_deref(), Some("NAND"));
    }

    #[test]
    fn parse_partitions_benar() {
        let t = parse_proc_partitions(PROC_PARTITIONS);
        assert_eq!(t.partitions.len(), 3);
        assert_eq!(t.partitions[0].device, "mmcblk0");
        assert_eq!(t.partitions[0].size_bytes, Some(7634944 * 1024));
        assert_eq!(t.storage_kind.as_deref(), Some("MMC"));
    }

    #[test]
    fn parse_any_prioritas_mtd() {
        let combined = [PROC_MTD, PROC_PARTITIONS].concat();
        let t = parse_any(&combined);
        assert_eq!(t.storage_kind.as_deref(), Some("NAND"));
    }

    #[test]
    fn data_kosong() {
        let t = parse_any(b"nothing here");
        assert!(t.is_empty());
        assert_eq!(t.storage_kind, None);
    }

    #[test]
    fn render_tabel() {
        let t = parse_proc_mtd(PROC_MTD);
        let text = t.to_text();
        assert!(text.contains("NAND"));
        assert!(text.contains("bootloader"));
    }
}
