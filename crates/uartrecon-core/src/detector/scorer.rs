//! Algoritma penilaian (scoring) untuk data UART.
//!
//! Menentukan baudrate **tidak boleh** hanya berdasarkan "ada data". Kita
//! menggunakan beberapa indikator terukur dan menggabungkannya menjadi skor
//! 0..100. Skor ini adalah *confidence score*, **bukan** probabilitas statistik.
//!
//! ## Formula
//!
//! | Komponen                | Rentang  |
//! |-------------------------|----------|
//! | Printable ratio         | +0..40   |
//! | Valid line structure    | +0..15   |
//! | Known embedded pattern  | +0..25   |
//! | UTF-8 validity          | +0..10   |
//! | Binary randomness       | -0..30   |
//!
//! Skor akhir di-clamp ke 0..100.

use serde::{Deserialize, Serialize};

/// Rincian komponen skor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Kontribusi rasio karakter printable (0..40).
    pub printable: f32,
    /// Kontribusi struktur baris (0..15).
    pub line_structure: f32,
    /// Kontribusi pola embedded yang dikenal (0..25).
    pub known_pattern: f32,
    /// Kontribusi validitas UTF-8 (0..10).
    pub utf8: f32,
    /// Penalti karena data tampak acak (0..30, dikurangkan).
    pub randomness_penalty: f32,
}

impl ScoreBreakdown {
    /// Skor total (0..100), sudah di-clamp.
    pub fn total(&self) -> f32 {
        let raw = self.printable + self.line_structure + self.known_pattern + self.utf8
            - self.randomness_penalty;
        raw.clamp(0.0, 100.0)
    }
}

/// Pola-pola yang sering muncul di log boot embedded.
const KNOWN_PATTERNS: &[&str] = &[
    "U-Boot",
    "Das U-Boot",
    "Linux version",
    "BusyBox",
    "login:",
    "root@",
    "DRAM",
    "NAND",
    "MMC",
    "kernel",
    "Booting",
    "Starting kernel",
    "Hit any key",
    "CFE",
    "Barebox",
    "OpenWrt",
    "SquashFS",
    "cmdline",
    "bootargs",
    "init",
];

/// Karakter ASCII yang dianggap "valid" untuk log (mendapat skor positif).
fn is_log_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b' ' | b'\n'
                | b'\r'
                | b'\t'
                | b':'
                | b'/'
                | b'.'
                | b'_'
                | b'-'
                | b'['
                | b']'
                | b'('
                | b')'
                | b'='
                | b','
                | b';'
                | b'#'
                | b'*'
                | b'>'
                | b'<'
                | b'@'
                | b'%'
                | b'+'
        )
}

/// Apakah byte printable ASCII (32..=126) atau whitespace umum.
fn is_printable(b: u8) -> bool {
    (0x20..=0x7E).contains(&b) || b == b'\n' || b == b'\r' || b == b'\t'
}

/// Menghitung rasio byte printable (0..1).
pub fn printable_ratio(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let printable = data.iter().filter(|&&b| is_printable(b)).count();
    printable as f32 / data.len() as f32
}

/// Menghitung rasio byte yang "log-like" (0..1).
fn log_char_ratio(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let n = data.iter().filter(|&&b| is_log_char(b)).count();
    n as f32 / data.len() as f32
}

/// Skor struktur baris (0..15).
///
/// Memberi nilai untuk: keberadaan newline, panjang baris yang wajar, dan
/// konsistensi CR/LF.
pub fn line_structure_score(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let newlines = data.iter().filter(|&&b| b == b'\n').count();
    let crlf = data.windows(2).filter(|w| w == b"\r\n").count();

    // Tidak ada newline sama sekali -> kecil.
    let newline_component = if newlines == 0 {
        0.0
    } else {
        // Ada newline: beri skor bertahap, plateau setelah beberapa baris.
        (newlines as f32 / 3.0).min(1.0) * 7.0
    };

    // Konsistensi CR/LF: jika ada newline, proporsi yang CRLF menambah skor.
    let crlf_component = if newlines == 0 {
        0.0
    } else {
        (crlf as f32 / newlines as f32).min(1.0) * 4.0
    };

    // Panjang baris rata-rata wajar (10..120 char).
    let lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    let avg_len = if lines.is_empty() {
        0.0
    } else {
        lines.iter().map(|l| l.len()).sum::<usize>() as f32 / lines.len() as f32
    };
    let length_component = if (10.0..=120.0).contains(&avg_len) {
        4.0
    } else {
        0.0
    };

    (newline_component + crlf_component + length_component).min(15.0)
}

