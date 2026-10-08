//! Deteksi & analisis Embedded Linux dari log/shell output.

use serde::{Deserialize, Serialize};

use crate::detector::fingerprint::Confidence;

/// Indikator sistem Linux yang terdeteksi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LinuxIndicators {
    /// Versi kernel (bila terdeteksi).
    pub kernel_version: Option<String>,
    /// Indikator `/proc`, `/sys`, `/etc`.
    pub proc_present: bool,
    /// Indikator shell root.
    pub root_shell: bool,
    /// Indikator login prompt.
    pub login_prompt: bool,
}

/// Hasil analisis Linux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinuxAnalysis {
    /// Apakah Linux terdeteksi.
    pub detected: bool,
    /// Tingkat kepercayaan.
    pub confidence: Confidence,
    /// Indikator detail.
    pub indicators: LinuxIndicators,
}

/// Saran perintah read-only untuk sistem Linux embedded.
///
/// Tool hanya **menyarankan**; user harus memilih dan mengirim sendiri.
pub const READONLY_SUGGESTIONS: &[(&str, &str)] = &[
    ("uname -a", "Kernel information"),
    ("cat /proc/cpuinfo", "CPU information"),
    ("cat /proc/meminfo", "Memory information"),
    ("cat /proc/mtd", "MTD partition information"),
    ("cat /proc/cmdline", "Kernel command line"),
    ("mount", "Mounted filesystems"),
    ("df -h", "Disk usage"),
    ("ls /dev", "Device nodes"),
];

/// Menganalisis apakah data menunjukkan Embedded Linux.
pub fn analyze(data: &[u8]) -> LinuxAnalysis {
    let text = String::from_utf8_lossy(data);
    let mut score = 0u32;
    let mut ind = LinuxIndicators::default();

    if let Some(caps) = regex::Regex::new(r"Linux version (\S+)")
        .ok()
        .and_then(|re| re.captures(&text))
    {
        ind.kernel_version = Some(caps[1].to_string());
        score += 45;
    }
    if text.contains("/proc") || text.contains("/sys") {
        ind.proc_present = true;
        score += 20;
    }
    if text.contains("/etc") {
        score += 10;
    }
    if text.contains("root@") {
        ind.root_shell = true;
        score += 15;
    }
    if text.contains("login:") {
        ind.login_prompt = true;
        score += 15;
    }
    if text.contains("BusyBox") {
        score += 10;
    }

    let score = score.min(100);
    let confidence = Confidence::from_score(score);
    LinuxAnalysis {
        detected: confidence != Confidence::Unknown,
        confidence,
        indicators: ind,
    }
}

/// Saran perintah read-only yang relevan dengan hasil analisis.
pub fn suggestions(analysis: &LinuxAnalysis) -> Vec<(String, String)> {
    if !analysis.detected {
        return Vec::new();
    }
    READONLY_SUGGESTIONS
        .iter()
        .map(|(c, d)| (c.to_string(), d.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deteksi_linux_lengkap() {
        let log =
            b"Linux version 5.10.0\r\n/proc /sys /etc\r\nBusyBox\r\nlogin: root\r\nroot@device:/#";
        let a = analyze(log);
        assert!(a.detected);
        assert_eq!(a.confidence, Confidence::Detected);
        assert_eq!(a.indicators.kernel_version.as_deref(), Some("5.10.0"));
        assert!(a.indicators.proc_present);
    }

    #[test]
    fn bukan_linux() {
        let a = analyze(b"U-Boot 2021.10\r\nHit any key\r\n");
        assert!(!a.detected);
        assert_eq!(a.confidence, Confidence::Unknown);
    }

    #[test]
    fn saran_hanya_bila_terdeteksi() {
        let yes = analyze(b"Linux version 5.10 /proc login:");
        assert!(!suggestions(&yes).is_empty());
        let no = analyze(b"garbage");
        assert!(suggestions(&no).is_empty());
    }
}
