//! Tipe error terpusat untuk UARTRecon core.

use thiserror::Error;

/// Error yang mungkin dikembalikan oleh operasi UARTRecon.
#[derive(Debug, Error)]
pub enum Error {
    /// Error dari layer serialport (open/configure/IO).
    #[error("serial port error: {0}")]
    Serial(#[from] serialport::Error),

    /// Error I/O umum (file, stdin/stdout, dsb.).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Error serialisasi/deserialisasi JSON.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Error parsing TOML.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// Error serialisasi TOML.
    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// Port serial yang diminta tidak ditemukan.
    #[error("port '{0}' tidak ditemukan")]
    PortNotFound(String),

    /// Tidak ada traffic yang dapat diandalkan pada kandidat yang diuji.
    #[error("tidak ada traffic UART yang dapat diandalkan; konfigurasi tidak dapat ditentukan")]
    NoReliableTraffic,

    /// Operasi diblokir karena bersifat destruktif (read-only first).
    #[error("operasi diblokir (destructive): {0}")]
    BlockedOperation(String),

    /// Konfigurasi tidak valid.
    #[error("konfigurasi tidak valid: {0}")]
    InvalidConfig(String),

    /// Error lain-lain dengan pesan bebas.
    #[error("{0}")]
    Other(String),
}

/// Alias `Result` khusus UARTRecon.
pub type Result<T> = std::result::Result<T, Error>;
