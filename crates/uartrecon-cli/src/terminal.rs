//! Terminal interaktif real-time via UART.
//!
//! Menampilkan output device secara live dan meneruskan setiap tombol yang
//! ditekan - seperti Tera Term / PuTTY, tapi di dalam UARTRecon.
//!
//! Tekan `Ctrl+]` untuk keluar (seperti Telnet).
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
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal as ct;
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::serial::connection::Connection;

use crate::ui;

/// Membuka terminal interaktif ke device via UART.
///
/// Kalau `spam_secs > 0`, terminal akan spam Enter dulu selama itu (untuk
/// menghentikan autoboot U-Boot), baru masuk mode interaktif.
pub fn terminal(
    port: &str,
    baud: u32,
    format: &str,
    enter_mode: &str,
    spam_secs: u64,
    log_path: Option<String>,
    _color: bool,
) -> Result<()> {
    let fmt = SerialFormat::parse(format).context("format tidak valid")?;
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
    ui::kv("Keluar", "Ctrl+]");

    // Fase opsional: spam Enter untuk hentikan autoboot.
    if spam_secs > 0 {
        println!("\n[*] Spam ENTER {spam_secs} detik (hentikan autoboot)...");
        println!("    Kalau baru colok power, lakukan sekarang.\n");
        let deadline = Instant::now() + Duration::from_secs(spam_secs);
        while Instant::now() < deadline {
            let _ = conn.write(&enter_bytes);
            if let Ok(data) = conn.read_for(Duration::from_millis(50))
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

    // Worker thread: miliki Connection, tangani read + write.
    std::thread::spawn(move || {
        worker(conn, rx_to_dev, tx_from_dev);
    });

    let mut log_file = match &log_path {
        Some(p) => Some(std::fs::File::create(p).context("gagal membuat file log")?),
        None => None,
    };

    let raw = enable_raw();
    let result = run_loop(&rx_from_dev, &tx_to_dev, &enter_bytes, &mut log_file);
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
    /// Auto-kirim command setelah prompt ketangkap (non-interaktif).
    pub send_cmd: Option<String>,
    /// Simpan output ke file.
    pub log_path: Option<String>,
}

/// Mode U-Boot: auto-spam Enter untuk menghentikan autoboot, lalu interaktif.
///
/// Cocok untuk device yang autoboot-nya cepat (bootdelay=0) sehingga sulit
/// ditangkap manual. Setelah prompt U-Boot terdeteksi, user bisa langsung
/// mengetik command (`norm`, `safe`, `printenv`, ...).
///
/// Kalau `send_cmd` diisi, command itu otomatis dikirim setelah prompt
/// terdeteksi (mode non-interaktif, cocok untuk skrip).
pub fn uboot(port: &str, baud: u32, format: &str, opts: UbootOptions) -> Result<()> {
    let UbootOptions {
        reboot,
        spam_secs,
        send_cmd,
        log_path,
    } = opts;
    let fmt = SerialFormat::parse(format).context("format tidak valid")?;
    let config = SerialConfig::new(baud, fmt);

    let mut conn = Connection::open(port, config).context("gagal membuka port")?;
    let _ = conn.clear_input();

    ui::header("U-BOOT MODE");
    ui::kv("Port", &format!("{port} @ {baud} {}", fmt.label()));
    if send_cmd.is_none() {
        ui::kv("Keluar", "Ctrl+]");
    }

    // Opsional: kirim reboot dulu (kalau sudah di shell device).
    if reboot {
        println!("\n[*] Mengirim 'reboot' ke device...");
        let _ = conn.write(b"\r\n");
        std::thread::sleep(Duration::from_millis(300));
        let _ = conn.write(b"reboot\r\n");
    }

    println!("\n[*] Spam ENTER maksimal {spam_secs} detik untuk menghentikan autoboot...");
    println!("    (kalau STB baru dinyalakan, colok power sekarang)\n");

    // Fase 1: spam Enter sampai prompt U-Boot terdeteksi atau timeout.
    let deadline = Instant::now() + Duration::from_secs(spam_secs);
    let mut detected = false;
    let mut spam_count: u64 = 0;

    while Instant::now() < deadline && !detected {
        // Kirim Enter.
        let _ = conn.write(b"\r\n");
        spam_count += 1;

        // Baca apa pun yang masuk.
        if let Ok(data) = conn.read_for(Duration::from_millis(60))
            && !data.is_empty()
        {
            let text = String::from_utf8_lossy(&data);
            print!("{text}");
            let _ = std::io::stdout().flush();
            // Deteksi prompt U-Boot (berbagai varian).
            if text.contains("STB-BOOT #")
                || text.contains("U-Boot #")
                || text.contains("=>")
                || (text.contains("#") && text.contains("U-Boot"))
            {
                detected = true;
            }
        }
    }

    println!();
    if detected {
        println!("[+] Prompt U-Boot terdeteksi (setelah {spam_count}x Enter).");
    } else {
        println!("[!] Prompt U-Boot belum terdeteksi (setelah {spam_count}x Enter).");
        println!("    Kalau belum masuk, colok power lalu jalankan ulang.");
    }

    // Fase 2a: mode non-interaktif — kirim command otomatis lalu selesai.
    if let Some(cmd) = send_cmd {
        println!("[*] Mengirim command: {cmd}");
        std::thread::sleep(Duration::from_millis(300));
        let _ = conn.write(format!("{cmd}\r\n").as_bytes());

        // Baca output selama beberapa detik.
        let read_until = Instant::now() + Duration::from_secs(8);
        let mut out = Vec::new();
        while Instant::now() < read_until {
            match conn.read_for(Duration::from_millis(200)) {
                Ok(data) if !data.is_empty() => out.extend_from_slice(&data),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        print!("{}", String::from_utf8_lossy(&out));
        if let Some(p) = &log_path {
            let _ = std::fs::write(p, &out);
            println!("\n[+] Output disimpan: {p}");
        }
        println!("\n[+] Selesai.");
        return Ok(());
    }

    println!("    Masuk mode interaktif. Contoh: `norm`, `safe`, `printenv`.\n");

    // Fase 2b: interaktif (sama seperti terminal biasa).
    let (tx_to_dev, rx_to_dev) = mpsc::channel::<Vec<u8>>();
    let (tx_from_dev, rx_from_dev) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        worker(conn, rx_to_dev, tx_from_dev);
    });

    let mut log_file = match &log_path {
        Some(p) => Some(std::fs::File::create(p).context("gagal membuat file log")?),
        None => None,
    };

    let raw = enable_raw();
    let result = run_loop(&rx_from_dev, &tx_to_dev, b"\r", &mut log_file);
    if raw {
        let _ = ct::disable_raw_mode();
    }

    println!("\n\n[+] U-Boot mode ditutup.");
    result
}

/// Worker: baca dari device & tulis ke device (memiliki Connection).
fn worker(mut conn: Connection, rx_to_dev: Receiver<Vec<u8>>, tx_from_dev: Sender<Vec<u8>>) {
    let mut buf = [0u8; 4096];
    loop {
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
///
/// Kalau stdin bukan terminal (mis. pipa / otomasi), pakai mode line-based:
/// baca baris dari stdin lalu kirim ke device.
fn run_loop(
    rx_from_dev: &Receiver<Vec<u8>>,
    tx_to_dev: &Sender<Vec<u8>>,
    enter_bytes: &[u8],
    log_file: &mut Option<std::fs::File>,
) -> Result<()> {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        run_loop_tty(rx_from_dev, tx_to_dev, enter_bytes, log_file)
    } else {
        run_loop_pipe(rx_from_dev, tx_to_dev, log_file)
    }
}

/// Mode TTY: baca tombol real-time (butuh raw mode).
fn run_loop_tty(
    rx_from_dev: &Receiver<Vec<u8>>,
    tx_to_dev: &Sender<Vec<u8>>,
    enter_bytes: &[u8],
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
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char(']') | KeyCode::Char('c'))
                    {
                        return Ok(());
                    }
                    if let Some(bytes) = key_to_bytes(key, enter_bytes) {
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
                // stdin habis; tetap tampilkan output sampai channel device tutup.
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Mengubah tombol menjadi byte untuk dikirim ke device.
fn key_to_bytes(key: KeyEvent, enter_bytes: &[u8]) -> Option<Vec<u8>> {
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
        KeyCode::Backspace => vec![0x08],
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
