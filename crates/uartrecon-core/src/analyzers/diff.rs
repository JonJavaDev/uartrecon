//! Diff dua capture: membandingkan dua sesi/capture untuk menemukan perbedaan.
//!
//! Berguna untuk membandingkan bootlog perangkat sebelum/sesudah perubahan,
//! atau dua firmware, atau dua capture dari device yang sama.

use serde::{Deserialize, Serialize};

/// Satu baris yang berbeda antara dua input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiffLine {
    /// Baris sama di kedua sisi.
    Same(String),
    /// Baris hanya ada di kiri (removed).
    Removed(String),
    /// Baris hanya ada di kanan (added).
    Added(String),
}

impl DiffLine {
    /// Prefix penanda untuk tampilan.
    pub fn prefix(&self) -> char {
        match self {
            DiffLine::Same(_) => ' ',
            DiffLine::Removed(_) => '-',
            DiffLine::Added(_) => '+',
        }
    }

    /// Isi baris.
    pub fn text(&self) -> &str {
        match self {
            DiffLine::Same(s) | DiffLine::Removed(s) | DiffLine::Added(s) => s,
        }
    }
}

/// Hasil diff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffResult {
    /// Baris-baris diff.
    pub lines: Vec<DiffLine>,
    /// Jumlah baris ditambahkan.
    pub added: usize,
    /// Jumlah baris dihapus.
    pub removed: usize,
    /// Jumlah baris sama.
    pub same: usize,
}

impl DiffResult {
    /// Apakah ada perbedaan.
    pub fn has_changes(&self) -> bool {
        self.added > 0 || self.removed > 0
    }

    /// Ringkasan singkat.
    pub fn summary(&self) -> String {
        format!(
            "{} added, {} removed, {} unchanged",
            self.added, self.removed, self.same
        )
    }

    /// Render sebagai teks unified-diff-like.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push(line.prefix());
            out.push(' ');
            out.push_str(line.text());
            out.push('\n');
        }
        out
    }
}

/// Menghitung LCS (Longest Common Subsequence) tabel untuk diff.
fn lcs_table(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let n = a.len();
    let m = b.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..n {
        for j in 0..m {
            dp[i + 1][j + 1] = if a[i] == b[j] {
                dp[i][j] + 1
            } else {
                dp[i][j + 1].max(dp[i + 1][j])
            };
        }
    }
    dp
}

/// Melakukan diff baris-per-baris antara dua string (dipisah `\n`).
pub fn diff_text(left: &str, right: &str) -> DiffResult {
    let a: Vec<&str> = left.lines().collect();
    let b: Vec<&str> = right.lines().collect();
    diff_lines(&a, &b)
}

/// Diff dua slice baris.
pub fn diff_lines(a: &[&str], b: &[&str]) -> DiffResult {
    let dp = lcs_table(a, b);
    let mut rev: Vec<DiffLine> = Vec::new();

    // Backtrack dari ujung (algoritma LCS standar).
    let mut i = a.len();
    let mut j = b.len();
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            rev.push(DiffLine::Same(a[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            rev.push(DiffLine::Removed(a[i - 1].to_string()));
            i -= 1;
        } else {
            rev.push(DiffLine::Added(b[j - 1].to_string()));
            j -= 1;
        }
    }
    while i > 0 {
        rev.push(DiffLine::Removed(a[i - 1].to_string()));
        i -= 1;
    }
    while j > 0 {
        rev.push(DiffLine::Added(b[j - 1].to_string()));
        j -= 1;
    }
    rev.reverse();

    let added = rev
        .iter()
        .filter(|l| matches!(l, DiffLine::Added(_)))
        .count();
    let removed = rev
        .iter()
        .filter(|l| matches!(l, DiffLine::Removed(_)))
        .count();
    let same = rev
        .iter()
        .filter(|l| matches!(l, DiffLine::Same(_)))
        .count();

    DiffResult {
        lines: rev,
        added,
        removed,
        same,
    }
}

/// Diff dua capture biner: melaporkan offset byte yang berbeda.
pub fn diff_bytes(a: &[u8], b: &[u8]) -> ByteDiff {
    let common = a.len().min(b.len());
    let mut differing = Vec::new();
    for i in 0..common {
        if a[i] != b[i] {
            differing.push(ByteDifference {
                offset: i,
                left: a[i],
                right: b[i],
            });
        }
    }
    let identical_prefix = differing.first().map(|d| d.offset).unwrap_or(common);
    ByteDiff {
        left_len: a.len(),
        right_len: b.len(),
        differing,
        identical_prefix,
    }
}

/// Satu perbedaan byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ByteDifference {
    /// Offset byte.
    pub offset: usize,
    /// Nilai di kiri.
    pub left: u8,
    /// Nilai di kanan.
    pub right: u8,
}

/// Hasil diff biner.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ByteDiff {
    /// Panjang data kiri.
    pub left_len: usize,
    /// Panjang data kanan.
    pub right_len: usize,
    /// Daftar perbedaan byte.
    pub differing: Vec<ByteDifference>,
    /// Panjang prefix identik.
    pub identical_prefix: usize,
}

impl ByteDiff {
    /// Apakah identik.
    pub fn is_identical(&self) -> bool {
        self.left_len == self.right_len && self.differing.is_empty()
    }

    /// Persentase byte identik (terhadap panjang maksimum).
    pub fn similarity(&self) -> f64 {
        let max_len = self.left_len.max(self.right_len);
        if max_len == 0 {
            return 1.0;
        }
        let same = max_len - self.differing.len();
        same as f64 / max_len as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_teks_sederhana() {
        let left = "line1\nline2\nline3";
        let right = "line1\nlineX\nline3";
        let d = diff_text(left, right);
        assert_eq!(d.added, 1);
        assert_eq!(d.removed, 1);
        assert_eq!(d.same, 2);
        assert!(d.has_changes());
    }

    #[test]
    fn diff_identik() {
        let d = diff_text("a\nb\nc", "a\nb\nc");
        assert!(!d.has_changes());
        assert_eq!(d.same, 3);
    }

    #[test]
    fn diff_semua_baru() {
        let d = diff_text("", "a\nb");
        assert_eq!(d.added, 2);
        assert_eq!(d.removed, 0);
    }

    #[test]
    fn diff_render() {
        let d = diff_text("a\nb", "a\nc");
        let text = d.render();
        assert!(text.contains("+ c"));
        assert!(text.contains("- b"));
    }

    #[test]
    fn diff_biner() {
        let a = [1u8, 2, 3, 4];
        let b = [1u8, 2, 9, 4];
        let d = diff_bytes(&a, &b);
        assert_eq!(d.differing.len(), 1);
        assert_eq!(d.differing[0].offset, 2);
        assert!(!d.is_identical());
        assert!(d.similarity() > 0.7);
    }

    #[test]
    fn diff_biner_identik() {
        let d = diff_bytes(b"hello", b"hello");
        assert!(d.is_identical());
        assert_eq!(d.similarity(), 1.0);
    }
}
