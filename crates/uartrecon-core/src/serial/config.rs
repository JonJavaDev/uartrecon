//! Konfigurasi UART: baudrate, data bits, parity, stop bits, dan format.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Paritas UART.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParityCfg {
    /// None (N).
    None,
    /// Even (E).
    Even,
    /// Odd (O).
    Odd,
}

impl ParityCfg {
    /// Huruf penanda paritas (N/E/O).
    pub fn letter(self) -> char {
        match self {
            ParityCfg::None => 'N',
            ParityCfg::Even => 'E',
            ParityCfg::Odd => 'O',
        }
    }

    /// Parse dari huruf penanda (case-insensitive).
    pub fn from_letter(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'N' => Some(ParityCfg::None),
            'E' => Some(ParityCfg::Even),
            'O' => Some(ParityCfg::Odd),
            _ => None,
        }
    }
}

/// Format baudrate UART (mis. `8N1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialFormat {
    /// Jumlah data bit (7 atau 8).
    pub data_bits: u8,
    /// Paritas.
    pub parity: ParityCfg,
    /// Jumlah stop bit (1 atau 2).
    pub stop_bits: u8,
}

impl Default for SerialFormat {
    fn default() -> Self {
        SerialFormat {
            data_bits: 8,
            parity: ParityCfg::None,
            stop_bits: 1,
        }
    }
}

impl SerialFormat {
    /// Format paling umum `8N1`.
    pub const EIGHT_N_ONE: SerialFormat = SerialFormat {
        data_bits: 8,
        parity: ParityCfg::None,
        stop_bits: 1,
    };

    /// Membuat format baru dengan validasi nilai yang didukung.
    pub fn new(data_bits: u8, parity: ParityCfg, stop_bits: u8) -> Result<Self> {
        if data_bits != 7 && data_bits != 8 {
            return Err(Error::InvalidConfig(format!(
                "data bits harus 7 atau 8, dapat {data_bits}"
            )));
        }
        if stop_bits != 1 && stop_bits != 2 {
            return Err(Error::InvalidConfig(format!(
                "stop bits harus 1 atau 2, dapat {stop_bits}"
            )));
        }
        Ok(SerialFormat {
            data_bits,
            parity,
            stop_bits,
        })
    }

    /// Label ringkas, mis. `8N1`.
    pub fn label(&self) -> String {
        format!(
            "{}{}{}",
            self.data_bits,
            self.parity.letter(),
            self.stop_bits
        )
    }

    /// Parse label seperti `8N1`, `7E1`, `8O2` (case-insensitive).
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 3 {
            return Err(Error::InvalidConfig(format!(
                "format '{s}' tidak valid; contoh: 8N1"
            )));
        }
        let data_bits = chars[0]
            .to_digit(10)
            .ok_or_else(|| Error::InvalidConfig(format!("data bits tidak valid pada '{s}'")))?
            as u8;
        let parity = ParityCfg::from_letter(chars[1])
            .ok_or_else(|| Error::InvalidConfig(format!("parity tidak valid pada '{s}'")))?;
        let stop_bits = chars[2]
            .to_digit(10)
            .ok_or_else(|| Error::InvalidConfig(format!("stop bits tidak valid pada '{s}'")))?
            as u8;
        SerialFormat::new(data_bits, parity, stop_bits)
    }
}

/// Konfigurasi UART lengkap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialConfig {
    /// Baudrate (mis. 115200).
    pub baudrate: u32,
    /// Format (data/parity/stop).
    pub format: SerialFormat,
}

impl SerialConfig {
    /// Membuat konfigurasi baru.
    pub fn new(baudrate: u32, format: SerialFormat) -> Self {
        SerialConfig { baudrate, format }
    }

    /// Label ringkas, mis. `115200 8N1`.
    pub fn label(&self) -> String {
        format!("{} {}", self.baudrate, self.format.label())
    }
}

impl Default for SerialConfig {
    fn default() -> Self {
        SerialConfig {
            baudrate: 115_200,
            format: SerialFormat::EIGHT_N_ONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_format() {
        assert_eq!(SerialFormat::EIGHT_N_ONE.label(), "8N1");
    }

    #[test]
    fn parse_format_roundtrip() {
        for label in ["8N1", "7E1", "8O2", "7N2"] {
            let f = SerialFormat::parse(label).unwrap();
            assert_eq!(f.label(), label);
        }
    }

    #[test]
    fn parse_format_invalid() {
        assert!(SerialFormat::parse("9N1").is_err());
        assert!(SerialFormat::parse("8X1").is_err());
        assert!(SerialFormat::parse("abc").is_err());
    }

    #[test]
    fn config_label() {
        let c = SerialConfig::default();
        assert_eq!(c.label(), "115200 8N1");
    }
}
