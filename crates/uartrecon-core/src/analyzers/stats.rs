//! Statistik data: histogram byte, distribusi, dan metrik ringkas.

use serde::{Deserialize, Serialize};

/// Statistik ringkas dari sekumpulan byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    /// Total byte.
    pub total: usize,
    /// Jumlah byte unik (0..=256).
    pub unique_bytes: usize,
    /// Byte paling sering muncul.
    pub most_common: Option<(u8, usize)>,
    /// Byte paling jarang muncul (yang ada).
    pub least_common: Option<(u8, usize)>,
    /// Jumlah byte printable ASCII.
    pub printable: usize,
    /// Jumlah byte null (0x00).
    pub nulls: usize,
    /// Jumlah byte high-bit (>= 0x80).
    pub high_bit: usize,
    /// Histogram 256 bin.
    #[serde(with = "histogram_serde")]
    pub histogram: [usize; 256],
}

/// Modul serde untuk array 256 elemen (serde tidak impl untuk array > 32).
mod histogram_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(data: &[usize; 256], s: S) -> Result<S::Ok, S::Error> {
        data.as_slice().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[usize; 256], D::Error> {
        let v: Vec<usize> = Vec::deserialize(d)?;
        let mut out = [0usize; 256];
        for (i, val) in v.into_iter().take(256).enumerate() {
            out[i] = val;
        }
        Ok(out)
    }
}

/// Menghitung statistik dari data.
pub fn compute(data: &[u8]) -> Stats {
    let mut histogram = [0usize; 256];
    for &b in data {
        histogram[b as usize] += 1;
    }
    let unique_bytes = histogram.iter().filter(|c| **c > 0).count();

    let most_common = histogram
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .max_by_key(|(_, c)| **c)
        .map(|(i, c)| (i as u8, *c));
    let least_common = histogram
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .min_by_key(|(_, c)| **c)
        .map(|(i, c)| (i as u8, *c));

    let printable = data.iter().filter(|&&b| (0x20..=0x7E).contains(&b)).count();
    let nulls = histogram[0];
    let high_bit = data.iter().filter(|&&b| b >= 0x80).count();

    Stats {
        total: data.len(),
        unique_bytes,
        most_common,
        least_common,
        printable,
        nulls,
        high_bit,
        histogram,
    }
}

impl Stats {
    /// Rasio byte printable (0..1).
    pub fn printable_ratio(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.printable as f64 / self.total as f64
        }
    }

    /// Render histogram sebagai teks (ASCII bar chart) untuk N byte teratas.
    pub fn histogram_text(&self, top_n: usize) -> String {
        let mut entries: Vec<(usize, usize)> = self
            .histogram
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(i, c)| (i, *c))
            .collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.1));
        entries.truncate(top_n);

        let max = entries.first().map(|e| e.1).unwrap_or(1);
        let mut out = String::new();
        for (byte, count) in entries {
            let bar_len = (count * 40 / max.max(1)).max(1);
            let ch = if (0x20..=0x7E).contains(&(byte as u8)) {
                byte as u8 as char
            } else {
                '.'
            };
            out.push_str(&format!(
                "  0x{byte:02X} '{ch}' {:>8} {}\n",
                count,
                "#".repeat(bar_len)
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistik_dasar() {
        let data = b"aaabbbbcd";
        let s = compute(data);
        assert_eq!(s.total, 9);
        assert_eq!(s.unique_bytes, 4);
        assert_eq!(s.most_common, Some((b'b', 4)));
        assert_eq!(s.printable, 9);
    }

    #[test]
    fn null_dan_high_bit() {
        let data = [0x00, 0x00, 0xFF, 0x80, b'A'];
        let s = compute(&data);
        assert_eq!(s.nulls, 2);
        assert_eq!(s.high_bit, 2);
        assert_eq!(s.printable, 1);
    }

    #[test]
    fn printable_ratio_benar() {
        let s = compute(b"AB\x00\x00");
        assert!((s.printable_ratio() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn data_kosong() {
        let s = compute(b"");
        assert_eq!(s.total, 0);
        assert_eq!(s.unique_bytes, 0);
        assert_eq!(s.most_common, None);
        assert_eq!(s.printable_ratio(), 0.0);
    }

    #[test]
    fn histogram_text_tidak_panic() {
        let s = compute(b"aaabbc");
        let text = s.histogram_text(5);
        assert!(text.contains("0x61"));
    }
}
