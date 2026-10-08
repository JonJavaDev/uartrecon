//! Koneksi serial: membuka, mengonfigurasi, membaca, dan menulis.

use std::io::{Read, Write};
use std::time::{Duration, Instant};

use serialport::{SerialPort, SerialPortBuilder};

use crate::error::{Error, Result};
use crate::serial::config::{ParityCfg, SerialConfig};

/// Koneksi aktif ke sebuah port serial.
pub struct Connection {
    port: Box<dyn SerialPort>,
    config: SerialConfig,
    port_name: String,
}

impl Connection {
    /// Membuka port dengan konfigurasi yang diberikan.
    pub fn open(port_name: &str, config: SerialConfig) -> Result<Self> {
        let builder = build(port_name, config);
        let port = builder.open()?;
        tracing::info!(port = port_name, config = %config.label(), "serial port dibuka");
        Ok(Connection {
            port,
            config,
            port_name: port_name.to_string(),
        })
    }

    /// Membuka port dengan timeout baca tertentu (untuk scanner).
    pub fn open_with_timeout(
        port_name: &str,
        config: SerialConfig,
        timeout: Duration,
    ) -> Result<Self> {
        let builder = build(port_name, config).timeout(timeout);
        let port = builder.open()?;
        Ok(Connection {
            port,
            config,
            port_name: port_name.to_string(),
        })
    }

    /// Nama port.
    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    /// Konfigurasi aktif.
    pub fn config(&self) -> SerialConfig {
        self.config
    }

    /// Membaca hingga buffer penuh atau timeout.
    ///
    /// Mengembalikan jumlah byte yang dibaca (bisa 0 bila timeout).
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.port.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Membaca selama `duration` dan mengumpulkan semua byte.
    pub fn read_for(&mut self, duration: Duration) -> Result<Vec<u8>> {
        let deadline = Instant::now() + duration;
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        while Instant::now() < deadline {
            match self.read(&mut buf) {
                Ok(0) => {
                    // Tidak ada data; beri jeda kecil agar tidak busy-loop.
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }

    /// Mengirim data mentah (write).
    ///
    /// Catatan: ini operasi TX. Pemanggil bertanggung jawab memastikan data
    /// yang dikirim bersifat read-only/non-destruktif.
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let n = self.port.write(data)?;
        self.port.flush()?;
        Ok(n)
    }

    /// Mengosongkan buffer input.
    pub fn clear_input(&mut self) -> Result<()> {
        self.port.clear(serialport::ClearBuffer::Input)?;
        Ok(())
    }

    /// Menutup koneksi secara eksplisit.
    pub fn close(self) -> Result<()> {
        drop(self.port);
        Ok(())
    }
}

fn build(port_name: &str, config: SerialConfig) -> SerialPortBuilder {
    serialport::new(port_name, config.baudrate)
        .data_bits(match config.format.data_bits {
            7 => serialport::DataBits::Seven,
            _ => serialport::DataBits::Eight,
        })
        .parity(match config.format.parity {
            ParityCfg::None => serialport::Parity::None,
            ParityCfg::Even => serialport::Parity::Even,
            ParityCfg::Odd => serialport::Parity::Odd,
        })
        .stop_bits(match config.format.stop_bits {
            2 => serialport::StopBits::Two,
            _ => serialport::StopBits::One,
        })
        .timeout(Duration::from_millis(100))
}
