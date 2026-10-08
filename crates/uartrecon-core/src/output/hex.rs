//! Formatter hex.

pub use crate::capture::exporter::to_hexdump as hexdump;

/// Mengubah byte menjadi string hex tanpa spasi (mis. `DEADBEEF`).
pub fn to_hex_string(data: &[u8]) -> String {
    hex::encode(data)
}
