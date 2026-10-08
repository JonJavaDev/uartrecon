//! Firmware signature scanner (binwalk-like).
//!
//! Memindai data untuk mencari "magic bytes" yang menandai awal dari
//! struktur yang dikenal: filesystem (SquashFS, JFFS2, cramfs, UBIFS),
//! arsip (gzip, xz, lzma), kernel (uImage, FIT), bootloader (U-Boot),
//! dan format lain yang umum di perangkat embedded.
//!
//! Semua operasi bersifat **read-only**: hanya membaca dan melaporkan offset.

use serde::{Deserialize, Serialize};

/// Satu signature yang dikenal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signature {
    /// Nama format (mis. `SquashFS`).
    pub name: String,
    /// Kategori (filesystem/archive/kernel/bootloader/firmware/other).
    pub category: String,
    /// Offset byte tempat magic ditemukan.
    pub offset: usize,
    /// Deskripsi singkat.
    pub description: String,
    /// Apakah hasil ini sering menjadi false positive (magic pendek).
    pub weak: bool,
}

/// Definisi magic: nama, kategori, magic bytes, deskripsi.
struct MagicDef {
    name: &'static str,
    category: &'static str,
    magic: &'static [u8],
    description: &'static str,
}

/// Daftar magic yang dicari.
const MAGICS: &[MagicDef] = &[
    MagicDef {
        name: "SquashFS (LE)",
        category: "filesystem",
        magic: b"hsqs",
        description: "SquashFS little-endian",
    },
    MagicDef {
        name: "SquashFS (BE)",
        category: "filesystem",
        magic: b"sqsh",
        description: "SquashFS big-endian",
    },
    MagicDef {
        name: "JFFS2",
        category: "filesystem",
        magic: &[0x85, 0x19],
        description: "JFFS2 filesystem",
    },
    MagicDef {
        name: "cramfs",
        category: "filesystem",
        magic: &[0x45, 0x3D, 0xCD, 0x28],
        description: "cramfs filesystem",
    },
    MagicDef {
        name: "UBIFS",
        category: "filesystem",
        magic: b"UBI#",
        description: "UBIFS filesystem",
    },
    MagicDef {
        name: "ext2/3/4",
        category: "filesystem",
        magic: &[0x53, 0xEF],
        description: "ext2/3/4 superblock",
    },
    MagicDef {
        name: "gzip",
        category: "archive",
        magic: &[0x1F, 0x8B],
        description: "gzip compressed data",
    },
    MagicDef {
        name: "xz",
        category: "archive",
        magic: &[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00],
        description: "xz compressed data",
    },
    MagicDef {
        name: "LZMA",
        category: "archive",
        magic: &[0x5D, 0x00, 0x00],
        description: "LZMA stream (5-byte header)",
    },
    MagicDef {
        name: "bzip2",
        category: "archive",
        magic: b"BZh",
        description: "bzip2 compressed data",
    },
    MagicDef {
        name: "Zstandard",
        category: "archive",
        magic: &[0x28, 0xB5, 0x2F, 0xFD],
        description: "Zstandard compressed data",
    },
    MagicDef {
        name: "LZ4",
        category: "archive",
        magic: &[0x04, 0x22, 0x4D, 0x18],
        description: "LZ4 frame",
    },
    MagicDef {
        name: "7-Zip",
        category: "archive",
        magic: &[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C],
        description: "7-Zip archive",
    },
    MagicDef {
        name: "tar (ustar)",
        category: "archive",
        magic: b"ustar",
        description: "POSIX tar archive",
    },
    MagicDef {
        name: "cpio (newc)",
        category: "archive",
        magic: b"070701",
        description: "cpio newc archive",
    },
    MagicDef {
        name: "cpio (odc)",
        category: "archive",
        magic: b"070707",
        description: "cpio odc archive",
    },
    MagicDef {
        name: "U-Boot uImage",
        category: "kernel",
        magic: &[0x27, 0x05, 0x19, 0x56],
        description: "U-Boot legacy uImage",
    },
    MagicDef {
        name: "FIT/DTB",
        category: "kernel",
        magic: &[0xD0, 0x0D, 0xFE, 0xED],
        description: "Flattened Device Tree / FIT image",
    },
    MagicDef {
        name: "Linux kernel (bzImage)",
        category: "kernel",
        magic: b"HdrS",
        description: "Linux x86 bzImage setup header",
    },
    MagicDef {
        name: "Android boot img",
        category: "kernel",
        magic: b"ANDROID!",
        description: "Android boot image",
    },
    MagicDef {
        name: "U-Boot",
        category: "bootloader",
        magic: b"U-Boot ",
        description: "U-Boot bootloader banner",
    },
    MagicDef {
        name: "ELF",
        category: "executable",
        magic: &[0x7F, 0x45, 0x4C, 0x46],
        description: "ELF executable",
    },
    MagicDef {
        name: "PE/EXE",
        category: "executable",
        magic: b"MZ",
        description: "DOS/PE executable (weak, 2 byte)",
    },
    MagicDef {
        name: "ZIP",
        category: "archive",
        magic: b"PK\x03\x04",
        description: "ZIP archive",
    },
    MagicDef {
        name: "PNG",
        category: "image",
        magic: &[0x89, 0x50, 0x4E, 0x47],
        description: "PNG image",
    },
    MagicDef {
        name: "JPEG",
        category: "image",
        magic: &[0xFF, 0xD8, 0xFF],
        description: "JPEG image",
    },
    MagicDef {
        name: "SQLite",
        category: "database",
        magic: b"SQLite format 3",
        description: "SQLite database",
    },
];

