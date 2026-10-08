//! Definisi command-line interface UARTRecon.

use clap::{Parser, Subcommand, ValueEnum};

/// UARTRecon — toolkit reconnaissance UART read-only-first.
#[derive(Debug, Parser)]
#[command(
    name = "uartrecon",
    version,
    about = "UART Recon & Analysis Toolkit (read-only first)",
    long_about = "Toolkit reconnaissance untuk perangkat embedded melalui UART.\n\
                  Fokus: detect, capture, analyze, export. Tidak ada operasi tulis/erase flash."
)]
pub struct Cli {
    /// Tingkat verbosity log (dapat diulang: -v, -vv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Nonaktifkan output warna.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Subcommand.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Format output yang didukung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Teks manusiawi.
    Txt,
    /// JSON.
    Json,
    /// Hex dump.
    Hex,
    /// CSV.
    Csv,
}

/// Subcommand UARTRecon.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Daftar serial port yang tersedia (USB-UART diidentifikasi).
    Ports {
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Scan baudrate & format pada sebuah port.
    Scan {
        /// Nama port (mis. COM7 atau /dev/ttyUSB0).
        port: String,
        /// Daftar baudrate kustom (dipisah koma). Default: prioritas embedded.
        #[arg(long, value_delimiter = ',')]
        baudrates: Option<Vec<u32>>,
        /// Durasi baca per kandidat (milidetik).
        #[arg(long, default_value_t = 700)]
        duration_ms: u64,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Buka koneksi UART dan masuk mode interaktif.
    Connect {
        /// Nama port.
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        /// Format (mis. 8N1).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Deteksi otomatis baudrate sebelum connect.
        #[arg(long)]
        auto: bool,
    },

    /// Monitor data dari port (streaming ke stdout).
    Monitor {
        /// Nama port.
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        /// Format (mis. 8N1).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Tampilkan sebagai hexdump.
        #[arg(long)]
        hex: bool,
        /// Simpan ke sesi dengan nama ini.
        #[arg(long)]
        session: Option<String>,
        /// Durasi monitor (detik). 0 = sampai Ctrl+C.
        #[arg(long, default_value_t = 0)]
        seconds: u64,
    },

    /// Analisis file capture yang sudah ada (raw byte).
    Analyze {
        /// Path file capture (raw).
        file: String,
        /// Format output.
        #[arg(long, value_enum, default_value_t = OutputFormat::Txt)]
        format: OutputFormat,
        /// Konfigurasi UART yang diasumsikan (untuk label laporan).
        #[arg(long, default_value = "115200 8N1")]
        config: String,
        /// Tampilkan hexdump juga.
        #[arg(long)]
        hex: bool,
    },

    /// Tampilkan laporan sesi yang tersimpan.
    Session {
        /// Direktori sesi (default: cari di ./sessions).
        #[arg(long)]
        dir: Option<String>,
        /// Nama sesi.
        name: Option<String>,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Logic analyzer: decode UART dari waveform digital (physical layer).
    Logic {
        /// Path file waveform (raw biner 0/1, atau CSV `index,level`).
        #[arg(long)]
        file: Option<String>,
        /// Laju sampel waveform (Hz). Default 921600 (8x 115200).
        #[arg(long, default_value_t = 921_600)]
        sample_rate: u32,
        /// Format UART untuk decoding (mis. 8N1).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Gunakan waveform demo sintetis (tanpa file).
        #[arg(long)]
        demo: bool,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Cari pola (literal/regex/hex) dalam file capture atau sesi.
    Search {
        /// Path file (raw) atau direktori sesi.
        #[arg(long)]
        file: String,
        /// Pola pencarian.
        pattern: String,
        /// Mode: literal | regex | hex | icase.
        #[arg(long, default_value = "literal")]
        mode: String,
        /// Konteks byte di kiri/kanan snippet.
        #[arg(long, default_value_t = 16)]
        context: usize,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Ekstrak string printable dari file/sesi (seperti `strings`).
    Strings {
        /// Path file (raw) atau direktori sesi.
        #[arg(long)]
        file: String,
        /// Panjang minimum string.
        #[arg(long, default_value_t = 4)]
        min_len: usize,
        /// Hanya tampilkan yang cocok pola menarik (password, url, path, ...).
        #[arg(long)]
        interesting: bool,
        /// Filter berdasarkan kata kunci.
        #[arg(long)]
        grep: Option<String>,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Analisis entropi (deteksi region compressed/encrypted).
    Entropy {
        /// Path file (raw) atau direktori sesi.
        #[arg(long)]
        file: String,
        /// Ukuran blok.
        #[arg(long, default_value_t = 1024)]
        block_size: usize,
        /// Ambang entropi untuk melaporkan region (bit/byte).
        #[arg(long, default_value_t = 7.0)]
        threshold: f64,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Scan signature firmware (binwalk-like: SquashFS, uImage, gzip, ...).
    Signatures {
        /// Path file (raw) atau direktori sesi.
        #[arg(long)]
        file: String,
        /// Tampilkan juga signature lemah (magic pendek).
        #[arg(long)]
        all: bool,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Statistik data: histogram byte & metrik ringkas.
    Stats {
        /// Path file (raw) atau direktori sesi.
        #[arg(long)]
        file: String,
        /// Tampilkan N byte teratas di histogram.
        #[arg(long, default_value_t = 16)]
        top: usize,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Bandingkan dua file capture / dua sesi.
    Diff {
        /// Path kiri (raw atau direktori sesi).
        #[arg(long)]
        left: String,
        /// Path kanan (raw atau direktori sesi).
        #[arg(long)]
        right: String,
        /// Mode teks (baris) alih-alih biner.
        #[arg(long)]
        text: bool,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Lihat/ubah konfigurasi persisten.
    Config {
        /// Tampilkan path file config.
        #[arg(long)]
        path: bool,
        /// Tulis config default ke disk.
        #[arg(long)]
        init: bool,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Verifikasi environment (serial support, port).
    Doctor,
}

/// Parsing string `"115200 8N1"` menjadi baudrate + format.
pub fn parse_config_str(s: &str) -> Result<(u32, String), String> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(format!(
            "format config tidak valid: '{s}' (contoh: 115200 8N1)"
        ));
    }
    let baud: u32 = parts[0]
        .parse()
        .map_err(|_| format!("baudrate tidak valid: '{}'", parts[0]))?;
    Ok((baud, parts[1].to_string()))
}
