//! Sistem sesi: metadata, penyimpanan, dan pemuatan ulang.
//!
//! Setiap sesi adalah direktori di bawah `sessions/` berisi raw RX/TX,
//! output txt/hex, dan metadata JSON. Sesi dapat dibuka kembali tanpa
//! melakukan capture ulang.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::capture::exporter;
use crate::capture::recorder::Recorder;
use crate::error::{Error, Result};
use crate::serial::config::SerialConfig;
use crate::util::{date_ymd, now_unix};

/// Metadata sebuah sesi capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureMetadata {
    /// Nama sesi (mis. `2026-10-08_stb01`).
    pub session: String,
    /// Port yang dipakai.
    pub port: String,
    /// Baudrate.
    pub baudrate: u32,
    /// Data bits.
    pub data_bits: u8,
    /// Pariti (`N`/`E`/`O`).
    pub parity: String,
    /// Stop bits.
    pub stop_bits: u8,
    /// Timestamp Unix (detik) saat sesi dibuat.
    pub timestamp: u64,
    /// Timestamp manusiawi (UTC).
    pub timestamp_utc: String,
    /// Nama adapter USB-UART (bila diketahui).
    pub adapter: Option<String>,
    /// SHA-256 dari `rx.raw` (diisi saat disimpan).
    pub rx_sha256: Option<String>,
    /// SHA-256 dari `tx.raw` (diisi saat disimpan).
    pub tx_sha256: Option<String>,
    /// Catatan bebas.
    pub notes: Option<String>,
}

impl CaptureMetadata {
    /// Membuat metadata dari konfigurasi dan nama port.
    pub fn new(session: impl Into<String>, port: impl Into<String>, config: SerialConfig) -> Self {
        let ts = now_unix();
        let (y, m, d) = date_ymd(ts);
        CaptureMetadata {
            session: session.into(),
            port: port.into(),
            baudrate: config.baudrate,
            data_bits: config.format.data_bits,
            parity: config.format.parity.letter().to_string(),
            stop_bits: config.format.stop_bits,
            timestamp: ts,
            timestamp_utc: format!("{y:04}-{m:02}-{d:02}"),
            adapter: None,
            rx_sha256: None,
            tx_sha256: None,
            notes: None,
        }
    }

    /// Konfigurasi UART dari metadata.
    pub fn serial_config(&self) -> Result<SerialConfig> {
        let parity = crate::serial::config::ParityCfg::from_letter(
            self.parity.chars().next().unwrap_or('N'),
        )
        .ok_or_else(|| Error::InvalidConfig(format!("parity tidak valid: {}", self.parity)))?;
        let format =
            crate::serial::config::SerialFormat::new(self.data_bits, parity, self.stop_bits)?;
        Ok(SerialConfig::new(self.baudrate, format))
    }
}

/// Path-path di dalam sebuah sesi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPaths {
    /// Direktori sesi.
    pub dir: PathBuf,
}

impl SessionPaths {
    /// Membuat dari direktori sesi.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        SessionPaths { dir: dir.into() }
    }

    /// Path `metadata.json`.
    pub fn metadata(&self) -> PathBuf {
        self.dir.join("metadata.json")
    }
    /// Path `rx.raw`.
    pub fn rx_raw(&self) -> PathBuf {
        self.dir.join("rx.raw")
    }
    /// Path `tx.raw`.
    pub fn tx_raw(&self) -> PathBuf {
        self.dir.join("tx.raw")
    }
    /// Path `output.txt`.
    pub fn output_txt(&self) -> PathBuf {
        self.dir.join("output.txt")
    }
    /// Path `output.hex`.
    pub fn output_hex(&self) -> PathBuf {
        self.dir.join("output.hex")
    }
    /// Path `events.json`.
    pub fn events(&self) -> PathBuf {
        self.dir.join("events.json")
    }
    /// Path `analysis.json`.
    pub fn analysis(&self) -> PathBuf {
        self.dir.join("analysis.json")
    }
}

/// Sebuah sesi: metadata + data.
#[derive(Debug, Clone)]
pub struct Session {
    /// Metadata sesi.
    pub metadata: CaptureMetadata,
    /// Path sesi.
    pub paths: SessionPaths,
}

