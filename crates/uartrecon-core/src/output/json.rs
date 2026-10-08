//! Formatter JSON.

use crate::error::Result;

/// Serialisasi nilai apa pun menjadi JSON pretty.
pub fn to_json_pretty<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}