/// Magic pendek (<= 3 byte) dianggap lemah (rawan false positive).
fn is_weak(magic: &[u8]) -> bool {
    magic.len() <= 3
}

/// Memindai seluruh data untuk semua signature yang dikenal.
pub fn scan(data: &[u8]) -> Vec<Signature> {
    let mut out = Vec::new();
    for def in MAGICS {
        let mut start = 0usize;
        while start + def.magic.len() <= data.len() {
            if let Some(pos) = find(&data[start..], def.magic) {
                let offset = start + pos;
                out.push(Signature {
                    name: def.name.to_string(),
                    category: def.category.to_string(),
                    offset,
                    description: def.description.to_string(),
                    weak: is_weak(def.magic),
                });
                start = offset + 1;
            } else {
                break;
            }
        }
    }
    out.sort_by_key(|s| s.offset);
    out
}

/// Memindai hanya signature yang kuat (mengabaikan magic pendek).
pub fn scan_strong(data: &[u8]) -> Vec<Signature> {
    scan(data).into_iter().filter(|s| !s.weak).collect()
}

/// Pencarian substring sederhana (naive, cukup untuk magic pendek).
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deteksi_squashfs() {
        let data = b"xxxxhsqs\x00\x00\x00\x00rest";
        let sigs = scan_strong(data);
        assert!(sigs.iter().any(|s| s.name == "SquashFS (LE)"));
        let s = sigs.iter().find(|s| s.name == "SquashFS (LE)").unwrap();
        assert_eq!(s.offset, 4);
        assert_eq!(s.category, "filesystem");
    }

    #[test]
    fn deteksi_multiple() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0x27, 0x05, 0x19, 0x56]); // uImage (kuat)
        data.extend_from_slice(&[0u8; 10]);
        data.extend_from_slice(b"hsqs"); // squashfs (kuat)
        let sigs = scan_strong(&data);
        assert!(sigs.iter().any(|s| s.name == "U-Boot uImage"));
        assert!(sigs.iter().any(|s| s.name == "SquashFS (LE)"));
        assert!(sigs[0].offset <= sigs[1].offset);
    }

    #[test]
    fn weak_ditandai() {
        let data = b"MZ\x90\x00";
        let sigs = scan(data);
        let pe = sigs.iter().find(|s| s.name == "PE/EXE").unwrap();
        assert!(pe.weak);
        assert!(scan_strong(data).iter().all(|s| !s.weak));
    }

    #[test]
    fn uimage_dan_dtb() {
        let data = &[0x27, 0x05, 0x19, 0x56, 0, 0, 0, 0, 0xD0, 0x0D, 0xFE, 0xED];
        let sigs = scan_strong(data);
        assert!(sigs.iter().any(|s| s.name == "U-Boot uImage"));
        assert!(sigs.iter().any(|s| s.name == "FIT/DTB"));
    }

    #[test]
    fn data_kosong() {
        assert!(scan(b"").is_empty());
    }

    #[test]
    fn multiple_occurrences() {
        let data = b"hsqs___hsqs";
        let sigs = scan(data);
        let squash: Vec<_> = sigs.iter().filter(|s| s.name == "SquashFS (LE)").collect();
        assert_eq!(squash.len(), 2);
        assert_eq!(squash[0].offset, 0);
        assert_eq!(squash[1].offset, 7);
    }
}
