//! Terminal interaktif real-time via UART.
//!
//! Menampilkan output device secara live dan meneruskan setiap tombol yang
//! ditekan - seperti Tera Term / PuTTY, tapi di dalam UARTRecon.
//!
//! ## Hotkey
//!
//! - `Ctrl+]` - keluar
//! - `Ctrl+T` - toggle timestamp
//! - `Ctrl+H` - tampilkan bantuan
//!
//! ## Arsitektur
//!
//! ```text
//! main thread (keyboard)  --tx_channel-->  worker thread (punya Connection)
//!         ^                                        |
//!         +-----------rx_channel-------------------+
//! ```
//!
//! Satu worker thread memiliki `Connection` dan melakukan read + write, agar
//! port tidak dipegang dua thread sekaligus.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal as ct;
use uartrecon_core::detector::{self, SerialSampleSource};
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::serial::connection::Connection;

use crate::ui;

/// Byte code untuk tombol Backspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackspaceMode {
    /// Kirim `0x08` (BS) - default BusyBox/Linux.
    Bs,
    /// Kirim `0x7F` (DEL) - beberapa shell butuh ini.
    Del,
}

impl BackspaceMode {
    /// Parse dari string.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "del" | "delete" | "0x7f" => BackspaceMode::Del,
            _ => BackspaceMode::Bs,
        }
    }

    fn byte(self) -> u8 {
        match self {
            BackspaceMode::Bs => 0x08,
            BackspaceMode::Del => 0x7F,
        }
    }
}

/// Opsi untuk mode terminal interaktif.
pub struct TerminalOptions {
    /// Line ending untuk tombol Enter (`cr` / `lf` / `crlf`).
    pub enter_mode: String,
    /// Spam Enter dulu N detik (0 = tidak).
    pub spam_secs: u64,
    /// Jeda antar spam (milidetik).
    pub spam_delay_ms: u64,
    /// Tombol yang di-spam saat menangkap U-Boot.
    pub spam_key: SpamKey,
    /// Deteksi baudrate otomatis sebelum connect.
    pub auto_baud: bool,
    /// Prefiks timestamp di log.
    pub timestamp: bool,
    /// Mode backspace.
    pub backspace: BackspaceMode,
    /// Simpan sesi ke file.
    pub log_path: Option<String>,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        TerminalOptions {
            enter_mode: "cr".to_string(),
            spam_secs: 0,
            spam_delay_ms: 50,
            spam_key: SpamKey::Enter,
            auto_baud: false,
            timestamp: false,
            backspace: BackspaceMode::Bs,
            log_path: None,
        }
    }
}

/// Tombol yang dipakai untuk spam saat menangkap autoboot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpamKey {
    /// Enter (CR).
    Enter,
    /// Spasi.
    Space,
    /// Ctrl+C (`0x03`) - beberapa bootloader pakai ini.
    CtrlC,
}

impl SpamKey {
    /// Parse dari string.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "space" | "spasi" | " " => SpamKey::Space,
            "ctrl-c" | "ctrlc" | "ctrl+c" => SpamKey::CtrlC,
            _ => SpamKey::Enter,
        }
    }

    fn byte(self) -> u8 {
        match self {
            SpamKey::Enter => b'\r',
            SpamKey::Space => b' ',
            SpamKey::CtrlC => 0x03,
        }
    }
}