/// Skor pola embedded yang dikenal (0..25).
pub fn known_pattern_score(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let text = String::from_utf8_lossy(data);
    let mut hits = 0u32;
    for pat in KNOWN_PATTERNS {
        if text.contains(pat) {
            hits += 1;
        }
    }
    // Setiap pola yang cocok memberi nilai; plateau di 25.
    (hits as f32 * 8.0).min(25.0)
}

/// Skor validitas UTF-8 (0..10).
pub fn utf8_score(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    // Validasi UTF-8 secara ketat. Data UART biner akan sering gagal.
    match std::str::from_utf8(data) {
        Ok(s) => {
            // Beri skor penuh bila isinya masuk akal (bukan hanya kontrol).
            let control = s
                .chars()
                .filter(|c| c.is_control() && !c.is_whitespace())
                .count();
            if control * 10 > s.chars().count() {
                5.0
            } else {
                10.0
            }
        }
        Err(_) => 0.0,
    }
}

/// Penalti karena data tampak acak (0..30).
///
/// Menggunakan pendekatan sederhana: menghitung rasio byte high-bit (>=0x80)
/// dan entropi kasar distribusi byte. Data log didominasi ASCII 7-bit.
pub fn randomness_penalty(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let high_bit = data.iter().filter(|&&b| b >= 0x80).count() as f32 / data.len() as f32;
    let log_ratio = log_char_ratio(data);

    // Semakin banyak byte high-bit dan semakin sedikit log-char, semakin besar penalti.
    let high_penalty = high_bit * 20.0;
    let low_log_penalty = (1.0 - log_ratio).max(0.0) * 10.0;
    (high_penalty + low_log_penalty).min(30.0)
}

/// Menghitung seluruh rincian skor untuk sekumpulan byte.
pub fn score_bytes(data: &[u8]) -> ScoreBreakdown {
    let pr = printable_ratio(data);
    let printable = pr * 40.0;
    let line_structure = line_structure_score(data);
    let known_pattern = known_pattern_score(data);
    let utf8 = utf8_score(data);
    let randomness_penalty = randomness_penalty(data);

    ScoreBreakdown {
        printable,
        line_structure,
        known_pattern,
        utf8,
        randomness_penalty,
    }
}

/// Skor total saja (0..100).
pub fn score(data: &[u8]) -> f32 {
    score_bytes(data).total()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_kosong_skor_nol() {
        assert_eq!(score(&[]), 0.0);
    }

    #[test]
    fn log_uboot_skor_tinggi() {
        let data =
            b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nHit any key to stop autoboot\r\n";
        let s = score(data);
        assert!(s > 70.0, "skor log U-Boot seharusnya tinggi, dapat {s}");
    }

    #[test]
    fn data_biner_acak_skor_rendah() {
        let data: Vec<u8> = (0..256).map(|i| (i * 37 + 91) as u8).collect();
        let s = score(&data);
        assert!(s < 40.0, "data biner seharusnya skor rendah, dapat {s}");
    }

    #[test]
    fn printable_ratio_benar() {
        assert_eq!(printable_ratio(b"abcd"), 1.0);
        assert_eq!(printable_ratio(&[0x00, 0xFF]), 0.0);
    }

    #[test]
    fn pola_dikenali() {
        let s = known_pattern_score(b"Linux version 5.10 BusyBox login:");
        assert!(s >= 24.0);
    }

    #[test]
    fn breakdown_total_konsisten() {
        let data = b"Linux version 5.10 (gcc)\r\nBusyBox v1.35\r\nlogin: ";
        let b = score_bytes(data);
        assert!((b.total() - score(data)).abs() < f32::EPSILON);
    }

    #[test]
    fn skor_dibatasi_100() {
        // Data yang sangat "bagus" tidak boleh melebihi 100.
        let data = b"U-Boot Linux BusyBox login: root@ DRAM NAND MMC kernel Booting\r\n".repeat(50);
        assert!(score(&data) <= 100.0);
    }
}
