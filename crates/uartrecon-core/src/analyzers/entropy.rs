//! Analisis entropi (Shannon).
//!
//! Entropi tinggi (~8 bit/byte) menandakan data terkompresi atau terenkripsi;
//! entropi rendah menandakan teks/struktur. Berguna untuk menemukan region
//! firmware yang terkompresi atau terenkripsi.

use serde::{Deserialize, Serialize};

/// Interpretasi tingkat entropi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntropyClass {
    /// Sangat terstruktur/teks (entropi < 3.5).
    Structured,
    /// Data campuran (3.5..=6.5).
    Mixed,
    /// Terkompresi (6.5..=7.5).
    Compressed,
    /// Terenkripsi/acak (> 7.5).
    Encrypted,
}

impl EntropyClass {
    /// Klasifikasi dari nilai entropi (bit/byte).
    pub fn from_entropy(e: f64) -> Self {
        if e < 3.5 {
            EntropyClass::Structured
        } else if e <= 6.5 {
            EntropyClass::Mixed
        } else if e <= 7.5 {
            EntropyClass::Compressed
        } else {
            EntropyClass::Encrypted
        }
    }

    /// Label.
    pub fn label(self) -> &'static str {
        match self {
            EntropyClass::Structured => "structured/text",
            EntropyClass::Mixed => "mixed",
            EntropyClass::Compressed => "compressed",
            EntropyClass::Encrypted => "encrypted/random",
        }
    }
}

/// Hasil analisis entropi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntropyReport {
    /// Entropi keseluruhan (bit/byte, 0..8).
    pub overall: f64,
    /// Klasifikasi keseluruhan.
    pub class: EntropyClass,
    /// Entropi per blok (untuk menemukan region terkompresi).
    pub blocks: Vec<EntropyBlock>,
    /// Ukuran blok (byte).
    pub block_size: usize,
}

/// Entropi satu blok.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntropyBlock {
    /// Offset byte awal blok.
    pub offset: usize,
    /// Entropi blok (bit/byte).
    pub entropy: f64,
    /// Klasifikasi blok.
    pub class: EntropyClass,
}

/// Menghitung entropi Shannon (bit/byte) dari sekumpulan byte.
pub fn shannon(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut entropy = 0.0;
    for &c in counts.iter() {
        if c > 0 {
            let p = c as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// Menganalisis entropi data, dengan opsi ukuran blok.
pub fn analyze(data: &[u8], block_size: usize) -> EntropyReport {
    let overall = shannon(data);
    let class = EntropyClass::from_entropy(overall);

    let mut blocks = Vec::new();
    if block_size > 0 {
        for (i, chunk) in data.chunks(block_size).enumerate() {
            let e = shannon(chunk);
            blocks.push(EntropyBlock {
                offset: i * block_size,
                entropy: e,
                class: EntropyClass::from_entropy(e),
            });
        }
    }

    EntropyReport {
        overall,
        class,
        blocks,
        block_size,
    }
}

/// Menganalisis dengan ukuran blok default 1024 byte.
pub fn analyze_default(data: &[u8]) -> EntropyReport {
    analyze(data, 1024)
}

/// Mencari region dengan entropi tinggi (kemungkinan compressed/encrypted).
///
/// Mengembalikan daftar `(offset, length)` region yang entropinya di atas
/// ambang `threshold` (bit/byte).
pub fn high_entropy_regions(data: &[u8], block_size: usize, threshold: f64) -> Vec<(usize, usize)> {
    if block_size == 0 || data.is_empty() {
        return Vec::new();
    }
    let mut regions = Vec::new();
    let mut current: Option<(usize, usize)> = None;

    for (i, chunk) in data.chunks(block_size).enumerate() {
        let offset = i * block_size;
        let e = shannon(chunk);
        if e >= threshold {
            match &mut current {
                Some((_, len)) => *len += chunk.len(),
                None => current = Some((offset, chunk.len())),
            }
        } else if let Some(region) = current.take() {
            regions.push(region);
        }
    }
    if let Some(region) = current {
        regions.push(region);
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropi_data_seragam_nol() {
        let data = vec![0xAAu8; 1000];
        assert_eq!(shannon(&data), 0.0);
    }

    #[test]
    fn entropi_acak_tinggi() {
        // 0..=255 berulang -> distribusi merata -> entropi ~8.
        let data: Vec<u8> = (0..=255u16).cycle().take(4096).map(|b| b as u8).collect();
        let e = shannon(&data);
        assert!((e - 8.0).abs() < 0.01, "entropi = {e}");
    }

    #[test]
    fn entropi_teks_sedang() {
        let data = b"U-Boot 2021.10 DRAM: 512 MiB Linux version BusyBox login: root".repeat(20);
        let e = shannon(&data);
        assert!((2.0..6.0).contains(&e), "entropi teks = {e}");
    }

    #[test]
    fn klasifikasi() {
        assert_eq!(EntropyClass::from_entropy(1.0), EntropyClass::Structured);
        assert_eq!(EntropyClass::from_entropy(5.0), EntropyClass::Mixed);
        assert_eq!(EntropyClass::from_entropy(7.0), EntropyClass::Compressed);
        assert_eq!(EntropyClass::from_entropy(7.9), EntropyClass::Encrypted);
    }

    #[test]
    fn region_entropi_tinggi() {
        let mut data = vec![0u8; 2048]; // entropi rendah
        // Tambahkan 1024 byte acak.
        let random: Vec<u8> = (0..=255u16).cycle().take(1024).map(|b| b as u8).collect();
        data.extend_from_slice(&random);
        let regions = high_entropy_regions(&data, 512, 7.0);
        assert!(!regions.is_empty());
        // Region harus mencakup area acak (offset >= 2048).
        assert!(regions.iter().any(|(off, _)| *off >= 2048));
    }

    #[test]
    fn blok_analysis() {
        let data: Vec<u8> = (0..2048u32).map(|i| (i % 256) as u8).collect();
        let report = analyze(&data, 1024);
        assert_eq!(report.blocks.len(), 2);
        assert!(report.overall > 7.0);
    }
}
