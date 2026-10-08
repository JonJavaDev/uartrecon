//! Scanner baudrate adaptif.
//!
//! Karena USB-UART biasa tidak memberi raw edge timing ke aplikasi, kita
//! memakai strategi **adaptive baud scanning + data scoring**, bukan deteksi
//! baud fisik. Lihat dokumentasi proyek untuk penjelasan batasan ini.
//!
//! Agar bisa diuji tanpa hardware, scanning bekerja di atas trait
//! [`SampleSource`]. Implementasi nyata (`SerialSampleSource`) membungkus
//! [`crate::serial::connection::Connection`], sedangkan test memakai
//! [`MockSampleSource`].

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::detector::scorer::{ScoreBreakdown, score_bytes};
use crate::error::Result;
use crate::serial::config::SerialFormat;

/// Daftar baudrate umum (300..2_000_000).
pub const COMMON_BAUDRATES: &[u32] = &[
    300, 600, 1200, 2400, 4800, 9600, 14_400, 19_200, 28_800, 38_400, 57_600, 115_200, 230_400,
    460_800, 921_600, 1_000_000, 1_500_000, 2_000_000,
];

/// Baudrate yang diprioritaskan untuk perangkat embedded (diuji lebih dulu).
pub const EMBEDDED_PRIORITY_BAUDRATES: &[u32] = &[
    9600, 19_200, 38_400, 57_600, 115_200, 230_400, 460_800, 921_600,
];

/// Sumber byte untuk diuji pada sebuah baudrate.
///
/// Implementasi nyata membuka port pada baud tertentu dan membaca; mock
/// mengembalikan byte yang sudah disiapkan.
pub trait SampleSource {
    /// Mengambil sampel byte pada baudrate & format tertentu.
    ///
    /// Mengembalikan `None` bila port tidak dapat dibuka/dikonfigurasi pada
    /// kombinasi tersebut (bukan error fatal, cukup dilewati).
    fn sample(
        &mut self,
        baudrate: u32,
        format: SerialFormat,
        duration: Duration,
    ) -> Result<Option<Vec<u8>>>;
}

/// Hasil pengujian satu kandidat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaudCandidate {
    /// Baudrate yang diuji.
    pub baudrate: u32,
    /// Format yang dipakai saat menguji.
    pub format: SerialFormat,
    /// Jumlah byte yang terbaca.
    pub bytes_read: usize,
    /// Rincian skor.
    pub breakdown: ScoreBreakdown,
}

impl BaudCandidate {
    /// Skor total 0..100.
    pub fn score(&self) -> f32 {
        self.breakdown.total()
    }
}

/// Hasil akhir pemindaian baudrate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaudScanResult {
    /// Semua kandidat terurut menurun berdasarkan skor.
    pub ranking: Vec<BaudCandidate>,
    /// Kandidat terbaik, bila ada yang menghasilkan traffic.
    pub best: Option<BaudCandidate>,
}

impl BaudScanResult {
    /// Confidence (0..100) dari kandidat terbaik, dibulatkan.
    pub fn confidence(&self) -> u8 {
        self.best
            .as_ref()
            .map(|c| c.score().round().clamp(0.0, 100.0) as u8)
            .unwrap_or(0)
    }

    /// Apakah ada kandidat dengan traffic yang cukup meyakinkan.
    ///
    /// Ambang: minimal ada byte terbaca dan skor >= `min_score`.
    pub fn is_reliable(&self, min_score: f32) -> bool {
        self.best
            .as_ref()
            .map(|c| c.bytes_read > 0 && c.score() >= min_score)
            .unwrap_or(false)
    }
}

/// Opsi pemindaian baudrate.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Daftar baudrate yang akan diuji (urut prioritas).
    pub baudrates: Vec<u32>,
    /// Durasi baca per kandidat.
    pub duration: Duration,
    /// Format yang diuji (default `8N1`).
    pub format: SerialFormat,
    /// Ambang minimal byte agar kandidat dianggap ada traffic.
    pub min_bytes: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            baudrates: EMBEDDED_PRIORITY_BAUDRATES.to_vec(),
            duration: Duration::from_millis(700),
            format: SerialFormat::EIGHT_N_ONE,
            min_bytes: 4,
        }
    }
}

