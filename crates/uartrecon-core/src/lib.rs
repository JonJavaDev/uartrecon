//! # UARTRecon Core
//!
//! Engine inti yang platform-independent untuk reconnaissance UART:
//! enumerasi port serial, deteksi baudrate/format, capture, fingerprinting,
//! dan analyzer (U-Boot / Linux / MTD).
//!
//! Crate ini **tidak** bergantung pada `ratatui`, `clap`, atau GUI apa pun.
//! CLI, TUI, dan GUI masa depan semuanya dibangun di atas core ini.
//!
//! ## Prinsip desain
//!
//! - **Read-only first**: tidak ada operasi erase/write/flash-modify.
//! - **Raw data adalah source of truth**: semua analisis bekerja di atas byte mentah.
//! - **Testable tanpa hardware**: algoritma scoring/deteksi bisa diuji dengan data sintetis
//!   melalui trait [`detector::baudrate::SampleSource`].

pub mod analyzers;
pub mod capture;
pub mod config;
pub mod detector;
pub mod error;
pub mod i18n;
pub mod logic;
pub mod output;
pub mod profiles;
pub mod recovery;
pub mod safety;
pub mod serial;
pub mod terminal;
pub mod util;

pub use config::Config;
pub use error::{Error, Result};
pub use recovery::RecoveryPlan;
pub use safety::{Criticality, PartitionRule, SafetyVerdict};
pub use serial::config::{ParityCfg, SerialConfig};
pub use serial::ports::{PortKind, SerialPortInfo, list_ports};
