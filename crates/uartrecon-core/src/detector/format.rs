//! Deteksi format UART (data bits / parity / stop bits).
//!
//! Setelah baudrate diketahui, kita mencoba beberapa format dan memilih yang
//! menghasilkan skor statistik terbaik. Strategi sama dengan deteksi baudrate.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::detector::baudrate::SampleSource;
use crate::detector::scorer::score_bytes;
use crate::error::Result;
use crate::serial::config::{ParityCfg, SerialFormat};

/// Format yang diuji, dalam urutan prioritas.
pub const CANDIDATE_FORMATS: &[SerialFormat] = &[
    SerialFormat {
        data_bits: 8,
        parity: ParityCfg::None,
        stop_bits: 1,
    },
    SerialFormat {
        data_bits: 8,
        parity: ParityCfg::Even,
        stop_bits: 1,
    },
    SerialFormat {
        data_bits: 8,
        parity: ParityCfg::Odd,
        stop_bits: 1,
    },
    SerialFormat {
        data_bits: 8,
        parity: ParityCfg::None,
        stop_bits: 2,
    },
    SerialFormat {
        data_bits: 7,
        parity: ParityCfg::Even,
        stop_bits: 1,
    },
    SerialFormat {
        data_bits: 7,
        parity: ParityCfg::Odd,
        stop_bits: 1,
    },
];

/// Hasil pengujian satu format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatCandidate {
    /// Format yang diuji.
    pub format: SerialFormat,
    /// Jumlah byte terbaca.
    pub bytes_read: usize,
    /// Skor 0..100.
    pub score: f32,
}

/// Hasil deteksi format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatScanResult {
    /// Semua kandidat, terurut menurun berdasarkan skor.
    pub ranking: Vec<FormatCandidate>,
    /// Format terbaik.
    pub best: Option<FormatCandidate>,
}

/// Opsi pemindaian format.
#[derive(Debug, Clone)]
pub struct FormatScanOptions {
    /// Baudrate yang sudah diketahui.
    pub baudrate: u32,
    /// Format kandidat yang diuji (urut prioritas).
    pub formats: Vec<SerialFormat>,
    /// Durasi baca per kandidat.
    pub duration: Duration,
    /// Ambang minimal byte.
    pub min_bytes: usize,
}

impl Default for FormatScanOptions {
    fn default() -> Self {
        FormatScanOptions {
            baudrate: 115_200,
            formats: CANDIDATE_FORMATS.to_vec(),
            duration: Duration::from_millis(500),
            min_bytes: 4,
        }
    }
}

/// Melakukan pemindaian format terhadap sebuah [`SampleSource`] pada baudrate tetap.
pub fn scan<S: SampleSource>(source: &mut S, opts: &FormatScanOptions) -> Result<FormatScanResult> {
    let mut ranking = Vec::new();
    for &format in &opts.formats {
        let sampled = source.sample(opts.baudrate, format, opts.duration)?;
        let bytes = match sampled {
            Some(b) => b,
            None => continue,
        };
        let score = score_bytes(&bytes).total();
        ranking.push(FormatCandidate {
            format,
            bytes_read: bytes.len(),
            score,
        });
    }

    ranking.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.bytes_read.cmp(&a.bytes_read))
    });

    let best = ranking
        .iter()
        .find(|c| c.bytes_read >= opts.min_bytes)
        .cloned();

    Ok(FormatScanResult { ranking, best })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock yang mengembalikan data bagus hanya untuk format tertentu.
    struct FormatMock {
        good: SerialFormat,
        data: Vec<u8>,
    }

    impl SampleSource for FormatMock {
        fn sample(
            &mut self,
            _baudrate: u32,
            format: SerialFormat,
            _duration: Duration,
        ) -> Result<Option<Vec<u8>>> {
            if format == self.good {
                Ok(Some(self.data.clone()))
            } else {
                // Data "rusak" untuk format lain.
                Ok(Some(
                    (0..self.data.len())
                        .map(|i| (i as u8).wrapping_mul(53))
                        .collect(),
                ))
            }
        }
    }

    #[test]
    fn deteksi_8n1() {
        let mut mock = FormatMock {
            good: SerialFormat::EIGHT_N_ONE,
            data: b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nBusyBox login:\r\n".to_vec(),
        };
        let result = scan(&mut mock, &FormatScanOptions::default()).unwrap();
        assert_eq!(result.best.unwrap().format.label(), "8N1");
    }

    #[test]
    fn deteksi_7e1() {
        let seven_e_one = SerialFormat {
            data_bits: 7,
            parity: ParityCfg::Even,
            stop_bits: 1,
        };
        let mut mock = FormatMock {
            good: seven_e_one,
            data: b"Linux version 5.10\r\nroot@device:/# \r\n".to_vec(),
        };
        let result = scan(&mut mock, &FormatScanOptions::default()).unwrap();
        assert_eq!(result.best.unwrap().format.label(), "7E1");
    }

    #[test]
    fn ranking_lengkap() {
        let mut mock = FormatMock {
            good: SerialFormat::EIGHT_N_ONE,
            data: b"U-Boot Linux BusyBox login: DRAM NAND\r\n".to_vec(),
        };
        let result = scan(&mut mock, &FormatScanOptions::default()).unwrap();
        assert_eq!(result.ranking.len(), CANDIDATE_FORMATS.len());
    }
}
