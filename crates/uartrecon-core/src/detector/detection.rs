//! Orkestrasi deteksi: menggabungkan scanning baudrate dan format.
//!
//! Fungsi di sini menyatukan langkah-langkah agar CLI/TUI/GUI tidak
//! menduplikasi logika.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::detector::baudrate::{SampleSource, ScanOptions, scan};
use crate::detector::format::{FormatScanOptions, scan as scan_format};
use crate::error::Result;
use crate::serial::config::{SerialConfig, SerialFormat};

/// Hasil deteksi UART lengkap (baudrate + format).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Konfigurasi terbaik yang ditemukan.
    pub config: Option<SerialConfig>,
    /// Confidence 0..100.
    pub confidence: u8,
    /// Ranking baudrate (semua kandidat).
    pub baud_ranking: Vec<crate::detector::baudrate::BaudCandidate>,
    /// Ranking format (semua kandidat).
    pub format_ranking: Vec<crate::detector::format::FormatCandidate>,
}

/// Opsi deteksi lengkap.
#[derive(Debug, Clone)]
pub struct DetectOptions {
    /// Opsi scanning baudrate.
    pub baud: ScanOptions,
    /// Durasi baca per kandidat format.
    pub format_duration: Duration,
    /// Ambang minimal byte.
    pub min_bytes: usize,
}

impl Default for DetectOptions {
    fn default() -> Self {
        DetectOptions {
            baud: ScanOptions::default(),
            format_duration: Duration::from_millis(500),
            min_bytes: 4,
        }
    }
}

/// Menjalankan deteksi baudrate lalu format secara berurutan.
pub fn detect<S: SampleSource>(source: &mut S, opts: &DetectOptions) -> Result<DetectionResult> {
    let baud_result = scan(source, &opts.baud)?;

    let best_baud = match &baud_result.best {
        Some(b) => b,
        None => {
            return Ok(DetectionResult {
                config: None,
                confidence: 0,
                baud_ranking: baud_result.ranking,
                format_ranking: Vec::new(),
            });
        }
    };

    // Uji format pada baudrate terbaik.
    let fmt_opts = FormatScanOptions {
        baudrate: best_baud.baudrate,
        formats: crate::detector::format::CANDIDATE_FORMATS.to_vec(),
        duration: opts.format_duration,
        min_bytes: opts.min_bytes,
    };
    let fmt_result = scan_format(source, &fmt_opts)?;

    let best_format: SerialFormat = fmt_result
        .best
        .as_ref()
        .map(|c| c.format)
        .unwrap_or(SerialFormat::EIGHT_N_ONE);

    let config = SerialConfig::new(best_baud.baudrate, best_format);

    // Confidence = rata-rata skor baud terbaik dan format terbaik.
    let baud_score = best_baud.score();
    let fmt_score = fmt_result.best.as_ref().map(|c| c.score).unwrap_or(0.0);
    let confidence = ((baud_score + fmt_score) / 2.0).round().clamp(0.0, 100.0) as u8;

    Ok(DetectionResult {
        config: Some(config),
        confidence,
        baud_ranking: baud_result.ranking,
        format_ranking: fmt_result.ranking,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::baudrate::MockSampleSource;

    #[test]
    fn deteksi_lengkap_menemukan_115200_8n1() {
        let uboot = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nHit any key\r\n".to_vec();
        let garbage: Vec<u8> = (0..200u32)
            .map(|i| (i.wrapping_mul(137) ^ 0x5A) as u8)
            .collect();
        let mut src = MockSampleSource::new([(9600, garbage.clone()), (115_200, uboot)]);
        let opts = DetectOptions {
            baud: ScanOptions {
                baudrates: vec![9600, 115_200],
                ..Default::default()
            },
            ..Default::default()
        };
        let result = detect(&mut src, &opts).unwrap();
        let cfg = result.config.expect("harus terdeteksi");
        assert_eq!(cfg.baudrate, 115_200);
        assert_eq!(cfg.format.label(), "8N1");
        assert!(result.confidence >= 60);
    }

    #[test]
    fn tanpa_traffic_config_none() {
        let mut src = MockSampleSource::new([]);
        let result = detect(&mut src, &DetectOptions::default()).unwrap();
        assert!(result.config.is_none());
        assert_eq!(result.confidence, 0);
    }
}
