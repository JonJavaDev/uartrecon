//! Definisi command-line interface UARTRecon.

use clap::{Parser, Subcommand, ValueEnum};

/// UARTRecon - toolkit reconnaissance UART read-only-first.
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

    /// Kebijakan keamanan: partisi kritis & command yang diblokir.
    Safety {
        /// Periksa satu command (apakah aman dikirim ke device).
        #[arg(long)]
        check: Option<String>,
        /// Mode keras: blokir tulis ke partisi Critical + High.
        #[arg(long, default_value_t = true)]
        hard: bool,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Backup partisi kritis dari device via UART (hexdump -> file + verifikasi MD5).
    Backup {
        /// Nama port (mis. COM3).
        #[arg(long)]
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115200)]
        baud: u32,
        /// Partisi yang dibackup (nama, mis. `boot`). Bisa diulang.
        #[arg(long = "part", value_delimiter = ',')]
        parts: Vec<String>,
        /// Backup SEMUA partisi kritis (boot/env/kernel/rootfs).
        #[arg(long)]
        critical: bool,
        /// Direktori output.
        #[arg(long, default_value = "backups")]
        out: String,
        /// Nama device (untuk manifest).
        #[arg(long, default_value = "device")]
        device: String,
    },

    /// Recovery: backup/restore partisi kritis via SD card (UART hanya kirim perintah).
    Recover {
        /// Subcommand: `plan` | `run` | `restore`.
        #[arg(default_value = "plan")]
        action: String,
        /// Nama port (mis. COM3) untuk `run`/`restore`.
        #[arg(long)]
        port: Option<String>,
        /// Baudrate.
        #[arg(long, default_value_t = 115200)]
        baud: u32,
        /// Nama device.
        #[arg(long, default_value = "B700V5S1")]
        device: String,
        /// Subdirektori backup di SD card.
        #[arg(long, default_value = "uartrecon_backup")]
        subdir: String,
        /// Tulis script ke direktori ini (untuk `plan`).
        #[arg(long)]
        out: Option<String>,
    },

    /// Verifikasi environment (serial support, port).
    Doctor,

    /// Ganti bahasa antarmuka (id / en).
    Lang {
        /// Kode bahasa: `id` (Indonesia) atau `en` (English). Kosong = tampilkan.
        #[arg(default_value = "")]
        code: String,
    },

    /// Masuk U-Boot: auto-spam Enter untuk hentikan autoboot, lalu interaktif.
    ///
    /// Cocok untuk device dengan autoboot cepat (bootdelay=0). Setelah prompt
    /// U-Boot ketangkap, langsung bisa ketik `norm`, `safe`, `printenv`, dll.
    Uboot {
        /// Nama port (mis. COM3).
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        /// Format (mis. 8N1).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Kirim `reboot` dulu (kalau sudah berada di shell device).
        #[arg(long)]
        reboot: bool,
        /// Auto-kirim command U-Boot setelah prompt ketangkap (non-interaktif).
        /// Contoh: --send "norm" atau --send "printenv"
        #[arg(long)]
        send: Option<String>,
        /// Lama spam Enter (detik).
        #[arg(long, default_value_t = 30)]
        seconds: u64,
        /// Jeda antar spam (milidetik).
        #[arg(long, default_value_t = 50)]
        spam_delay: u64,
        /// Tombol spam: enter | space | ctrl-c.
        #[arg(long, default_value = "enter")]
        spam_key: String,
        /// Jeda antar command saat --send (milidetik).
        #[arg(long, default_value_t = 300)]
        send_delay: u64,
        /// Simpan sesi ke file (raw).
        #[arg(long)]
        log: Option<String>,
    },

    /// Terminal interaktif real-time via UART (kirim & terima langsung).
    ///
    /// Berbeda dari `connect` (yang mengirim per-baris), `terminal` menampilkan
    /// output device secara live dan meneruskan setiap tombol yang Anda tekan.
    /// Cocok untuk masuk shell device (BusyBox/LEDE), lihat bootlog, atau
    /// berinteraksi seperti di Tera Term/PuTTY.
    ///
    /// Hotkey: Ctrl+] keluar, Ctrl+T timestamp, Ctrl+H bantuan.
    Terminal {
        /// Nama port (mis. COM3).
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        /// Format (mis. 8N1).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Kirim line-ending CR (default CR, cocok untuk BusyBox/Linux).
        #[arg(long, default_value = "cr")]
        enter: String,
        /// Spam tombol dulu N detik (hentikan autoboot U-Boot), lalu interaktif.
        /// Contoh: --spam 20
        #[arg(long, default_value_t = 0)]
        spam: u64,
        /// Jeda antar spam (milidetik).
        #[arg(long, default_value_t = 50)]
        spam_delay: u64,
        /// Tombol spam: enter | space | ctrl-c.
        #[arg(long, default_value = "enter")]
        spam_key: String,
        /// Deteksi baudrate otomatis sebelum connect.
        #[arg(long)]
        auto: bool,
        /// Tampilkan timestamp di log (bisa toggle dengan Ctrl+T).
        #[arg(long)]
        timestamp: bool,
        /// Mode backspace: bs (0x08) | del (0x7f).
        #[arg(long, default_value = "bs")]
        backspace: String,
        /// Simpan seluruh sesi ke file (raw).
        #[arg(long)]
        log: Option<String>,
    },

    /// Kelola preset koneksi device (simpan/muat/hapus).
    Preset {
        /// Aksi: `list` | `show` | `save` | `remove`.
        #[arg(default_value = "list")]
        action: String,
        /// Nama preset (untuk show/save/remove).
        name: Option<String>,
        /// Baudrate (untuk save).
        #[arg(long)]
        baud: Option<u32>,
        /// Format, mis. `8N1` (untuk save).
        #[arg(long)]
        format: Option<String>,
        /// Nama port (untuk save).
        #[arg(long)]
        port: Option<String>,
        /// Deskripsi (untuk save).
        #[arg(long)]
        description: Option<String>,
        /// Command U-Boot favorit (bisa diulang / dipisah koma).
        #[arg(long = "uboot", value_delimiter = ',')]
        uboot_commands: Vec<String>,
        /// Command shell favorit (bisa diulang / dipisah koma).
        #[arg(long = "cmd", value_delimiter = ',')]
        shell_commands: Vec<String>,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Kelola macro command (rangkaian command tersimpan).
    Macro {
        /// Aksi: `list` | `show` | `save` | `remove` | `run`.
        #[arg(default_value = "list")]
        action: String,
        /// Nama macro (untuk show/save/remove/run).
        name: Option<String>,
        /// Daftar command untuk save, dipisah `;`.
        #[arg(long)]
        commands: Option<String>,
        /// Deskripsi (untuk save).
        #[arg(long)]
        description: Option<String>,
        /// Jeda default antar command (milidetik).
        #[arg(long, default_value_t = 500)]
        wait_ms: u64,
        /// Port (untuk run).
        #[arg(long)]
        port: Option<String>,
        /// Baudrate (untuk run).
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        /// Format (untuk run).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Output JSON.
        #[arg(long)]
        json: bool,
    },

    /// Auto-login ke device via UART (tunggu prompt, kirim kredensial).
    ///
    /// Contoh: `uartrecon login COM3 --user root --password toor --shell`
    Login {
        /// Nama port (mis. COM3).
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        /// Format (mis. 8N1).
        #[arg(long, default_value = "8N1")]
        format: String,
        /// Username.
        #[arg(long)]
        user: Option<String>,
        /// Password.
        #[arg(long)]
        password: Option<String>,
        /// Ambil baud/format dari preset ini.
        #[arg(long)]
        preset: Option<String>,
        /// Timeout menunggu tiap prompt (detik).
        #[arg(long, default_value_t = 8)]
        timeout: u64,
        /// Buka terminal interaktif setelah login berhasil.
        #[arg(long)]
        shell: bool,
        /// Simpan seluruh sesi ke file (raw).
        #[arg(long)]
        log: Option<String>,
    },

    /// Flash LEDE/OpenWrt ke partisi rootfs STB (workflow otomatis).
    ///
    /// Mengikuti metode yang terbukti: boot ke slot lain, timpa rootfs dorman,
    /// copy OS, JANGAN buat /init (kernel fallback ke /sbin/init).
    FlashLede {
        /// Nama port (mis. COM3).
        #[arg(long)]
        port: String,
        /// Baudrate.
        #[arg(long, default_value_t = 115200)]
        baud: u32,
        /// File squashfs OS (di SD card device) atau path lokal untuk instruksi.
        #[arg(long)]
        image: String,
        /// Partisi target (mtd6 = norm, mtd9 = safe).
        #[arg(long, default_value_t = 6)]
        target_mtd: u32,
        /// Subcommand: `plan` (tampilkan langkah) | `check` (preflight) | `run` (jalankan).
        #[arg(default_value = "plan")]
        action: String,
    },
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
