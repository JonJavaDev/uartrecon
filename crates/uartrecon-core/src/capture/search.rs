//! Pencarian pola dalam capture: literal, regex, dan heksadesimal.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Satu hasil pencarian.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Offset byte tempat match dimulai.
    pub offset: usize,
    /// Panjang match (byte).
    pub length: usize,
    /// Cuplikan teks sekitar match.
    pub snippet: String,
}

/// Mode pencarian.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Literal byte (case-sensitive).
    Literal,
    /// Regex (pada representasi lossy UTF-8).
    Regex,
    /// Pola hex, mis. `DE AD BE EF`.
    Hex,
    /// Case-insensitive literal.
    LiteralInsensitive,
}

/// Opsi pencarian.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Mode pencarian.
    pub mode: SearchMode,
    /// Konteks byte di kiri/kanan snippet.
    pub context: usize,
    /// Maksimum hasil.
    pub max_results: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions {
            mode: SearchMode::Literal,
            context: 16,
            max_results: 1000,
        }
    }
}

/// Mencari pola dalam data sesuai opsi.
pub fn search(data: &[u8], pattern: &str, opts: &SearchOptions) -> Result<Vec<SearchMatch>> {
    match opts.mode {
        SearchMode::Literal => Ok(search_bytes(data, pattern.as_bytes(), false, opts)),
        SearchMode::LiteralInsensitive => Ok(search_bytes(data, pattern.as_bytes(), true, opts)),
        SearchMode::Hex => {
            let needle = parse_hex_pattern(pattern)?;
            Ok(search_bytes(data, &needle, false, opts))
        }
        SearchMode::Regex => search_regex(data, pattern, opts),
    }
}

fn snippet(data: &[u8], offset: usize, length: usize, context: usize) -> String {
    let start = offset.saturating_sub(context);
    let end = (offset + length + context).min(data.len());
    let slice = &data[start..end];
    let text: String = slice
        .iter()
        .map(|&b| {
            if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    text
}

fn search_bytes(
    data: &[u8],
    needle: &[u8],
    insensitive: bool,
    opts: &SearchOptions,
) -> Vec<SearchMatch> {
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > data.len() {
        return out;
    }
    let needle_lower: Vec<u8> = needle.iter().map(|b| b.to_ascii_lowercase()).collect();

    let mut i = 0usize;
    while i + needle.len() <= data.len() {
        let matched = if insensitive {
            data[i..i + needle.len()]
                .iter()
                .map(|b| b.to_ascii_lowercase())
                .eq(needle_lower.iter().copied())
        } else {
            &data[i..i + needle.len()] == needle
        };
        if matched {
            out.push(SearchMatch {
                offset: i,
                length: needle.len(),
                snippet: snippet(data, i, needle.len(), opts.context),
            });
            if out.len() >= opts.max_results {
                break;
            }
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

fn search_regex(data: &[u8], pattern: &str, opts: &SearchOptions) -> Result<Vec<SearchMatch>> {
    let re = regex::Regex::new(pattern)
        .map_err(|e| Error::InvalidConfig(format!("regex tidak valid: {e}")))?;
    // Konversi ke string lossy; offset dalam byte tetap dipertahankan untuk ASCII.
    let text = String::from_utf8_lossy(data);
    let mut out = Vec::new();
    for m in re.find_iter(&text) {
        if out.len() >= opts.max_results {
            break;
        }
        let offset = m.start();
        let length = m.end() - m.start();
        out.push(SearchMatch {
            offset,
            length,
            snippet: snippet(data, offset, length, opts.context),
        });
    }
    Ok(out)
}

/// Parse pola hex seperti `DE AD BE EF` atau `DEADBEEF` atau `de:ad:be:ef`.
pub fn parse_hex_pattern(pattern: &str) -> Result<Vec<u8>> {
    let cleaned: String = pattern.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if cleaned.is_empty() {
        return Err(Error::InvalidConfig("pola hex kosong".into()));
    }
    if cleaned.len() % 2 != 0 {
        return Err(Error::InvalidConfig(format!(
            "pola hex harus berjumlah genap digit: '{pattern}'"
        )));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let bytes = cleaned.as_bytes();
    for chunk in bytes.chunks(2) {
        let s = std::str::from_utf8(chunk).expect("ascii hexdigit");
        let b = u8::from_str_radix(s, 16)
            .map_err(|_| Error::InvalidConfig(format!("byte hex tidak valid: '{s}'")))?;
        out.push(b);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_literal() {
        let data = b"U-Boot 2021.10\r\nDRAM: 512 MiB";
        let opts = SearchOptions::default();
        let matches = search(data, "DRAM", &opts).unwrap();
        assert_eq!(matches.len(), 1);
        // "DRAM" dimulai setelah "U-Boot 2021.10\r\n" (16 byte).
        assert_eq!(matches[0].offset, 16);
    }

    #[test]
    fn search_literal_insensitive() {
        let data = b"U-BOOT is booting";
        let opts = SearchOptions {
            mode: SearchMode::LiteralInsensitive,
            ..Default::default()
        };
        let matches = search(data, "u-boot", &opts).unwrap();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn search_hex() {
        let data = &[0x00, 0xDE, 0xAD, 0xBE, 0xEF, 0x00];
        let opts = SearchOptions {
            mode: SearchMode::Hex,
            ..Default::default()
        };
        let matches = search(data, "DE AD BE EF", &opts).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].offset, 1);
    }

    #[test]
    fn search_regex() {
        let data = b"Linux version 5.10.0-gcc and 4.4.14";
        let opts = SearchOptions {
            mode: SearchMode::Regex,
            ..Default::default()
        };
        let matches = search(data, r"\d+\.\d+\.\d+", &opts).unwrap();
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn parse_hex_berbagai_format() {
        assert_eq!(
            parse_hex_pattern("DEADBEEF").unwrap(),
            vec![0xDE, 0xAD, 0xBE, 0xEF]
        );
        assert_eq!(
            parse_hex_pattern("de:ad:be:ef").unwrap(),
            vec![0xDE, 0xAD, 0xBE, 0xEF]
        );
        assert_eq!(parse_hex_pattern("DE AD").unwrap(), vec![0xDE, 0xAD]);
        assert!(parse_hex_pattern("ABC").is_err());
        assert!(parse_hex_pattern("").is_err());
    }

    #[test]
    fn max_results_dihormati() {
        let data = b"aaaaaaaaaa";
        let opts = SearchOptions {
            mode: SearchMode::Literal,
            max_results: 3,
            ..Default::default()
        };
        let matches = search(data, "a", &opts).unwrap();
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn snippet_mengandung_konteks() {
        let data = b"prefix_MATCH_suffix";
        let opts = SearchOptions {
            context: 7,
            ..Default::default()
        };
        let matches = search(data, "MATCH", &opts).unwrap();
        assert!(matches[0].snippet.contains("prefix"));
        assert!(matches[0].snippet.contains("suffix"));
    }
}
