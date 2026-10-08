//! Capture engine: perekaman RX/TX, sesi, dan ekspor.
//!
//! Raw data adalah *source of truth*. Semua format lain (txt, hex, json)
//! diturunkan dari raw.

pub mod backup;
pub mod exporter;
pub mod recorder;
pub mod search;
pub mod session;

pub use backup::{BackupEntry, BackupManifest, parse_hexdump_c};
pub use recorder::Recorder;
pub use search::{SearchMatch, SearchMode, SearchOptions};
pub use session::{CaptureMetadata, Session, SessionPaths};
