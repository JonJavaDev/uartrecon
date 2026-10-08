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
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal as ct;
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::serial::connection::Connection;

use crate::ui;

/// Membuka terminal interaktif ke device via UART.
pub fn terminal(
    port: &str,
    baud: u32,
    format: &str,
    enter_mode: &str,
    log_path: Option<String>,
    _color: bool,
) -> Result<()> {
    let fmt = SerialFormat::parse(format).context("format tidak valid")?;
    let config = SerialConfig::new(baud, fmt);

    let conn = Connection::open(port, config).context("gagal membuka port")?;

    // Line ending untuk tombol Enter.
    let enter_bytes: Vec<u8> = match enter_mode.to_ascii_lowercase().as_str() {
        "lf" | "nl" => b"\n".to_vec(),
        "crlf" | "crnl" => b"\r\n".to_vec(),
        _ => b"\r".to_vec(), // default CR (cocok BusyBox/Linux)
    };

    // Channel: main -> worker (perintah tulis), worker -> main (data baca).
    let (tx_to_dev, rx_to_dev) = mpsc::channel::<Vec<u8>>();
    let (tx_from_dev, rx_from_dev) = mpsc::channel::<Vec<u8>>();

    // Worker thread: miliki Connection, tangani read + write.
    std::thread::spawn(move || {
        worker(conn, rx_to_dev, tx_from_dev);
    });

    ui::header("UART TERMINAL");
    ui::kv("Port", &format!("{port} @ {baud} {}", fmt.label()));
    ui::kv("Keluar", "Ctrl+]");
    println!("\n(mode interaktif - ketik langsung, output device tampil live)\n");

    let mut log_file = match &log_path {
        Some(p) => Some(std::fs::File::create(p).context("gagal membuat file log")?),
        None => None,
    };

    ct::enable_raw_mode().context("gagal enable raw mode")?;
    let result = run_loop(&rx_from_dev, &tx_to_dev, &enter_bytes, &mut log_file);
    let _ = ct::disable_raw_mode();

    println!("\n\n[+] Terminal ditutup.");
    if let Some(p) = &log_path {
        println!("    Log tersimpan: {p}");
    }
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
fn run_loop(
    rx_from_dev: &Receiver<Vec<u8>>,
    tx_to_dev: &Sender<Vec<u8>>,
    enter_bytes: &[u8],
    log_file: &mut Option<std::fs::File>,
) -> Result<()> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Timeout koneksi serial di set 100ms di core, jadi poll keyboard cepat.
    let _ = Instant::now();

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