/// Melakukan pemindaian baudrate terhadap sebuah [`SampleSource`].
///
/// Semua kandidat disimpan (bukan hanya yang terbaik), sesuai spesifikasi.
pub fn scan<S: SampleSource>(source: &mut S, opts: &ScanOptions) -> Result<BaudScanResult> {
    let mut ranking: Vec<BaudCandidate> = Vec::new();

    for &baud in &opts.baudrates {
        let sampled = source.sample(baud, opts.format, opts.duration)?;
        let bytes = match sampled {
            Some(b) => b,
            None => continue, // port tak bisa dibuka pada baud ini; lewati
        };
        let breakdown = score_bytes(&bytes);
        let candidate = BaudCandidate {
            baudrate: baud,
            format: opts.format,
            bytes_read: bytes.len(),
            breakdown,
        };
        tracing::debug!(
            baudrate = baud,
            bytes = candidate.bytes_read,
            score = candidate.score(),
            "kandidat baudrate"
        );
        ranking.push(candidate);
    }

    // Urutkan menurun berdasarkan skor, lalu berdasarkan jumlah byte.
    ranking.sort_by(|a, b| {
        b.score()
            .partial_cmp(&a.score())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.bytes_read.cmp(&a.bytes_read))
    });

    let best = ranking
        .iter()
        .find(|c| c.bytes_read >= opts.min_bytes)
        .cloned();

    Ok(BaudScanResult { ranking, best })
}

/// `SampleSource` berbasis data in-memory, untuk test tanpa hardware.
///
/// Memetakan baudrate -> data yang "seharusnya" terbaca pada baud itu. Baudrate
/// yang tidak dipetakan dianggap tidak bisa dibuka (`None`).
pub struct MockSampleSource {
    /// Pemetaan baudrate ke data yang dikembalikan.
    pub data: std::collections::HashMap<u32, Vec<u8>>,
}

impl MockSampleSource {
    /// Membuat mock dari pasangan `(baudrate, data)`.
    pub fn new(pairs: impl IntoIterator<Item = (u32, Vec<u8>)>) -> Self {
        MockSampleSource {
            data: pairs.into_iter().collect(),
        }
    }
}

impl SampleSource for MockSampleSource {
    fn sample(
        &mut self,
        baudrate: u32,
        _format: SerialFormat,
        _duration: Duration,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(&baudrate).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_uboot() -> Vec<u8> {
        b"U-Boot 2021.10 (Jan 01 2021)\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nHit any key to stop autoboot\r\n".to_vec()
    }

    fn sample_garbage() -> Vec<u8> {
        (0..200u32)
            .map(|i| (i.wrapping_mul(137) ^ 0x5A) as u8)
            .collect()
    }

    #[test]
    fn deteksi_baudrate_benar_di_antara_noise() {
        let mut src = MockSampleSource::new([
            (9600, sample_garbage()),
            (115_200, sample_uboot()),
            (57_600, sample_garbage()),
        ]);
        let opts = ScanOptions {
            baudrates: vec![9600, 19_200, 38_400, 57_600, 115_200],
            ..Default::default()
        };
        let result = scan(&mut src, &opts).unwrap();
        let best = result.best.clone().expect("harus ada kandidat terbaik");
        assert_eq!(best.baudrate, 115_200);
        assert!(result.is_reliable(70.0));
        assert!(result.confidence() >= 70);
    }

    #[test]
    fn ranking_memuat_semua_kandidat() {
        let mut src = MockSampleSource::new([(9600, sample_garbage()), (115_200, sample_uboot())]);
        let opts = ScanOptions {
            baudrates: vec![9600, 115_200],
            ..Default::default()
        };
        let result = scan(&mut src, &opts).unwrap();
        assert_eq!(result.ranking.len(), 2);
        // Terurut menurun.
        assert!(result.ranking[0].score() >= result.ranking[1].score());
    }

    #[test]
    fn tanpa_traffic_tidak_reliable() {
        let mut src = MockSampleSource::new([]);
        let result = scan(&mut src, &ScanOptions::default()).unwrap();
        assert!(result.best.is_none());
        assert!(!result.is_reliable(50.0));
        assert_eq!(result.confidence(), 0);
    }

    #[test]
    fn min_bytes_mempengaruhi_best() {
        let mut src = MockSampleSource::new([(115_200, b"ab".to_vec())]);
        let opts = ScanOptions {
            baudrates: vec![115_200],
            min_bytes: 10,
            ..Default::default()
        };
        let result = scan(&mut src, &opts).unwrap();
        assert!(result.best.is_none(), "traffic terlalu sedikit");
    }
}
