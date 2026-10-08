//! Parser bootlog: mengekstrak baris dengan timestamp host relatif.
//!
//! Timestamp berasal dari **host capture**, bukan clock internal device.

use serde::{Deserialize, Serialize};

/// Satu baris bootlog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootLine {
    /// Timestamp host relatif (milidetik) saat baris diterima.
    pub offset_ms: u64,
    /// Isi baris (tanpa newline).
    pub text: String,
}

/// Hasil parsing bootlog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BootLog {
    /// Semua baris yang terparsing.
    pub lines: Vec<BootLine>,
}

impl BootLog {
    /// Jumlah baris.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Apakah kosong.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Render sebagai teks dengan timestamp `MM:SS.mmm`.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            let total_ms = line.offset_ms;
            let secs = total_ms / 1000;
            let ms = total_ms % 1000;
            let mm = secs / 60;
            let ss = secs % 60;
            out.push_str(&format!("{mm:02}.{ss:02}.{ms:03} {}\n", line.text));
        }
        out
    }
}

/// Mem-parsing byte menjadi [`BootLog`].
///
/// `base_offset_ms` adalah offset awal (mis. dari event pertama). Setiap baris
/// dianggap datang berurutan; karena raw tidak menyimpan timing per baris,
/// kita membagi offset total secara merata sebagai aproksimasi, kecuali bila
/// event timing tersedia (lihat [`parse_with_events`]).
pub fn parse(data: &[u8], base_offset_ms: u64) -> BootLog {
    let text = String::from_utf8_lossy(data);
    let raw_lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();

    let n = raw_lines.iter().filter(|l| !l.is_empty()).count().max(1);
    let mut lines = Vec::new();
    let mut idx = 0usize;
    for l in raw_lines {
        if l.is_empty() {
            continue;
        }
        // Aproksimasi: baris ke-i tersebar merata setelah base_offset.
        let offset = base_offset_ms + (idx as u64 * 50);
        lines.push(BootLine {
            offset_ms: offset,
            text: l.to_string(),
        });
        idx += 1;
        let _ = n;
    }
    BootLog { lines }
}

/// Mem-parsing dengan memakai timing event yang sebenarnya (lebih akurat).
pub fn parse_with_events(data: &[u8], events: &[crate::capture::recorder::Chunk]) -> BootLog {
    let text = String::from_utf8_lossy(data);
    let mut lines = Vec::new();
    // Gunakan offset event RX pertama sebagai basis.
    let base = events
        .iter()
        .find(|e| e.direction == crate::capture::recorder::Direction::Rx)
        .map(|e| e.offset_ms)
        .unwrap_or(0);

    let mut idx = 0u64;
    for l in text.split('\n').map(|l| l.trim_end_matches('\r')) {
        if l.is_empty() {
            continue;
        }
        // Ambil offset event RX ke-idx bila ada.
        let offset = events
            .iter()
            .filter(|e| e.direction == crate::capture::recorder::Direction::Rx)
            .nth(idx as usize)
            .map(|e| e.offset_ms)
            .unwrap_or(base + idx * 50);
        lines.push(BootLine {
            offset_ms: offset,
            text: l.to_string(),
        });
        idx += 1;
    }
    BootLog { lines }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_baris_dasar() {
        let log = parse(b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\n", 0);
        assert_eq!(log.len(), 2);
        assert_eq!(log.lines[0].text, "U-Boot 2021.10");
        assert_eq!(log.lines[1].text, "DRAM: 512 MiB");
    }

    #[test]
    fn timestamp_naik() {
        let log = parse(b"a\nb\nc\n", 100);
        assert!(log.lines[0].offset_ms >= 100);
        assert!(log.lines[1].offset_ms >= log.lines[0].offset_ms);
    }

    #[test]
    fn render_teks_berformat() {
        let log = parse(b"boot\n", 0);
        let text = log.to_text();
        assert!(text.contains("boot"));
        assert!(text.starts_with("00.00."));
    }

    #[test]
    fn baris_kosong_dilewati() {
        let log = parse(b"a\n\n\nb\n", 0);
        assert_eq!(log.len(), 2);
    }
}