impl Session {
    /// Membuat sesi baru di bawah `root_dir` dengan nama `session`.
    ///
    /// Direktori dibuat bila belum ada.
    pub fn create(
        root_dir: impl AsRef<Path>,
        session: impl Into<String>,
        port: impl Into<String>,
        config: SerialConfig,
    ) -> Result<Self> {
        let name = session.into();
        let dir = root_dir.as_ref().join(&name);
        fs::create_dir_all(&dir)?;
        let metadata = CaptureMetadata::new(name, port, config);
        Ok(Session {
            metadata,
            paths: SessionPaths::new(dir),
        })
    }

    /// Membuat nama sesi default dengan tanggal (mis. `2026-10-08_session`).
    pub fn default_name(tag: &str) -> String {
        let (y, m, d) = date_ymd(now_unix());
        format!("{y:04}-{m:02}-{d:02}_{tag}")
    }

    /// Menyimpan seluruh data dari recorder ke disk.
    ///
    /// Menulis `rx.raw`, `tx.raw`, `output.txt`, `output.hex`, `events.json`,
    /// dan `metadata.json`. Hash SHA-256 dihitung dan disimpan di metadata.
    pub fn save(&mut self, recorder: &Recorder) -> Result<()> {
        fs::create_dir_all(&self.paths.dir)?;

        fs::write(self.paths.rx_raw(), recorder.rx())?;
        fs::write(self.paths.tx_raw(), recorder.tx())?;

        let text = exporter::to_text(recorder.rx());
        fs::write(self.paths.output_txt(), text)?;

        let hexdump = exporter::to_hexdump(recorder.rx());
        fs::write(self.paths.output_hex(), hexdump)?;

        let events_json = serde_json::to_string_pretty(recorder.events())?;
        fs::write(self.paths.events(), events_json)?;

        self.metadata.rx_sha256 = Some(crate::util::hash_sha256(recorder.rx()));
        self.metadata.tx_sha256 = Some(crate::util::hash_sha256(recorder.tx()));

        self.write_metadata()?;
        tracing::info!(session = %self.metadata.session, "sesi disimpan");
        Ok(())
    }

    /// Menulis `metadata.json` saja.
    pub fn write_metadata(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(self.paths.metadata(), json)?;
        Ok(())
    }

    /// Membuka sesi dari direktori yang sudah ada.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let metadata_path = dir.join("metadata.json");
        let data = fs::read_to_string(&metadata_path)?;
        let metadata: CaptureMetadata = serde_json::from_str(&data)?;
        Ok(Session {
            metadata,
            paths: SessionPaths::new(dir),
        })
    }

    /// Membaca kembali `rx.raw`.
    pub fn read_rx(&self) -> Result<Vec<u8>> {
        Ok(fs::read(self.paths.rx_raw())?)
    }

    /// Membaca kembali `tx.raw`.
    pub fn read_tx(&self) -> Result<Vec<u8>> {
        Ok(fs::read(self.paths.tx_raw())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial::config::SerialFormat;

    fn tmp_dir(tag: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("uartrecon_test_{tag}_{}", now_unix()));
        let _ = fs::remove_dir_all(&base);
        base
    }

    #[test]
    fn metadata_dari_config() {
        let cfg = SerialConfig::new(115_200, SerialFormat::EIGHT_N_ONE);
        let md = CaptureMetadata::new("s1", "COM7", cfg);
        assert_eq!(md.baudrate, 115_200);
        assert_eq!(md.parity, "N");
        assert_eq!(md.serial_config().unwrap(), cfg);
    }

    #[test]
    fn save_dan_open_roundtrip() {
        let root = tmp_dir("roundtrip");
        let cfg = SerialConfig::new(115_200, SerialFormat::EIGHT_N_ONE);
        let mut session = Session::create(&root, "s_test", "COM7", cfg).unwrap();
        let mut rec = Recorder::new();
        rec.push_rx(b"U-Boot 2021.10\r\n");
        rec.push_tx(b"\r\n");
        session.save(&rec).unwrap();

        assert!(session.paths.rx_raw().exists());
        assert!(session.paths.tx_raw().exists());
        assert!(session.paths.output_txt().exists());
        assert!(session.paths.output_hex().exists());
        assert!(session.paths.events().exists());

        let reopened = Session::open(&session.paths.dir).unwrap();
        assert_eq!(reopened.metadata.session, "s_test");
        assert_eq!(reopened.read_rx().unwrap(), b"U-Boot 2021.10\r\n");
        assert_eq!(reopened.read_tx().unwrap(), b"\r\n");
        assert!(reopened.metadata.rx_sha256.is_some());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn default_name_berformat() {
        let name = Session::default_name("stb01");
        assert!(name.ends_with("_stb01"));
        assert_eq!(name.len(), "2026-10-08_stb01".len());
    }
}
