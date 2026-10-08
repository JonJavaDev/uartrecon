//! Implementasi [`SampleSource`] nyata di atas koneksi serial.
//!
//! Dipisahkan dari [`crate::detector::baudrate`] agar modul deteksi tetap
//! bebas hardware dan mudah diuji.

use std::time::Duration;

use crate::detector::baudrate::SampleSource;
use crate::error::Result;
use crate::serial::config::SerialFormat;
use crate::serial::connection::Connection;

/// `SampleSource` yang membuka port serial nyata pada tiap kandidat.
pub struct SerialSampleSource {
    port_name: String,
}

impl SerialSampleSource {
    /// Membuat sumber sampel untuk sebuah port.
    pub fn new(port_name: impl Into<String>) -> Self {
        SerialSampleSource {
            port_name: port_name.into(),
        }
    }
}

impl SampleSource for SerialSampleSource {
    fn sample(
        &mut self,
        baudrate: u32,
        format: SerialFormat,
        duration: Duration,
    ) -> Result<Option<Vec<u8>>> {
        let config = crate::serial::config::SerialConfig::new(baudrate, format);
        let mut conn = match Connection::open_with_timeout(&self.port_name, config, duration) {
            Ok(c) => c,
            Err(e) => {
                tracing::debug!(port = %self.port_name, baud = baudrate, error = %e, "gagal membuka port");
                return Ok(None);
            }
        };
        // Bersihkan buffer lama agar tidak bercampur dengan sampel baru.
        let _ = conn.clear_input();
        let data = conn.read_for(duration)?;
        Ok(Some(data))
    }
}
