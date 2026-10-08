//! Ekspor data capture ke berbagai format: TXT, HEX, JSON, CSV.
//!
//! BIN adalah data mentah itu sendiri (tidak perlu fungsi khusus).

use serde::{Deserialize, Serialize};

use crate::capture::recorder::Chunk;

/// Mengubah byte menjadi teks (lossy UTF-8), mempertahankan baris.
///
/// Byte non-printable diganti dengan representasi aman agar tetap terbaca.
pub fn to_text(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len());
    for &b in data {
        match b {
            b'\n' | b'\r' | b'\t' => out.push(b as char),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02X}")),
        }
    }
    out
}

/// Membuat hexdump klasik (offset, 16 byte hex, dan ASCII).
pub fn to_hexdump(data: &[u8]) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = i * 16;
        let mut hex_part = String::with_capacity(16 * 3);
        let mut ascii_part = String::with_capacity(16);
        for (j, &b) in chunk.iter().enumerate() {
            hex_part.push_str(&format!("{b:02X} "));
            if j == 7 {
                hex_part.push(' ');
            }
            ascii_part.push(if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '.'
            });
        }
        // Pad agar kolom ASCII rata.
        while hex_part.len() < 16 * 3 + 1 {
            hex_part.push(' ');
        }
        out.push_str(&format!("{offset:08X}  {hex_part} |{ascii_part}|\n"));
    }
    out
}

/// Satu baris CSV untuk hexdump (offset,hex,ascii).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvRow {
    /// Offset byte.
    pub offset: usize,
    /// Hex string byte tunggal.
    pub byte: String,
    /// Karakter ASCII (atau titik).
    pub ascii: String,
}

/// Mengubah data menjadi baris-baris CSV (satu byte per baris).
pub fn to_csv_rows(data: &[u8]) -> Vec<CsvRow> {
    data.iter()
        .enumerate()
        .map(|(i, &b)| CsvRow {
            offset: i,
            byte: format!("{b:02X}"),
            ascii: if (0x20..=0x7E).contains(&b) {
                (b as char).to_string()
            } else {
                ".".to_string()
            },
        })
        .collect()
}

/// Mengubah data menjadi string CSV lengkap.
pub fn to_csv(data: &[u8]) -> String {
    let mut out = String::from("offset,byte,ascii\n");
    for row in to_csv_rows(data) {
        let ascii = if row.ascii == "," {
            "\",\"".to_string()
        } else {
            row.ascii
        };
        out.push_str(&format!("{},{},{}\n", row.offset, row.byte, ascii));
    }
    out
}

/// Serialisasi events ke JSON.
pub fn events_to_json(events: &[Chunk]) -> crate::error::Result<String> {
    Ok(serde_json::to_string_pretty(events)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_menjaga_printable() {
        assert_eq!(to_text(b"ABC\n"), "ABC\n");
    }

    #[test]
    fn text_mengganti_nonprintable() {
        assert_eq!(to_text(&[0x00, 0xFF]), "\\x00\\xFF");
    }

    #[test]
    fn hexdump_format_benar() {
        let dump = to_hexdump(b"Hello, World!1234");
        assert!(dump.starts_with("00000000  "));
        assert!(dump.contains("|Hello, World!12"));
    }

    #[test]
    fn csv_berisi_header() {
        let csv = to_csv(b"AB");
        assert!(csv.starts_with("offset,byte,ascii\n"));
        assert!(csv.contains("0,41,A"));
        assert!(csv.contains("1,42,B"));
    }

    #[test]
    fn events_json_valid() {
        let events = vec![Chunk {
            offset_ms: 5,
            direction: crate::capture::recorder::Direction::Rx,
            len: 3,
        }];
        let json = events_to_json(&events).unwrap();
        assert!(json.contains("\"offset_ms\": 5"));
    }
}
