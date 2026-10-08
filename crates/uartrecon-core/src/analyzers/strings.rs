//! Ekstraksi string printable (mirip perintah `strings`).
//!
//! Berguna untuk menarik teks dari firmware/capture: path, URL, command,
//! versi, kredensial default, dsb.

use serde::{Deserialize, Serialize};

/// Satu string yang diekstrak.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedString {
    /// Offset byte tempat string dimulai.
    pub offset: usize,
    /// Isi string.
    pub value: String,
}

/// Ekstraksi string ASCII printable dengan panjang minimum tertentu.
pub fn extract_ascii(data: &[u8], min_len: usize) -> Vec<ExtractedString> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &b) in data.iter().enumerate() {
        let printable = (0x20..=0x7E).contains(&b);
        match (printable, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= min_len {
                    out.push(ExtractedString {
                        offset: s,
                        value: String::from_utf8_lossy(&data[s..i]).to_string(),
                    });
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start
        && data.len() - s >= min_len
    {
        out.push(ExtractedString {
            offset: s,
            value: String::from_utf8_lossy(&data[s..]).to_string(),
        });
    }
    out
}

/// Ekstraksi string yang mencakup karakter whitespace (spasi/tab) juga.
pub fn extract_ascii_ws(data: &[u8], min_len: usize) -> Vec<ExtractedString> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &b) in data.iter().enumerate() {
        let printable = (0x20..=0x7E).contains(&b) || b == b'\t';
        match (printable, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                let slice = &data[s..i];
                let trimmed_len = slice.iter().filter(|&&c| c != b'\t').count();
                if trimmed_len >= min_len {
                    out.push(ExtractedString {
                        offset: s,
                        value: String::from_utf8_lossy(slice).to_string(),
                    });
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(ExtractedString {
            offset: s,
            value: String::from_utf8_lossy(&data[s..]).to_string(),
        });
    }
    out
}

/// Filter string berdasarkan kata kunci (case-insensitive).
pub fn filter_keyword<'a>(
    strings: &'a [ExtractedString],
    keyword: &str,
) -> Vec<&'a ExtractedString> {
    let kw = keyword.to_ascii_lowercase();
    strings
        .iter()
        .filter(|s| s.value.to_ascii_lowercase().contains(&kw))
        .collect()
}

/// Kategori string "menarik" untuk reconnaissance.
pub fn interesting_patterns() -> &'static [(&'static str, &'static str)] {
    &[
        ("password", "kemungkinan kredensial"),
        ("passwd", "file password"),
        ("root:", "entri root"),
        ("admin", "akun admin"),
        ("http://", "URL"),
        ("https://", "URL"),
        ("ftp://", "URL"),
        ("telnet", "protokol"),
        ("ssh", "protokol"),
        ("/bin/", "path binary"),
        ("/etc/", "path config"),
        ("/proc/", "path proc"),
        ("/dev/", "device node"),
        ("token", "kemungkinan token"),
        ("secret", "kemungkinan rahasia"),
        ("key=", "kemungkinan key"),
        ("version", "versi"),
        ("busybox", "shell"),
        ("login", "login prompt"),
        ("wifi", "network"),
        ("ssid", "network"),
    ]
}

/// Mengembalikan string yang cocok dengan pola menarik.
pub fn find_interesting(strings: &[ExtractedString]) -> Vec<(ExtractedString, &'static str)> {
    let mut out = Vec::new();
    for s in strings {
        let lower = s.value.to_ascii_lowercase();
        for (pat, desc) in interesting_patterns() {
            if lower.contains(pat) {
                out.push((s.clone(), *desc));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ekstraksi_dasar() {
        let data = b"\x00\x01Hello World\x00\xffByeLong";
        let strings = extract_ascii(data, 4);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].value, "Hello World");
        assert_eq!(strings[0].offset, 2);
        assert_eq!(strings[1].value, "ByeLong");
    }

    #[test]
    fn min_len_dihormati() {
        let data = b"ab\x00abcdef";
        let strings = extract_ascii(data, 4);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].value, "abcdef");
    }

    #[test]
    fn filter_keyword_bekerja() {
        let data = b"root:admin\x00password=123\x00other";
        let strings = extract_ascii(data, 3);
        let found = filter_keyword(&strings, "password");
        assert_eq!(found.len(), 1);
        assert!(found[0].value.contains("password"));
    }

    #[test]
    fn interesting_ditemukan() {
        let data = b"http://example.com\x00/bin/sh\x00root:secret\x00";
        let strings = extract_ascii(data, 3);
        let interesting = find_interesting(&strings);
        assert!(interesting.len() >= 2);
    }

    #[test]
    fn data_tanpa_string() {
        let data = [0u8, 1, 2, 3, 255, 254];
        assert!(extract_ascii(&data, 4).is_empty());
    }
}
