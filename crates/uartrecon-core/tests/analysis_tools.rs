//! Integration test: fitur analisis lanjutan (signatures, entropy, strings,
//! stats, diff, search).
//!
//! Semua test berjalan tanpa hardware.

use uartrecon_core::analyzers::{diff, entropy, signatures, stats, strings};
use uartrecon_core::capture::search::{self, SearchMode, SearchOptions};

const UBOOT_LOG: &[u8] = b"U-Boot 2021.10 (Jan 01 2021)\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nLinux version 5.10.0\r\nBusyBox v1.35\r\nlogin: root\r\nroot@stb:~# ";

// ---------- Signatures ----------

#[test]
fn signature_squashfs_dan_gzip() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x27, 0x05, 0x19, 0x56]); // uImage (kuat)
    data.extend_from_slice(&[0u8; 100]);
    data.extend_from_slice(b"hsqs"); // SquashFS (kuat)
    data.extend_from_slice(&[0u8; 100]);
    data.extend_from_slice(&[0x1F, 0x8B, 0x08]); // gzip (weak, 2 byte)

    let strong = signatures::scan_strong(&data);
    assert!(strong.iter().any(|s| s.name == "U-Boot uImage"));
    assert!(strong.iter().any(|s| s.name == "SquashFS (LE)"));

    // gzip bersifat weak, hanya muncul pada scan penuh.
    let all = signatures::scan(&data);
    assert!(all.iter().any(|s| s.name == "gzip"));
}

#[test]
fn signature_terurut_offset() {
    let data: &[u8] = b"hsqs____________UBI#";
    let sigs = signatures::scan(data);
    for w in sigs.windows(2) {
        assert!(w[0].offset <= w[1].offset);
    }
}

// ---------- Entropy ----------

#[test]
fn entropy_teks_vs_acak() {
    let text = UBOOT_LOG.repeat(50);
    let text_e = entropy::shannon(&text);

    let random: Vec<u8> = (0..=255u16).cycle().take(4096).map(|b| b as u8).collect();
    let random_e = entropy::shannon(&random);

    assert!(text_e < random_e, "teks harus lebih rendah dari acak");
    assert!(random_e > 7.5);
    assert!(text_e < 6.0);
}

#[test]
fn entropy_region_terdeteksi() {
    let mut data = vec![0u8; 4096];
    let random: Vec<u8> = (0..=255u16).cycle().take(2048).map(|b| b as u8).collect();
    data.extend_from_slice(&random);

    let regions = entropy::high_entropy_regions(&data, 512, 7.0);
    assert!(!regions.is_empty());
    assert!(regions.iter().any(|(off, _)| *off >= 4096));
}

// ---------- Strings ----------

#[test]
fn strings_ekstraksi() {
    let data = b"\x00\x01password=secret123\x00/bin/sh\x00https://example.com/api\x00";
    let extracted = strings::extract_ascii(data, 4);
    assert!(extracted.iter().any(|s| s.value.contains("password")));
    assert!(extracted.iter().any(|s| s.value.contains("/bin/sh")));
}

#[test]
fn strings_interesting() {
    let data = b"root:admin\x00http://evil.com\x00/bin/busybox\x00normal text here";
    let extracted = strings::extract_ascii(data, 3);
    let interesting = strings::find_interesting(&extracted);
    assert!(interesting.len() >= 2);
}

// ---------- Stats ----------

#[test]
fn stats_histogram() {
    let data = b"aaabbbbbcc";
    let s = stats::compute(data);
    assert_eq!(s.total, 10);
    assert_eq!(s.most_common, Some((b'b', 5)));
    assert_eq!(s.unique_bytes, 3);
    assert_eq!(s.printable, 10);
}

#[test]
fn stats_biner() {
    let data = [0x00, 0x00, 0xFF, 0xFF, 0x80, b'A'];
    let s = stats::compute(&data);
    assert_eq!(s.nulls, 2);
    assert_eq!(s.high_bit, 3);
    assert_eq!(s.printable, 1);
}

