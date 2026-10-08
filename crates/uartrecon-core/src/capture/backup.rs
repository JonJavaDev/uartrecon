//! Backup engine: mem-parsing output `hexdump`/`base64` dari device menjadi
//! file biner, plus verifikasi checksum.
//!
//! Karena transfer via UART itu lambat dan rawan, backup bekerja dengan
//! mem-parse output teks yang dikirim device (mis. `hexdump -C /dev/mtd1`),
//! lalu merekonstruksi byte dan memverifikasi MD5 bila tersedia.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Satu baris hasil parse hexdump.
#[derive(Debug, Clone, PartialEq)]
pub struct HexdumpLine {
    /// Offset yang tertulis di hexdump.
    pub offset: u64,
    /// Byte hasil decode.
    pub bytes: Vec<u8>,
}

/// Mem-parsing output `hexdump -C` menjadi byte.
///
/// Format yang didukung (klasik `hexdump -C`):
/// ```text
/// 00000000  42 4f 4f 54 4c 4f 41  44 45 52 21 00 00 00 00  |BOOTLOADER!.....|
/// ```
pub fn parse_hexdump_c(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        // Cari bagian setelah offset (8 digit hex di awal).
        let trimmed = line.trim_start();
        // Harus diawali hex offset minimal 8 char.
        let hex_digits = trimmed
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        if hex_digits < 8 {
            continue;
        }
        // Bagian setelah offset sampai '|' (ASCII) adalah hex bytes.
        let rest = &trimmed[hex_digits..];
        let hex_part = rest.split('|').next().unwrap_or(rest);
        let mut bytes = Vec::new();
        for tok in hex_part.split_whitespace() {
            if tok.len() == 2
                && tok.chars().all(|c| c.is_ascii_hexdigit())
                && let Ok(b) = u8::from_str_radix(tok, 16)
            {
                bytes.push(b);
            }
        }
        if !bytes.is_empty() {
            out.extend_from_slice(&bytes);
        }
    }
    Ok(out)
}

/// Mem-parsing output `hexdump` format kanonik (tanpa `-C`):
/// `0000000 424f 4f54 4c4f ...` (word-based, big-endian grouping).
pub fn parse_hexdump_plain(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let hex_digits = trimmed
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        if hex_digits == 0 {
            continue;
        }
        // Lewati token offset pertama.
        let rest = &trimmed[hex_digits..];
        for tok in rest.split_whitespace() {
            if tok.chars().all(|c| c.is_ascii_hexdigit()) && tok.len() % 2 == 0 && tok.len() <= 16 {
                // Kelompok word: byte low dulu (little-endian grouping).
                let mut chunk = Vec::new();
                let b = tok.as_bytes();
                let mut i = 0;
                while i + 2 <= b.len() {
                    if let Ok(byte) =
                        u8::from_str_radix(std::str::from_utf8(&b[i..i + 2]).unwrap_or("00"), 16)
                    {
                        chunk.push(byte);
                    }
                    i += 2;
                }
                out.extend_from_slice(&chunk);
            }
        }
    }
    Ok(out)
}

/// Mem-parsing output base64 (mis. `base64 /dev/mtd1` atau per-baris).
///
/// Mendukung base64 standar (A-Za-z0-9+/=), mengabaikan whitespace.
pub fn parse_base64(text: &str) -> Result<Vec<u8>> {
    let cleaned: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    decode_base64(&cleaned)
}

/// Decoder base64 minimal (tanpa dependency).
fn decode_base64(s: &str) -> Result<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lut = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lut[c as usize] = i as u8;
    }

    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &c in bytes {
        if c == b'=' {
            break;
        }
        let v = lut[c as usize];
        if v == 255 {
            return Err(Error::InvalidConfig(format!(
                "karakter base64 tidak valid: '{}'",
                c as char
            )));
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Ok(out)
}

/// Metadata backup satu partisi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Nama partisi.
    pub partition: String,
    /// Nomor mtd.
    pub mtd: Option<u32>,
    /// Ukuran byte yang berhasil dipulihkan.
    pub size: usize,
    /// MD5 dari device (bila tersedia).
    pub md5_device: Option<String>,
    /// MD5 dari data yang dipulihkan host.
    pub md5_host: String,
    /// Apakah cocok.
    pub verified: bool,
}

