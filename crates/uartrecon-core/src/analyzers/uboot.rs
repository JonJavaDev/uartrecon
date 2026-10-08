//! Deteksi & analisis U-Boot.
//!
//! Tool **tidak** mengubah U-Boot environment. Hanya analisis read-only.

use serde::{Deserialize, Serialize};

use crate::detector::fingerprint::Confidence;

/// Indikator U-Boot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UBootIndicators {
    /// Versi U-Boot (bila terdeteksi).
    pub version: Option<String>,
    /// Prompt `=>` atau `U-Boot>` terlihat.
    pub prompt_seen: bool,
    /// Ada indikator environment (`bootcmd`/`bootargs`).
    pub env_indicators: bool,
    /// Ada pesan "Hit any key".
    pub hit_any_key: bool,
}

/// Hasil analisis U-Boot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UBootAnalysis {
    /// Apakah U-Boot terdeteksi.
    pub detected: bool,
    /// Tingkat kepercayaan.
    pub confidence: Confidence,
    /// Indikator detail.
    pub indicators: UBootIndicators,
}

/// Menganalisis apakah data menunjukkan U-Boot.
pub fn analyze(data: &[u8]) -> UBootAnalysis {
    let text = String::from_utf8_lossy(data);
    let mut score = 0u32;
    let mut ind = UBootIndicators::default();

    if let Some(caps) = regex::Regex::new(r"U-Boot (\S+)")
        .ok()
        .and_then(|re| re.captures(&text))
    {
        ind.version = Some(caps[1].to_string());
        score += 55;
    } else if text.contains("U-Boot") {
        score += 40;
    }
    if text.contains("Hit any key") {
        ind.hit_any_key = true;
        score += 20;
    }
    if regex::Regex::new(r"(?m)^\s*(=>|U-Boot>)")
        .map(|re| re.is_match(&text))
        .unwrap_or(false)
    {
        ind.prompt_seen = true;
        score += 20;
    }
    if text.contains("bootcmd") || text.contains("bootargs") {
        ind.env_indicators = true;
        score += 20;
    }

    let score = score.min(100);
    let confidence = Confidence::from_score(score);
    UBootAnalysis {
        detected: confidence != Confidence::Unknown,
        confidence,
        indicators: ind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deteksi_uboot_kuat() {
        let log = b"U-Boot 2021.10 (Jan 01 2021)\r\nHit any key to stop autoboot\r\n=> \r\n";
        let a = analyze(log);
        assert!(a.detected);
        assert_eq!(a.confidence, Confidence::Detected);
        assert_eq!(a.indicators.version.as_deref(), Some("2021.10"));
        assert!(a.indicators.hit_any_key);
    }

    #[test]
    fn bukan_uboot() {
        let a = analyze(b"Linux version 5.10\r\nlogin:");
        assert!(!a.detected);
    }

    #[test]
    fn env_indicators() {
        let a = analyze(b"U-Boot\r\nbootcmd=bootm 0x80000000\r\n");
        assert!(a.indicators.env_indicators);
    }
}