// ---------- Diff ----------

#[test]
fn diff_text_perubahan() {
    let a = "U-Boot 2021.10\nDRAM: 512 MiB\nNAND: 256 MiB";
    let b = "U-Boot 2021.10\nDRAM: 1024 MiB\nNAND: 256 MiB";
    let d = diff::diff_text(a, b);
    assert_eq!(d.added, 1);
    assert_eq!(d.removed, 1);
    assert!(d.render().contains("1024"));
}

#[test]
fn diff_biner_similarity() {
    let a = b"hello world firmware";
    let mut b = a.to_vec();
    b[6] = b'X';
    let d = diff::diff_bytes(a, &b);
    assert_eq!(d.differing.len(), 1);
    assert_eq!(d.differing[0].offset, 6);
    assert!(d.similarity() > 0.9);
}

// ---------- Search ----------

#[test]
fn search_literal_dan_regex() {
    let opts = SearchOptions {
        mode: SearchMode::Literal,
        ..Default::default()
    };
    let m = search::search(UBOOT_LOG, "BusyBox", &opts).unwrap();
    assert_eq!(m.len(), 1);

    let opts = SearchOptions {
        mode: SearchMode::Regex,
        ..Default::default()
    };
    let m = search::search(UBOOT_LOG, r"\d+ MiB", &opts).unwrap();
    assert_eq!(m.len(), 2);
}

#[test]
fn search_hex_pattern() {
    let data = &[0x00, 0xDE, 0xAD, 0xBE, 0xEF, 0x00];
    let opts = SearchOptions {
        mode: SearchMode::Hex,
        ..Default::default()
    };
    let m = search::search(data, "deadbeef", &opts).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].offset, 1);
}

// ---------- Kombinasi: pipeline analisis firmware ----------

#[test]
fn pipeline_analisis_lengkap() {
    // Bangun "firmware" sintetis: header uImage + log + SquashFS.
    let mut fw = Vec::new();
    fw.extend_from_slice(&[0x27, 0x05, 0x19, 0x56]); // uImage magic
    fw.extend_from_slice(UBOOT_LOG);
    fw.extend_from_slice(b"hsqs");
    fw.extend_from_slice(&[0u8; 64]);

    // 1. Signatures
    let sigs = signatures::scan_strong(&fw);
    assert!(sigs.iter().any(|s| s.name == "U-Boot uImage"));
    assert!(sigs.iter().any(|s| s.name == "SquashFS (LE)"));

    // 2. Strings
    let extracted = strings::extract_ascii(&fw, 6);
    assert!(extracted.iter().any(|s| s.value.contains("U-Boot")));

    // 3. Entropy
    let report = entropy::analyze(&fw, 256);
    assert!(report.overall > 0.0);

    // 4. Stats
    let s = stats::compute(&fw);
    assert_eq!(s.total, fw.len());
}

/// Mencari direktori testdata (naik dari CWD).
fn testdata_dir() -> std::path::PathBuf {
    let candidates = [
        std::path::PathBuf::from("testdata"),
        std::path::PathBuf::from("../../testdata"),
        std::path::PathBuf::from("../../../testdata"),
    ];
    candidates
        .into_iter()
        .find(|c| c.exists())
        .expect("direktori testdata tidak ditemukan")
}

#[test]
fn firmware_demo_dari_testdata() {
    let path = testdata_dir().join("firmware").join("demo.bin");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skip: {} tidak ada", path.display());
        return;
    };

    // Signatures.
    let sigs = signatures::scan_strong(&data);
    assert!(sigs.iter().any(|s| s.name == "U-Boot uImage"));
    assert!(sigs.iter().any(|s| s.name == "SquashFS (LE)"));

    // Strings menarik.
    let extracted = strings::extract_ascii(&data, 4);
    let interesting = strings::find_interesting(&extracted);
    assert!(interesting.iter().any(|(s, _)| s.value.contains("http://")));
    assert!(interesting.iter().any(|(s, _)| s.value.contains("/bin/")));
}
