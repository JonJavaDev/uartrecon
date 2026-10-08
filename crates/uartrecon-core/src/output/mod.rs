//! Format output: TXT, JSON, HEX, CSV.
//!
//! BIN tidak butuh formatter (raw byte apa adanya).

pub mod hex;
pub mod json;
pub mod text;

pub use hex::hexdump;
pub use json::to_json_pretty;
pub use text::render_report;