impl BackupEntry {
    /// Membuat entri dengan verifikasi otomatis.
    pub fn new(
        partition: impl Into<String>,
        mtd: Option<u32>,
        data: &[u8],
        md5_device: Option<String>,
    ) -> Self {
        let md5_host = crate::util::md5_hex(data);
        let verified = md5_device
            .as_ref()
            .map(|d| d.eq_ignore_ascii_case(&md5_host))
            .unwrap_or(false);
        BackupEntry {
            partition: partition.into(),
            mtd,
            size: data.len(),
            md5_device,
            md5_host,
            verified,
        }
    }
}

/// Manifest backup (daftar semua partisi yang dibackup).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Nama device/target (mis. `ZTE B700V5S1`).
    pub device: String,
    /// Timestamp Unix.
    pub timestamp: u64,
    /// Daftar entri.
    pub entries: Vec<BackupEntry>,
}

impl BackupManifest {
    /// Membuat manifest baru.
    pub fn new(device: impl Into<String>) -> Self {
        BackupManifest {
            device: device.into(),
            timestamp: crate::util::now_unix(),
            entries: Vec::new(),
        }
    }

    /// Menambah entri.
    pub fn add(&mut self, entry: BackupEntry) {
        self.entries.push(entry);
    }

    /// Apakah semua entri terverifikasi.
    pub fn all_verified(&self) -> bool {
        self.entries.iter().all(|e| e.verified)
    }

    /// Serialisasi ke JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hexdump_c_benar() {
        let text = "\
00000000  42 4f 4f 54 4c 4f 41 44  45 52 21 00 00 00 00 00  |BOOTLOADER!.....|
00000010  00 5e 00 00 00 00 00 40  4e a0 ce d6 4e 46 49 49  |.^.....@N...NFII|
";
        let bytes = parse_hexdump_c(text).unwrap();
        assert_eq!(&bytes[0..10], b"BOOTLOADER");
        assert_eq!(bytes.len(), 32);
        assert_eq!(bytes[16], 0x00);
        assert_eq!(bytes[17], 0x5e);
    }

    #[test]
    fn parse_hexdump_c_dengan_bintang() {
        // Baris "*" (pengulangan) harus dilewati.
        let text = "\
00000000  ff ff ff ff ff ff ff ff  ff ff ff ff ff ff ff ff  |................|
*
00100000
";
        let bytes = parse_hexdump_c(text).unwrap();
        assert_eq!(bytes.len(), 16);
        assert!(bytes.iter().all(|&b| b == 0xff));
    }

    #[test]
    fn parse_base64_benar() {
        // "Hello" -> SGVsbG8=
        let bytes = parse_base64("SGVsbG8=").unwrap();
        assert_eq!(bytes, b"Hello");
    }

    #[test]
    fn base64_abaikan_whitespace() {
        let bytes = parse_base64("SGVs\nbG8=\n").unwrap();
        assert_eq!(bytes, b"Hello");
    }

    #[test]
    fn base64_invalid_error() {
        assert!(parse_base64("!!!").is_err());
    }

    #[test]
    fn backup_entry_verifikasi() {
        let data = b"BOOTLOADER!";
        let md5 = crate::util::md5_hex(data);
        let e = BackupEntry::new("boot", Some(1), data, Some(md5.clone()));
        assert!(e.verified);
        assert_eq!(e.size, 11);

        let bad = BackupEntry::new("boot", Some(1), data, Some("deadbeef".into()));
        assert!(!bad.verified);
    }

    #[test]
    fn manifest_json() {
        let mut m = BackupManifest::new("B700V5");
        m.add(BackupEntry::new("boot", Some(1), b"x", None));
        let json = m.to_json().unwrap();
        assert!(json.contains("B700V5"));
        assert!(json.contains("\"boot\""));
    }

    #[test]
    fn md5_dikenal() {
        // MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
        assert_eq!(
            crate::util::md5_hex(b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }
}
