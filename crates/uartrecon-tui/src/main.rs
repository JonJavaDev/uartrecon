//! UARTRecon TUI entry point.

mod app;
mod terminal;
mod widgets;

use std::io::Write;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use uartrecon_core::detector::SerialSampleSource;
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::serial::connection::Connection;

use app::App;
use terminal::Tui;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Mode demo: tanpa hardware, memutar data sintetis.
    if args.iter().any(|a| a == "--demo") {
        return run_demo();
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let port = args
        .first()
        .cloned()
        .context("penggunaan: uartrecon-tui <PORT> --baud 115200 [--auto]")?;
    let baud = parse_flag(&args, "--baud")
        .and_then(|v| v.parse().ok())
        .unwrap_or(115_200);
    let format = parse_flag(&args, "--format").unwrap_or_else(|| "8N1".to_string());
    let auto = args.iter().any(|a| a == "--auto");

    let fmt = SerialFormat::parse(&format).context("format tidak valid")?;
    let config = if auto {
        auto_detect(&port)?
    } else {
        SerialConfig::new(baud, fmt)
    };

    run_live(port, config)
}

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn auto_detect(port: &str) -> Result<SerialConfig> {
    eprintln!("[*] auto-detect baudrate pada {port}...");
    let mut source = SerialSampleSource::new(port);
    let result = uartrecon_core::detector::detect(
        &mut source,
        &uartrecon_core::detector::DetectOptions::default(),
    )?;
    match result.config {
        Some(cfg) => {
            eprintln!(
                "[+] terdeteksi: {} (confidence {}/100)",
                cfg.label(),
                result.confidence
            );
            Ok(cfg)
        }
        None => anyhow::bail!("tidak ada traffic yang dapat diandalkan"),
    }
}

/// Menjalankan TUI dengan koneksi serial nyata.
fn run_live(port: String, config: SerialConfig) -> Result<()> {
    let mut conn = Connection::open(&port, config).context("gagal membuka port")?;

    // Thread pembaca serial -> channel.
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        loop {
            match conn.read_for(Duration::from_millis(100)) {
                Ok(data) if !data.is_empty() => {
                    if tx.send(data).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    let mut app = App::new(port, config, 0);
    app.log("Terhubung ke port.");
    event_loop(&mut app, rx, None)
}

/// Menjalankan TUI demo tanpa hardware.
fn run_demo() -> Result<()> {
    let script = demo_script();
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        for chunk in script {
            if tx.send(chunk).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(600));
        }
    });

    let config = SerialConfig::new(115_200, SerialFormat::EIGHT_N_ONE);
    let mut app = App::new("DEMO", config, 97);
    app.log("Mode demo: memutar data boot sintetis.");
    event_loop(&mut app, rx, Some(Duration::from_secs(30)))
}

fn event_loop(app: &mut App, rx: Receiver<Vec<u8>>, max_duration: Option<Duration>) -> Result<()> {
    let mut tui = Tui::new()?;
    let start = Instant::now();

    let result = (|| -> Result<()> {
        loop {
            if app.should_quit {
                break;
            }
            if let Some(max) = max_duration
                && start.elapsed() >= max
            {
                break;
            }

            // Ambil data dari channel.
            loop {
                match rx.try_recv() {
                    Ok(data) => app.push_rx(&data),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        app.should_quit = true;
                        break;
                    }
                }
            }

            tui.terminal().draw(|frame| widgets::draw(frame, app))?;

            // Poll event dengan timeout singkat.
            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
                && handle_key(app, key)
            {
                app.should_quit = true;
            }
        }
        Ok(())
    })();

    Tui::restore()?;

    // Ringkasan setelah keluar.
    println!("\nUARTRecon TUI ditutup.");
    println!("  RX terekam: {} bytes", app.recorder.rx().len());
    if let Some(fp) = &app.fingerprint {
        print!("{}", fp.summary());
    }

    result
}

/// Menangani tombol. Mengembalikan `true` bila harus keluar.
fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // Ctrl+C keluar.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Tab => app.panel = app.panel.next(),
        KeyCode::BackTab => app.panel = app.panel.prev(),
        KeyCode::Char('h') => {
            app.mode = app.mode.next();
            app.log(format!("Mode: {}", app.mode.label()));
        }
        KeyCode::Char('f') => {
            app.update_fingerprint();
            app.log("Fingerprint diperbarui.");
        }
        KeyCode::Char('w') => {
            if app.waveform.is_none() {
                app.load_demo_waveform();
            }
            app.panel = crate::app::Panel::Waveform;
            app.log("Panel waveform aktif (logic analyzer).");
        }
        KeyCode::Char('c') => {
            app.buffer.clear();
            app.log("Buffer dibersihkan.");
        }
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
        KeyCode::PageUp => app.scroll_up(20),
        KeyCode::PageDown => app.scroll_down(20),
        _ => {}
    }
    false
}

fn demo_script() -> Vec<Vec<u8>> {
    vec![
        b"U-Boot 2021.10 (Jan 01 2021 - 00:00:00)\r\n".to_vec(),
        b"DRAM:  512 MiB\r\n".to_vec(),
        b"NAND:  256 MiB\r\n".to_vec(),
        b"Hit any key to stop autoboot:  0\r\n".to_vec(),
        b"Starting kernel ...\r\n".to_vec(),
        b"Linux version 5.10.0 (gcc version 9.3.0)\r\n".to_vec(),
        b"BusyBox v1.35.0 multi-call binary\r\n".to_vec(),
        b"login: root\r\n".to_vec(),
        b"root@stb:~# ".to_vec(),
    ]
}

fn print_help() {
    let mut out = std::io::stdout();
    let _ = writeln!(out, "uartrecon-tui — monitor interaktif\n");
    let _ = writeln!(
        out,
        "  uartrecon-tui <PORT> --baud 115200 [--format 8N1] [--auto]"
    );
    let _ = writeln!(out, "  uartrecon-tui --demo     # demo tanpa hardware\n");
    let _ = writeln!(
        out,
        "Tombol: Tab=ganti panel  h=mode  f=fingerprint  c=clear  j/k=scroll  q=keluar"
    );
}
