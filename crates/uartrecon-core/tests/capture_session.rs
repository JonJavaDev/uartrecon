//! Integration test: capture engine, sesi, dan ekspor.

use std::fs;

use uartrecon_core::capture::exporter;
use uartrecon_core::capture::{Recorder, Session};
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::util::hash_sha256;

fn tmp_root(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "uartrecon_it_{tag}_{}",
        uartrecon_core::util::now_unix()
    ));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn session_create_save_open() {
    let root = tmp_root("session");
    let cfg = SerialConfig::new(115_200, SerialFormat::EIGHT_N_ONE);
    let mut session = Session::create(&root, "stb01", "COM7", cfg).unwrap();

    let mut rec = Recorder::new();
    rec.push_rx(b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\n");
    rec.push_tx(b"\r\n");
    session.save(&rec).unwrap();

    // Semua artefak harus ada.
    assert!(session.paths.rx_raw().exists());
    assert!(session.paths.tx_raw().exists());
    assert!(session.paths.output_txt().exists());
    assert!(session.paths.output_hex().exists());
    assert!(session.paths.events().exists());
    assert!(session.paths.metadata().exists());

    // Buka ulang tanpa capture.
    let reopened = Session::open(&session.paths.dir).unwrap();
    assert_eq!(reopened.metadata.port, "COM7");
    assert_eq!(
        reopened.read_rx().unwrap(),
        b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\n"
    );
    assert_eq!(reopened.read_tx().unwrap(), b"\r\n");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn hash_konsisten_dengan_file() {
    let root = tmp_root("hash");
    let cfg = SerialConfig::default();
    let mut session = Session::create(&root, "h", "COM1", cfg).unwrap();
    let mut rec = Recorder::new();
    let payload = b"firmware-evidence-bytes";
    rec.push_rx(payload);
    session.save(&rec).unwrap();

    let on_disk = fs::read(session.paths.rx_raw()).unwrap();
    assert_eq!(hash_sha256(&on_disk), hash_sha256(payload));
    assert_eq!(
        session.metadata.rx_sha256.as_deref(),
        Some(hash_sha256(payload).as_str())
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ekspor_semua_format() {
    let data = b"U-Boot 2021.10\r\n\x00\xFF\r\n";

    let text = exporter::to_text(data);
    assert!(text.contains("U-Boot"));
    assert!(text.contains("\\x00"));

    let hexdump = exporter::to_hexdump(data);
    assert!(hexdump.contains("00000000"));

    let csv = exporter::to_csv(data);
    assert!(csv.starts_with("offset,byte,ascii\n"));

    let rows = exporter::to_csv_rows(data);
    assert_eq!(rows.len(), data.len());
}

#[test]
fn recorder_mempertahankan_arah() {
    let mut rec = Recorder::new();
    rec.push_rx(b"RX-DATA");
    rec.push_tx(b"TX-DATA");
    assert_eq!(rec.rx(), b"RX-DATA");
    assert_eq!(rec.tx(), b"TX-DATA");
    assert_eq!(rec.total_bytes(), 14);
}