/// Membuka terminal interaktif ke device via UART.
pub fn terminal(port: &str, baud: u32, format: &str, opts: TerminalOptions) -> Result<()> {
    let TerminalOptions {
        enter_mode,
        spam_secs,
        spam_delay_ms,
        spam_key,
        auto_baud,
        timestamp,
        backspace,
        log_path,
    } = opts;

    let fmt = SerialFormat::parse(format).context("format tidak valid")?;

    // Auto-detect baudrate bila diminta.
    let baud = if auto_baud {
        ui::header("AUTO DETECT");
        println!("[*] Memindai baudrate...");
        let mut source = SerialSampleSource::new(port);
        let result = detector::detect(&mut source, &detector::DetectOptions::default())?;
        match result.config {
            Some(cfg) => {
                println!(
                    "[+] Terdeteksi: {} (confidence {}/100)",
                    cfg.label(),
                    result.confidence
                );
                cfg.baudrate
            }
            None => {
                println!(
                    "[!] Tidak ada traffic terdeteksi, pakai baud {} default.",
                    baud
                );
                baud
            }
        }
    } else {
        baud
    };

    let config = SerialConfig::new(baud, fmt);
    let mut conn = Connection::open(port, config).context("gagal membuka port")?;
    let _ = conn.clear_input();

    // Line ending untuk tombol Enter.
    let enter_bytes: Vec<u8> = match enter_mode.to_ascii_lowercase().as_str() {
        "lf" | "nl" => b"\n".to_vec(),
        "crlf" | "crnl" => b"\r\n".to_vec(),
        _ => b"\r".to_vec(), // default CR (cocok BusyBox/Linux)
    };

    ui::header("UART TERMINAL");
    ui::kv("Port", &format!("{port} @ {baud} {}", fmt.label()));
    ui::kv("Keluar", "Ctrl+]  (Ctrl+T timestamp, Ctrl+H bantuan)");

    // Fase opsional: spam tombol untuk hentikan autoboot.
    if spam_secs > 0 {
        println!(
            "\n[*] Spam {:?} {} detik (hentikan autoboot, jeda {}ms)...",
            spam_key, spam_secs, spam_delay_ms
        );
        println!("    Kalau baru colok power, lakukan sekarang.\n");
        let deadline = Instant::now() + Duration::from_secs(spam_secs);
        while Instant::now() < deadline {
            let _ = conn.write(&[spam_key.byte()]);
            if let Ok(data) = conn.read_for(Duration::from_millis(spam_delay_ms))
                && !data.is_empty()
            {
                print!("{}", String::from_utf8_lossy(&data));
                let _ = std::io::stdout().flush();
            }
        }
        println!("\n[*] Masuk mode interaktif.\n");
    } else {
        println!("\n(mode interaktif - ketik langsung, output device tampil live)\n");
    }

    // Channel: main -> worker (perintah tulis), worker -> main (data baca).
    let (tx_to_dev, rx_to_dev) = mpsc::channel::<Vec<u8>>();
    let (tx_from_dev, rx_from_dev) = mpsc::channel::<Vec<u8>>();
    let stop = Arc::new(AtomicBool::new(false));

    // Worker thread: miliki Connection, tangani read + write.
    let stop_w = stop.clone();
    std::thread::spawn(move || {
        worker(conn, rx_to_dev, tx_from_dev, stop_w);
    });

    let mut log_file = match &log_path {
        Some(p) => Some(std::fs::File::create(p).context("gagal membuat file log")?),
        None => None,
    };

    let raw = enable_raw();
    let result = run_loop(
        &rx_from_dev,
        &tx_to_dev,
        &enter_bytes,
        backspace,
        timestamp,
        &mut log_file,
    );
    stop.store(true, Ordering::Relaxed);
    if raw {
        let _ = ct::disable_raw_mode();
    }

    println!("\n\n[+] Terminal ditutup.");
    if let Some(p) = &log_path {
        println!("    Log tersimpan: {p}");
    }
    result
}

/// Aktifkan raw mode kalau stdout adalah terminal asli. Mengembalikan apakah
/// berhasil (kalau bukan TTY, kita pakai mode line-based).
fn enable_raw() -> bool {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return false;
    }
    ct::enable_raw_mode().is_ok()
}

/// Opsi untuk mode U-Boot.
pub struct UbootOptions {
    /// Kirim `reboot` dulu.
    pub reboot: bool,
    /// Lama spam Enter (detik).
    pub spam_secs: u64,
    /// Jeda antar spam (milidetik).
    pub spam_delay_ms: u64,
    /// Tombol yang di-spam.
    pub spam_key: SpamKey,
    /// Auto-kirim command setelah prompt ketangkap (non-interaktif).
    pub send_cmd: Option<String>,
    /// Jeda antar command (milidetik).
    pub send_delay_ms: u64,
    /// Simpan output ke file.
    pub log_path: Option<String>,
}

impl Default for UbootOptions {
    fn default() -> Self {
        UbootOptions {
            reboot: false,
            spam_secs: 30,
            spam_delay_ms: 50,
            spam_key: SpamKey::Enter,
            send_cmd: None,
            send_delay_ms: 300,
            log_path: None,
        }
    }
}

/// Deteksi prompt U-Boot pada output (setelah strip ANSI).
fn is_uboot_prompt(text: &str) -> bool {
    let clean = strip_ansi(text);
    clean.contains("STB-BOOT #")
        || clean.contains("U-Boot #")
        || clean.contains("U-Boot>")
        || clean.contains("uboot>")
        || clean.contains("=> ")
        || (clean.contains("U-Boot") && clean.contains("#"))
}

