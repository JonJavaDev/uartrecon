//! Deteksi otomatis: baudrate, format UART, dan fingerprint perangkat.
//!
//! Modul ini bekerja di atas data byte mentah dan **tidak** membutuhkan
//! hardware langsung, sehingga algoritmanya dapat diuji dengan data sintetis.

pub mod baudrate;
pub mod detection;
pub mod fingerprint;
pub mod format;
pub mod scorer;
pub mod source;

pub use baudrate::{
    BaudCandidate, BaudScanResult, COMMON_BAUDRATES, EMBEDDED_PRIORITY_BAUDRATES, MockSampleSource,
    SampleSource, ScanOptions,
};
pub use detection::{DetectOptions, DetectionResult, detect};
pub use fingerprint::{Confidence, Detection, Fingerprint};
pub use format::{CANDIDATE_FORMATS, FormatCandidate, FormatScanOptions, FormatScanResult};
pub use scorer::{ScoreBreakdown, score, score_bytes};
pub use source::SerialSampleSource;
