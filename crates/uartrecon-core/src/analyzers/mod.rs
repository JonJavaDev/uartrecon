//! Analyzer untuk log boot, Linux, U-Boot, MTD, dan analisis data lanjutan.
//!
//! Semua analyzer bersifat **read-only**: menerima byte dan mengembalikan
//! struktur hasil analisis.

pub mod bootlog;
pub mod diff;
pub mod entropy;
pub mod linux;
pub mod mtd;
pub mod signatures;
pub mod stats;
pub mod strings;
pub mod uboot;

pub use entropy::{EntropyClass, EntropyReport, shannon};
pub use signatures::{Signature, scan as scan_signatures, scan_strong};
pub use stats::Stats;
pub use strings::ExtractedString;