/// Buang escape sequence ANSI dari teks.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ESC [ ... (sampai huruf akhir).
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Mode U-Boot: auto-spam Enter untuk menghentikan autoboot, lalu interaktif.
pub fn uboot(port: &str, baud: u32, format: &str, opts: UbootOptions) -> Result<()> {
    let UbootOptions {
        reboot,
        spam_secs,
        spam_delay_ms,
        spam_key,
        send_cmd,
        send_delay_ms,
        log_path,
    } = opts;
    let fmt = SerialFormat::parse(format).context("format tidak valid")?;
    let config = SerialConfig::new(baud, fmt);

    let mut conn = Connection::open(port, config).context("gagal membuka port")?;
    let _ = conn.clear_input();

    ui::header("U-BOOT MODE");
    ui::kv("Port", &format!("{port} @ {baud} {}", fmt.label()));
    if send_cmd.is_none() {
        ui::kv("Keluar", "Ctrl+]  (Ctrl+T timestamp, Ctrl+H bantuan)");
    }

    // Opsional: kirim reboot dulu (kalau sudah di shell device).
    if reboot {
        println!("\n[*] Mengirim 'reboot' ke device...");
        let _ = conn.write(b"\r\n");
        std::thread::sleep(Duration::from_millis(300));
        let _ = conn.write(b"reboot\r\n");
    }

    println!(
        "\n[*] Spam {:?} maksimal {spam_secs} detik untuk menghentikan autoboot...",
        spam_key
    );
    println!("    (kalau STB baru dinyalakan, colok power sekarang)\n");

    // Fase 1: spam sampai prompt U-Boot terdeteksi atau timeout.
    let deadline = Instant::now() + Duration::from_secs(spam_secs);
    let mut detected = false;
    let mut spam_count: u64 = 0;
    let mut buf_acc = String::new();

    while Instant::now() < deadline && !detected {
        let _ = conn.write(&[spam_key.byte()]);
        spam_count += 1;

        if let Ok(data) = conn.read_for(Duration::from_millis(spam_delay_ms))
            && !data.is_empty()
        {
            let text = String::from_utf8_lossy(&data);
            print!("{text}");
            let _ = std::io::stdout().flush();
            buf_acc.push_str(&text);
            // Batasi buffer akumulasi.
            if buf_acc.len() > 8192 {
                let cut = buf_acc.len() - 4096;
                buf_acc = buf_acc[cut..].to_string();
            }
            if is_uboot_prompt(&buf_acc) {
                detected = true;
            }
        }
    }

    println!();
    if detected {
        println!(
            "[+] Prompt U-Boot terdeteksi (setelah {spam_count}x {:?}).",
            spam_key
        );
    } else {
        println!(
            "[!] Prompt U-Boot belum terdeteksi (setelah {spam_count}x {:?}).",
            spam_key
        );
        println!("    Kalau belum masuk, colok power lalu jalankan ulang.");
    }

    // Fase 2a: mode non-interaktif — kirim command otomatis lalu selesai.
    if let Some(cmd_str) = send_cmd {
        let cmds: Vec<&str> = cmd_str
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let mut all_out = Vec::new();
        for cmd in cmds {
            println!("[*] Mengirim: {cmd}");
            std::thread::sleep(Duration::from_millis(send_delay_ms));
            let _ = conn.write(format!("{cmd}\r\n").as_bytes());

            // Baca output sampai prompt U-Boot muncul lagi atau timeout.
            let read_until = Instant::now() + Duration::from_secs(8);
            let mut chunk = Vec::new();
            while Instant::now() < read_until {
                match conn.read_for(Duration::from_millis(200)) {
                    Ok(data) if !data.is_empty() => {
                        chunk.extend_from_slice(&data);
                        let s = String::from_utf8_lossy(&chunk);
                        if is_uboot_prompt(&s) && chunk.len() > 40 {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            print!("{}", String::from_utf8_lossy(&chunk));
            all_out.extend_from_slice(&chunk);
        }
        if let Some(p) = &log_path {
            let _ = std::fs::write(p, &all_out);
            println!("\n[+] Output disimpan: {p}");
        }
        println!("\n[+] Selesai.");
        return Ok(());
    }

    println!("    Masuk mode interaktif. Contoh: `norm`, `safe`, `printenv`.\n");

    // Fase 2b: interaktif.
    let (tx_to_dev, rx_to_dev) = mpsc::channel::<Vec<u8>>();
    let (tx_from_dev, rx_from_dev) = mpsc::channel::<Vec<u8>>();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_w = stop.clone();
    std::thread::spawn(move || {
        worker(conn, rx_to_dev, tx_from_dev, stop_w);
    });

    let mut log_file = match &log_path {
        Some(p) => Some(std::fs::File::create(p).context("gagal membuat file log")?),
        None => None,
    };

    let raw = enable_raw();
    let result = run_loop(
        &rx_from_dev,
        &tx_to_dev,
        b"\r",
        BackspaceMode::Bs,
        false,
        &mut log_file,
    );
    stop.store(true, Ordering::Relaxed);
    if raw {
        let _ = ct::disable_raw_mode();
    }

    println!("\n\n[+] U-Boot mode ditutup.");
    result
}

/// Worker: baca dari device & tulis ke device (memiliki Connection).
fn worker(
    mut conn: Connection,
    rx_to_dev: Receiver<Vec<u8>>,
    tx_from_dev: Sender<Vec<u8>>,
    stop: Arc<AtomicBool>,
) {
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        // 1. Baca dari device (timeout singkat).
        match conn.read(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                if tx_from_dev.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }

        // 2. Tangani perintah tulis.
        loop {
            match rx_to_dev.try_recv() {
                Ok(bytes) => {
                    if conn.write(&bytes).is_err() {
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Loop utama: tampilkan output device + baca keyboard.
fn run_loop(
    rx_from_dev: &Receiver<Vec<u8>>,
    tx_to_dev: &Sender<Vec<u8>>,
    enter_bytes: &[u8],
    backspace: BackspaceMode,
    timestamp: bool,
    log_file: &mut Option<std::fs::File>,
) -> Result<()> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        run_loop_tty(
            rx_from_dev,
            tx_to_dev,
            enter_bytes,
            backspace,
            timestamp,
            log_file,
        )
    } else {
        run_loop_pipe(rx_from_dev, tx_to_dev, log_file)
    }
}

/// Format timestamp untuk log.
fn ts() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d) = uartrecon_core::util::date_ymd(now);
    let secs = now % 86400;
    format!(
        "{y:04}-{mo:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Mode TTY: baca tombol real-time (butuh raw mode).
fn run_loop_tty(
    rx_from_dev: &Receiver<Vec<u8>>,
    tx_to_dev: &Sender<Vec<u8>>,
    enter_bytes: &[u8],
    backspace: BackspaceMode,
    mut timestamp: bool,
    log_file: &mut Option<std::fs::File>,
) -> Result<()> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    loop {
        // 1. Tampilkan output dari device.
        let mut got = false;
        loop {
            match rx_from_dev.try_recv() {
                Ok(data) => {
                    let _ = out.write_all(&data);
                    if let Some(f) = log_file.as_mut() {
                        if timestamp {
                            let _ = writeln!(f, "[{}]", ts());
                        }
                        let _ = f.write_all(&data);
                    }
                    got = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    let _ = out.flush();
                    return Ok(());
                }
            }
        }
        if got {
            let _ = out.flush();
        }

        // 2. Baca keyboard (non-blocking).
        if event::poll(Duration::from_millis(15))? {
            match event::read()? {
                Event::Key(key) => {
                    // Hanya proses tombol yang benar-benar ditekan.
                    // Tanpa filter ini, event Press + Release = input dobel.
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    // Hotkey kontrol.
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        match key.code {
                            KeyCode::Char(']') | KeyCode::Char('c') => return Ok(()),
                            KeyCode::Char('t') => {
                                timestamp = !timestamp;
                                let msg = if timestamp {
                                    "\r\n[timestamp ON]\r\n"
                                } else {
                                    "\r\n[timestamp OFF]\r\n"
                                };
                                let _ = out.write_all(msg.as_bytes());
                                let _ = out.flush();
                                continue;
                            }
                            KeyCode::Char('h') => {
                                let help = "\r\n[hotkey: Ctrl+] keluar | Ctrl+T timestamp | Ctrl+H bantuan]\r\n";
                                let _ = out.write_all(help.as_bytes());
                                let _ = out.flush();
                                continue;
                            }
                            _ => {}
                        }
                    }
                    if let Some(bytes) = key_to_bytes(key, enter_bytes, backspace) {
                        let _ = tx_to_dev.send(bytes);
                    }
                }
                Event::Paste(text) => {
                    let _ = tx_to_dev.send(text.into_bytes());
                }
                _ => {}
            }
        }
    }
}

/// Mode pipa: baca baris dari stdin (tanpa raw mode).
fn run_loop_pipe(
    rx_from_dev: &Receiver<Vec<u8>>,
    tx_to_dev: &Sender<Vec<u8>>,
    log_file: &mut Option<std::fs::File>,
) -> Result<()> {
    use std::io::{BufRead, BufReader};

    let (stdin_tx, stdin_rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let reader = BufReader::new(std::io::stdin());
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if stdin_tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    loop {
        // Tampilkan output device.
        let mut got = false;
        loop {
            match rx_from_dev.try_recv() {
                Ok(data) => {
                    let _ = out.write_all(&data);
                    if let Some(f) = log_file.as_mut() {
                        let _ = f.write_all(&data);
                    }
                    got = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    let _ = out.flush();
                    return Ok(());
                }
            }
        }
        if got {
            let _ = out.flush();
        }

        // Kirim baris dari stdin.
        match stdin_rx.try_recv() {
            Ok(line) => {
                if line == "exit" || line == "quit" {
                    // Beri jeda agar output terakhir sempat tertangkap.
                    let drain = Instant::now() + Duration::from_millis(500);
                    while Instant::now() < drain {
                        if let Ok(data) = rx_from_dev.try_recv() {
                            let _ = out.write_all(&data);
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    let _ = out.flush();
                    return Ok(());
                }
                let _ = tx_to_dev.send(format!("{line}\r\n").into_bytes());
            }
            Err(TryRecvError::Empty) => std::thread::sleep(Duration::from_millis(20)),
            Err(TryRecvError::Disconnected) => {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Mengubah tombol menjadi byte untuk dikirim ke device.
fn key_to_bytes(key: KeyEvent, enter_bytes: &[u8], backspace: BackspaceMode) -> Option<Vec<u8>> {
    Some(match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let lc = c.to_ascii_lowercase();
                if lc.is_ascii_lowercase() {
                    vec![(lc as u8) - b'a' + 1]
                } else {
                    c.to_string().into_bytes()
                }
            } else {
                c.to_string().into_bytes()
            }
        }
        KeyCode::Enter => enter_bytes.to_vec(),
        KeyCode::Backspace => vec![backspace.byte()],
        KeyCode::Tab => vec![0x09],
        KeyCode::Esc => vec![0x1B],
        KeyCode::Up => vec![0x1B, b'[', b'A'],
        KeyCode::Down => vec![0x1B, b'[', b'B'],
        KeyCode::Right => vec![0x1B, b'[', b'C'],
        KeyCode::Left => vec![0x1B, b'[', b'D'],
        KeyCode::Delete => vec![0x1B, b'[', b'3', b'~'],
        KeyCode::Home => vec![0x1B, b'[', b'H'],
        KeyCode::End => vec![0x1B, b'[', b'F'],
        KeyCode::PageUp => vec![0x1B, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1B, b'[', b'6', b'~'],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backspace_parse() {
        assert_eq!(BackspaceMode::parse("del"), BackspaceMode::Del);
        assert_eq!(BackspaceMode::parse("BS"), BackspaceMode::Bs);
        assert_eq!(BackspaceMode::parse(""), BackspaceMode::Bs);
    }

    #[test]
    fn spam_key_parse() {
        assert_eq!(SpamKey::parse("space"), SpamKey::Space);
        assert_eq!(SpamKey::parse("ctrl-c"), SpamKey::CtrlC);
        assert_eq!(SpamKey::parse("enter"), SpamKey::Enter);
        assert_eq!(SpamKey::parse("xyz"), SpamKey::Enter);
    }

    #[test]
    fn deteksi_prompt() {
        assert!(is_uboot_prompt("STB-BOOT # "));
        assert!(is_uboot_prompt("U-Boot #"));
        assert!(is_uboot_prompt("=> "));
        assert!(!is_uboot_prompt("root@LEDE:/# "));
    }

    #[test]
    fn strip_ansi_benar() {
        assert_eq!(strip_ansi("\x1b[32mOK\x1b[0m"), "OK");
        assert_eq!(strip_ansi("plain"), "plain");
    }
}
