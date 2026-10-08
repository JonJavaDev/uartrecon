//! Implementasi tiap subcommand CLI.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use uartrecon_core::analyzers::{
    diff as adiff, entropy, linux, mtd, signatures, stats, strings as astrings, uboot,
};
use uartrecon_core::capture::{Recorder, Session, search};
use uartrecon_core::config as core_config;
use uartrecon_core::detector::{self, DetectOptions, SerialSampleSource};
use uartrecon_core::output;
use uartrecon_core::serial::config::{SerialConfig, SerialFormat};
use uartrecon_core::serial::connection::Connection;
use uartrecon_core::serial::ports;

use crate::cli::OutputFormat;
use crate::ui;

/// Memuat byte dari path: file raw, atau direktori sesi (menggabungkan rx+tx).
fn load_bytes(path: &str) -> Result<Vec<u8>> {
    let p = Path::new(path);
    if p.is_dir() {
        // Direktori sesi: baca rx.raw dan tx.raw.
        let mut data = Vec::new();
        let rx = p.join("rx.raw");
        let tx = p.join("tx.raw");
        if rx.exists() {
            data.extend_from_slice(
                &std::fs::read(&rx).with_context(|| format!("gagal membaca {}", rx.display()))?,
            );
        }
        if tx.exists() {
            data.extend_from_slice(
                &std::fs::read(&tx).with_context(|| format!("gagal membaca {}", tx.display()))?,
            );
        }
        if data.is_empty() {
            bail!("sesi '{}' tidak berisi data (rx.raw/tx.raw kosong)", path);
        }
        Ok(data)
    } else {
        std::fs::read(p).with_context(|| format!("gagal membaca '{path}'"))
    }
}

/// Menampilkan bantuan singkat (dipakai di dalam menu).
#[allow(dead_code)]
pub fn print_help() {
    ui::header("Perintah CLI");
    println!("  uartrecon ports                          Daftar serial port");
    println!("  uartrecon scan <PORT> [--baudrates ...]  Scan baudrate & format");
    println!("  uartrecon connect <PORT> --baud 115200   Mode interaktif");
    println!("  uartrecon monitor <PORT> --baud 115200   Monitor data");
    println!("  uartrecon analyze <FILE>                 Analisis capture");
    println!("  uartrecon logic --demo | --file <FILE>   Logic analyzer (waveform)");
    println!("  uartrecon session [NAME]                 Lihat sesi tersimpan");
    println!("  uartrecon doctor                         Cek environment");
}

/// Menu interaktif: dipanggil saat `uartrecon` dijalankan tanpa argumen.
///
/// Alur: tampilkan daftar port → pilih → pilih aksi. Cukup `cargo run` saja.
pub fn interactive_menu(_color: bool) -> Result<()> {
    ui::banner();
    println!("(mode menu — tekan Ctrl+C kapan saja untuk keluar)\n");

    let stdin = std::io::stdin();

    loop {
        ui::header("MENU UTAMA");
        println!("  ── Koneksi & Capture ──");
        println!("  [1] Daftar serial port");
        println!("  [2] Scan baudrate & format (auto-detect)");
        println!("  [3] Monitor data");
        println!("  [4] Mode interaktif (kirim command)");
        println!("  [5] Analisis file capture / sesi");
        println!("  [6] Logic analyzer (waveform)");
        println!("  ── Analisis Lanjutan ──");
        println!("  [s] Search pola (literal/regex/hex)");
        println!("  [t] Strings (ekstraksi teks)");
        println!("  [e] Entropy (deteksi compressed/encrypted)");
        println!("  [g] Signatures (scan firmware binwalk-like)");
        println!("  [x] Stats (histogram byte)");
        println!("  [d] Diff (bandingkan dua capture)");
        println!("  ── Lainnya ──");
        println!("  [7] Lihat sesi tersimpan");
        println!("  [8] Doctor (cek environment)");
        println!("  [c] Konfigurasi");
        println!("  [q] Keluar");
        print!("\nPilih: ");
        std::io::stdout().flush()?;

        let mut choice = String::new();
        if stdin.read_line(&mut choice)? == 0 {
            break;
        }
        match choice.trim() {
            "1" => {
                ports(false, true)?;
            }
            "2" => {
                if let Some(port) = pick_port()? {
                    let dur = prompt("Durasi baca per kandidat (ms)", "700")?;
                    let duration_ms: u64 = dur.parse().unwrap_or(700);
                    scan(&port, None, duration_ms, false, true)?;
                }
            }
            "3" => {
                if let Some(port) = pick_port()? {
                    let (baud, fmt) = prompt_config()?;
                    let secs = prompt("Durasi monitor (detik, 0 = tak terbatas)", "0")?;
                    let seconds: u64 = secs.parse().unwrap_or(0);
                    monitor(&port, baud, &fmt, false, None, seconds, true)?;
                }
            }
            "4" => {
                if let Some(port) = pick_port()? {
                    let (baud, fmt) = prompt_config()?;
                    connect(&port, baud, &fmt, false, true)?;
                }
            }
            "5" => {
                let file = prompt("Path file capture / direktori sesi", "")?;
                if !file.trim().is_empty() {
                    let (baud, fmt) = prompt_config()?;
                    let config = format!("{baud} {fmt}");
                    analyze(&file, OutputFormat::Txt, &config, false, true)?;
                }
            }
            "6" => {
                logic(None, 921_600, "8N1", true, false, true)?;
            }
            "s" | "S" => {
                let file = prompt("Path file/sesi", "")?;
                if !file.trim().is_empty() {
                    let pattern = prompt("Pola", "")?;
                    let mode = prompt("Mode (literal/regex/hex/icase)", "literal")?;
                    search(&file, &pattern, &mode, 16, false, true)?;
                }
            }
            "t" | "T" => {
                let file = prompt("Path file/sesi", "")?;
                if !file.trim().is_empty() {
                    let min = prompt("Panjang minimum string", "4")?;
                    let min_len: usize = min.parse().unwrap_or(4);
                    let interesting = prompt("Hanya yang menarik? (y/N)", "n")?;
                    let int = interesting.eq_ignore_ascii_case("y");
                    strings(&file, min_len, int, None, false, true)?;
                }
            }
            "e" | "E" => {
                let file = prompt("Path file/sesi", "")?;
                if !file.trim().is_empty() {
                    entropy(&file, 1024, 7.0, false, true)?;
                }
            }
            "g" | "G" => {
                let file = prompt("Path file/sesi", "")?;
                if !file.trim().is_empty() {
                    signatures(&file, false, false, true)?;
                }
            }
            "x" | "X" => {
                let file = prompt("Path file/sesi", "")?;
                if !file.trim().is_empty() {
                    stats(&file, 16, false, true)?;
                }
            }
            "d" | "D" => {
                let left = prompt("Path kiri", "")?;
                let right = prompt("Path kanan", "")?;
                if !left.trim().is_empty() && !right.trim().is_empty() {
                    diff(&left, &right, false, false, true)?;
                }
            }
            "7" => {
                session(None, None, false, true)?;
            }
            "8" => {
                doctor(true)?;
            }
            "c" | "C" => {
                config(false, false, false, true)?;
            }
            "q" | "Q" | "exit" | "quit" => break,
            other => {
                ui::warn(&format!("pilihan '{other}' tidak dikenal"));
            }
        }
        println!();
    }
    println!("Sampai jumpa.");
    Ok(())
}

/// Meminta user memilih satu port dari daftar (atau ketik manual).
fn pick_port() -> Result<Option<String>> {
    let list = ports::list_ports().context("gagal enumerasi port")?;
    if list.is_empty() {
        ui::warn("tidak ada serial port terdeteksi");
        let manual = prompt("Ketik nama port manual (kosongkan untuk batal)", "")?;
        return Ok(if manual.trim().is_empty() {
            None
        } else {
            Some(manual.trim().to_string())
        });
    }

    ui::header("PILIH PORT");
    for (i, p) in list.iter().enumerate() {
        let chip = p
            .usb_uart_chip()
            .map(|c| format!(" [{c}]"))
            .unwrap_or_default();
        println!("  [{}] {}{}", i + 1, p.name, chip);
    }
    println!("  [0] batal / ketik manual");
    print!("\nPilih: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input == "0" {
        let manual = prompt("Nama port manual (kosongkan untuk batal)", "")?;
        return Ok(if manual.trim().is_empty() {
            None
        } else {
            Some(manual.trim().to_string())
        });
    }
    if let Ok(idx) = input.parse::<usize>()
        && idx >= 1
        && idx <= list.len()
    {
        return Ok(Some(list[idx - 1].name.clone()));
    }
    // Anggap input adalah nama port langsung.
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input.to_string()))
    }
}

/// Meminta baudrate & format.
fn prompt_config() -> Result<(u32, String)> {
    let baud_s = prompt("Baudrate", "115200")?;
    let baud: u32 = baud_s.parse().unwrap_or(115_200);
    let fmt = prompt("Format", "8N1")?;
    let fmt = if fmt.trim().is_empty() {
        "8N1".to_string()
    } else {
        fmt
    };
    Ok((baud, fmt))
}

/// Menampilkan prompt dan membaca input.
fn prompt(label: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{label}: ");
    } else {
        print!("{label} [{default}]: ");
    }
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

/// `logic`: logic analyzer dari waveform.
pub fn logic(
    file: Option<String>,
    sample_rate: u32,
    format: &str,
    demo: bool,
    json: bool,
    _color: bool,
) -> Result<()> {
    use uartrecon_core::logic as la;

    let fmt = SerialFormat::parse(format).context("format tidak valid")?;

    let wave = if demo || file.is_none() {
        if !json {
            ui::header("LOGIC ANALYZER (DEMO)");
            println!("[*] Membuat waveform sintetis 115200 8N1...");
        }
        let payload = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nLinux version 5.10\r\nBusyBox\r\nlogin: ";
        la::synth_uart(payload, 115_200, 8, SerialFormat::EIGHT_N_ONE)
    } else if let Some(path) = file {
        let data = std::fs::read(&path).with_context(|| format!("gagal membaca '{path}'"))?;
        // Deteksi CSV sederhana.
        let samples: Vec<u8> = if data.contains(&b',') || data.contains(&b'\n') {
            String::from_utf8_lossy(&data)
                .lines()
                .filter_map(|l| l.split(',').next_back())
                .filter_map(|s| s.trim().parse::<u8>().ok())
                .collect()
        } else {
            data.iter().map(|&b| b & 1).collect()
        };
        la::Waveform::new(sample_rate, samples)
    } else {
        unreachable!("file pasti ada karena cabang is_none sudah ditangani");
    };

    let analysis = la::analyze(&wave, Some(fmt)).context("analisis waveform gagal")?;

    if json {
        let report = serde_json::json!({
            "sample_rate": wave.sample_rate,
            "samples": wave.samples.len(),
            "duration_ms": wave.duration_ms(),
            "format": fmt.label(),
            "analysis": analysis,
        });
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    ui::header("WAVEFORM");
    ui::kv("Sample rate", &format!("{} Hz", wave.sample_rate));
    ui::kv("Samples", &wave.samples.len().to_string());
    ui::kv("Duration", &format!("{:.2} ms", wave.duration_ms()));

    ui::header("ANALYSIS");
    ui::kv("Edges", &analysis.edge_count.to_string());

    if let Some(p) = &analysis.pulse {
        ui::kv("Pulses", &p.count.to_string());
        ui::kv(
            "Width min/max",
            &format!("{:.2} / {:.2} µs", p.min_secs * 1e6, p.max_secs * 1e6),
        );
        ui::kv("Width median", &format!("{:.2} µs", p.median_secs * 1e6));
    }

    if let Some(b) = &analysis.baud {
        ui::kv("Bit period", &format!("{:.3} µs", b.bit_period_secs * 1e6));
        println!(
            "  {:<12} : {} (raw {:.0} baud, error {:.2}%)",
            "Baud (fisik)",
            b.baudrate,
            b.raw_baud,
            b.error_ratio * 100.0
        );
    }

    if let Some(d) = &analysis.decoded {
        ui::header("DECODED");
        ui::kv("Format", &fmt.label());
        ui::kv(
            "Bytes",
            &format!("{} ({} error)", d.bytes.len(), d.error_count),
        );
        println!(
            "\n  \"{}\"",
            d.text().replace('\r', "\\r").replace('\n', "\\n")
        );
    }

    Ok(())
}

/// `doctor`: cek environment.
pub fn doctor(_color: bool) -> Result<()> {
    ui::header("UARTRecon DOCTOR");
    println!("  Version       : {}", env!("CARGO_PKG_VERSION"));
    println!("  OS            : {}", std::env::consts::OS);
    println!("  Arch          : {}", std::env::consts::ARCH);

    match ports::list_ports() {
        Ok(list) => {
            println!("  Serial support: OK ({} port terdeteksi)", list.len());
            for p in &list {
                let chip = p.usb_uart_chip().unwrap_or("n/a");
                println!("    - {} [{}]", p.name, chip);
            }
        }
        Err(e) => println!("  Serial support: GAGAL ({e})"),
    }
    Ok(())
}

/// `ports`: daftar serial port.
pub fn ports(json: bool, _color: bool) -> Result<()> {
    let list = ports::list_ports().context("gagal enumerasi serial port")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&list)?);
        return Ok(());
    }

    if list.is_empty() {
        ui::warn("tidak ada serial port terdeteksi");
        return Ok(());
    }

    ui::header("UART DEVICES");
    for p in &list {
        println!("\n{}", p.name);
        let kind = match p.kind {
            uartrecon_core::PortKind::Usb => "USB",
            uartrecon_core::PortKind::Hardware => "Hardware",
            uartrecon_core::PortKind::Bluetooth => "Bluetooth",
            uartrecon_core::PortKind::Unknown => "Unknown",
        };
        ui::kv("Type", kind);
        if let Some(chip) = p.usb_uart_chip() {
            ui::kv("Chip", chip);
        }
        if let Some(m) = &p.manufacturer {
            ui::kv("Manufacturer", m);
        }
        if let Some(prod) = &p.product {
            ui::kv("Device", prod);
        }
        if let Some(vp) = p.vid_pid() {
            ui::kv("VID:PID", &vp);
        }
        if let Some(sn) = &p.serial_number {
            ui::kv("Serial", sn);
        }
    }
    println!();
    Ok(())
}

/// `scan`: deteksi baudrate & format.
pub fn scan(
    port: &str,
    baudrates: Option<Vec<u32>>,
    duration_ms: u64,
    json: bool,
    _color: bool,
) -> Result<()> {
    // Pastikan port ada.
    let info = ports::find_port(port).with_context(|| format!("port '{port}' tidak ditemukan"))?;

    let mut opts = DetectOptions::default();
    opts.baud.duration = Duration::from_millis(duration_ms);
    opts.format_duration = Duration::from_millis(duration_ms.min(500));
    if let Some(list) = baudrates {
        opts.baud.baudrates = list;
    }

    if !json {
        ui::header("UART SCAN");
        ui::kv("Port", &info.name);
        if let Some(chip) = info.usb_uart_chip() {
            ui::kv("Chip", chip);
        }
        println!(
            "\n[*] Memindai {} kandidat baudrate...",
            opts.baud.baudrates.len()
        );
    }

    let mut source = SerialSampleSource::new(port);
    let result = detector::detect(&mut source, &opts).context("deteksi gagal")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("\nRanking baudrate:");
    for c in &result.baud_ranking {
        let marker = if Some(c.baudrate) == result.config.map(|cfg| cfg.baudrate) {
            " <--"
        } else {
            ""
        };
        println!(
            "  {:<9} {:>3.0}  ({} bytes){}",
            c.baudrate,
            c.score(),
            c.bytes_read,
            marker
        );
    }

    match result.config {
        Some(cfg) => {
            println!();
            println!("Detected: {}", cfg.label());
            println!("Confidence score: {}/100", result.confidence);
            if result.confidence < 50 {
                ui::warn("confidence rendah; hasil mungkin tidak akurat");
            }
        }
        None => {
            println!();
            println!("No reliable traffic detected.");
            println!("Automatic baud detection cannot determine a configuration.");
        }
    }
    Ok(())
}

/// `connect`: mode interaktif sederhana (read + risk-checked send).
pub fn connect(port: &str, baud: u32, format: &str, auto: bool, _color: bool) -> Result<()> {
    let fmt = SerialFormat::parse(format).context("format tidak valid")?;

    let config = if auto {
        ui::header("AUTO DETECT");
        println!("[*] Memindai baudrate...");
        let mut source = SerialSampleSource::new(port);
        let result = detector::detect(&mut source, &DetectOptions::default())?;
        match result.config {
            Some(cfg) => {
                println!(
                    "[+] Terdeteksi: {} (confidence {}/100)",
                    cfg.label(),
                    result.confidence
                );
                cfg
            }
            None => {
                bail!("tidak ada traffic yang dapat diandalkan; coba --baud manual");
            }
        }
    } else {
        SerialConfig::new(baud, fmt)
    };

    let mut conn = Connection::open(port, config).context("gagal membuka port")?;
    ui::header("CONNECTED");
    ui::kv("Port", port);
    ui::kv("Config", &config.label());
    println!("\nKetik perintah untuk dikirim (read-only). 'exit' atau Ctrl+C untuk keluar.");
    println!("Perintah destruktif akan diblokir otomatis.\n");

    let stdin = std::io::stdin();
    loop {
        print!("> ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break; // EOF
        }
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }
        if cmd == "exit" || cmd == "quit" {
            break;
        }

        // Risk check read-only-first.
        match uartrecon_core::terminal::classify_risk(cmd) {
            uartrecon_core::terminal::CommandRisk::Destructive => {
                ui::error("perintah diblokir (destructive). UARTRecon bersifat read-only first.");
                continue;
            }
            uartrecon_core::terminal::CommandRisk::StateChanging => {
                print!("perintah ini mengubah state. Lanjutkan? [y/N] ");
                std::io::stdout().flush()?;
                let mut ans = String::new();
                stdin.read_line(&mut ans)?;
                if !ans.trim().eq_ignore_ascii_case("y") {
                    println!("dibatalkan.");
                    continue;
                }
            }
            uartrecon_core::terminal::CommandRisk::ReadOnly => {}
        }

        let payload = format!("{cmd}\r\n");
        conn.write(payload.as_bytes())?;

        // Baca balasan sebentar.
        let resp = conn.read_for(Duration::from_millis(400))?;
        if !resp.is_empty() {
            print!("{}", String::from_utf8_lossy(&resp));
        }
    }

    conn.close()?;
    println!("\nKoneksi ditutup.");
    Ok(())
}

/// `monitor`: streaming data ke stdout (opsional disimpan sebagai sesi).
pub fn monitor(
    port: &str,
    baud: u32,
    format: &str,
    hex: bool,
    session: Option<String>,
    seconds: u64,
    _color: bool,
) -> Result<()> {
    let fmt = SerialFormat::parse(format).context("format tidak valid")?;
    let config = SerialConfig::new(baud, fmt);

    let mut conn = Connection::open(port, config).context("gagal membuka port")?;
    ui::header("UART MONITOR");
    ui::kv("Port", port);
    ui::kv("Config", &config.label());
    ui::kv("Mode", if hex { "HEX" } else { "ASCII" });
    println!("\nTekan Ctrl+C untuk berhenti.\n");

    let mut recorder = Recorder::new();
    let deadline = if seconds > 0 {
        Some(std::time::Instant::now() + Duration::from_secs(seconds))
    } else {
        None
    };

    loop {
        if let Some(d) = deadline
            && std::time::Instant::now() >= d
        {
            break;
        }
        let data = conn.read_for(Duration::from_millis(200))?;
        if data.is_empty() {
            continue;
        }
        recorder.push_rx(&data);
        if hex {
            print!("{}", output::hexdump(&data));
        } else {
            print!("{}", String::from_utf8_lossy(&data));
        }
        std::io::stdout().flush()?;
    }

    if let Some(name) = session {
        let mut s = Session::create("sessions", name, port, config)?;
        s.save(&recorder)?;
        println!("\n[+] Sesi disimpan di {}", s.paths.dir.display());
    }
    Ok(())
}

/// `analyze`: analisis file capture.
pub fn analyze(
    file: &str,
    format: OutputFormat,
    config_str: &str,
    hex: bool,
    _color: bool,
) -> Result<()> {
    let data = std::fs::read(file).with_context(|| format!("gagal membaca '{file}'"))?;

    let (baud, fmt_str) =
        crate::cli::parse_config_str(config_str).map_err(|e| anyhow::anyhow!(e))?;
    let fmt = SerialFormat::parse(&fmt_str).context("format tidak valid")?;
    let config = SerialConfig::new(baud, fmt);

    let fp = detector::fingerprint::fingerprint(&data);
    let uboot_a = uboot::analyze(&data);
    let linux_a = linux::analyze(&data);
    let partitions = mtd::parse_any(&data);
    let sha = uartrecon_core::util::hash_sha256(&data);

    match format {
        OutputFormat::Txt => {
            let report = output::render_report(
                config,
                &fp,
                &uboot_a,
                &linux_a,
                Some(&partitions),
                Some(&sha),
            );
            print!("{report}");

            if linux_a.detected {
                println!("\nSUGGESTED READ-ONLY COMMANDS");
                for (i, (cmd, desc)) in linux::suggestions(&linux_a).iter().enumerate() {
                    println!("  [{}] {:<24} {}", i + 1, cmd, desc);
                }
            }

            if hex {
                println!("\nHEXDUMP (first 512 bytes)");
                let n = data.len().min(512);
                print!("{}", output::hexdump(&data[..n]));
            }
        }
        OutputFormat::Json => {
            let report = serde_json::json!({
                "config": config,
                "fingerprint": fp,
                "uboot": uboot_a,
                "linux": linux_a,
                "partitions": partitions,
                "sha256": sha,
                "size_bytes": data.len(),
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Hex => {
            print!("{}", output::hexdump(&data));
        }
        OutputFormat::Csv => {
            print!("{}", uartrecon_core::capture::exporter::to_csv(&data));
        }
    }
    Ok(())
}

/// `session`: lihat sesi tersimpan.
pub fn session(dir: Option<String>, name: Option<String>, json: bool, _color: bool) -> Result<()> {
    let root = dir.unwrap_or_else(|| "sessions".to_string());

    let session_dir = if let Some(n) = name {
        std::path::PathBuf::from(&root).join(n)
    } else {
        // Ambil sesi terbaru.
        let mut entries: Vec<_> = std::fs::read_dir(&root)
            .with_context(|| format!("direktori sesi '{root}' tidak ditemukan"))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        match entries.last() {
            Some(e) => e.path(),
            None => bail!("tidak ada sesi di '{root}'"),
        }
    };

    let s = Session::open(&session_dir).context("gagal membuka sesi")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&s.metadata)?);
        return Ok(());
    }

    ui::header("SESSION");
    ui::kv("Name", &s.metadata.session);
    ui::kv("Port", &s.metadata.port);
    ui::kv(
        "Config",
        &format!(
            "{} {}{}{}",
            s.metadata.baudrate, s.metadata.data_bits, s.metadata.parity, s.metadata.stop_bits
        ),
    );
    ui::kv("Timestamp", &s.metadata.timestamp_utc);
    if let Some(h) = &s.metadata.rx_sha256 {
        ui::kv("RX SHA256", &h[..h.len().min(16)]);
    }

    // Analisis ulang dari RX tersimpan (tanpa capture ulang).
    if let Ok(rx) = s.read_rx()
        && !rx.is_empty()
    {
        let fp = detector::fingerprint::fingerprint(&rx);
        println!("\n{}", fp.summary());
    }
    Ok(())
}

/// `search`: cari pola dalam capture.
pub fn search(
    file: &str,
    pattern: &str,
    mode: &str,
    context: usize,
    json: bool,
    _color: bool,
) -> Result<()> {
    let data = load_bytes(file)?;
    let mode = match mode.to_ascii_lowercase().as_str() {
        "literal" => search::SearchMode::Literal,
        "icase" | "insensitive" => search::SearchMode::LiteralInsensitive,
        "regex" => search::SearchMode::Regex,
        "hex" => search::SearchMode::Hex,
        other => bail!("mode '{other}' tidak dikenal (literal|regex|hex|icase)"),
    };
    let opts = search::SearchOptions {
        mode,
        context,
        max_results: 1000,
    };
    let matches = search::search(&data, pattern, &opts)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&matches)?);
        return Ok(());
    }

    ui::header("SEARCH");
    ui::kv("File", file);
    ui::kv("Pattern", pattern);
    ui::kv("Mode", &format!("{mode:?}"));
    ui::kv("Matches", &matches.len().to_string());
    println!();
    for m in &matches {
        println!("  0x{:08X}  {}", m.offset, m.snippet);
    }
    Ok(())
}

/// `strings`: ekstraksi string printable.
pub fn strings(
    file: &str,
    min_len: usize,
    interesting: bool,
    grep: Option<String>,
    json: bool,
    _color: bool,
) -> Result<()> {
    let data = load_bytes(file)?;
    let extracted = astrings::extract_ascii(&data, min_len);

    if json {
        println!("{}", serde_json::to_string_pretty(&extracted)?);
        return Ok(());
    }

    ui::header("STRINGS");
    ui::kv("File", file);
    ui::kv("Min length", &min_len.to_string());
    ui::kv("Total strings", &extracted.len().to_string());
    println!();

    if interesting {
        let found = astrings::find_interesting(&extracted);
        println!("Interesting ({}):", found.len());
        for (s, desc) in &found {
            println!("  0x{:08X}  [{}] {}", s.offset, desc, s.value);
        }
    } else if let Some(kw) = &grep {
        let found = astrings::filter_keyword(&extracted, kw);
        for s in found {
            println!("  0x{:08X}  {}", s.offset, s.value);
        }
    } else {
        for s in &extracted {
            println!("  0x{:08X}  {}", s.offset, s.value);
        }
    }
    Ok(())
}

/// `entropy`: analisis entropi.
pub fn entropy(
    file: &str,
    block_size: usize,
    threshold: f64,
    json: bool,
    _color: bool,
) -> Result<()> {
    let data = load_bytes(file)?;
    let report = entropy::analyze(&data, block_size);
    let regions = entropy::high_entropy_regions(&data, block_size, threshold);

    if json {
        let out = serde_json::json!({
            "report": report,
            "high_entropy_regions": regions,
            "threshold": threshold,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    ui::header("ENTROPY");
    ui::kv("File", file);
    ui::kv("Size", &format!("{} bytes", data.len()));
    ui::kv("Overall", &format!("{:.4} bit/byte", report.overall));
    ui::kv("Class", report.class.label());
    ui::kv("Blocks", &report.blocks.len().to_string());

    if !regions.is_empty() {
        println!("\nHigh-entropy regions (>= {threshold:.1} bit/byte):");
        for (offset, len) in &regions {
            println!("  0x{offset:08X}  {len} bytes");
        }
    }
    Ok(())
}

/// `signatures`: scan signature firmware.
pub fn signatures(file: &str, all: bool, json: bool, _color: bool) -> Result<()> {
    let data = load_bytes(file)?;
    let sigs = if all {
        signatures::scan(&data)
    } else {
        signatures::scan_strong(&data)
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&sigs)?);
        return Ok(());
    }

    ui::header("SIGNATURES");
    ui::kv("File", file);
    ui::kv("Size", &format!("{} bytes", data.len()));
    ui::kv("Found", &sigs.len().to_string());
    println!();
    for s in &sigs {
        let weak = if s.weak { " (weak)" } else { "" };
        println!(
            "  0x{:08X}  [{:<11}] {}{}",
            s.offset, s.category, s.name, weak
        );
    }
    Ok(())
}

/// `stats`: statistik data.
pub fn stats(file: &str, top: usize, json: bool, _color: bool) -> Result<()> {
    let data = load_bytes(file)?;
    let s = stats::compute(&data);

    if json {
        println!("{}", serde_json::to_string_pretty(&s)?);
        return Ok(());
    }

    ui::header("STATS");
    ui::kv("File", file);
    ui::kv("Total bytes", &s.total.to_string());
    ui::kv("Unique bytes", &s.unique_bytes.to_string());
    if let Some((b, c)) = s.most_common {
        ui::kv("Most common", &format!("0x{b:02X} ({c}x)"));
    }
    ui::kv(
        "Printable",
        &format!("{} ({:.1}%)", s.printable, s.printable_ratio() * 100.0),
    );
    ui::kv("Nulls", &s.nulls.to_string());
    ui::kv("High-bit", &s.high_bit.to_string());

    println!("\nTop {top} bytes:");
    print!("{}", s.histogram_text(top));
    Ok(())
}

/// `diff`: bandingkan dua capture.
pub fn diff(left: &str, right: &str, text: bool, json: bool, _color: bool) -> Result<()> {
    let a = load_bytes(left)?;
    let b = load_bytes(right)?;

    if text {
        let d = adiff::diff_text(&String::from_utf8_lossy(&a), &String::from_utf8_lossy(&b));
        if json {
            println!("{}", serde_json::to_string_pretty(&d)?);
            return Ok(());
        }
        ui::header("DIFF (text)");
        ui::kv("Left", left);
        ui::kv("Right", right);
        ui::kv("Summary", &d.summary());
        println!();
        print!("{}", d.render());
    } else {
        let d = adiff::diff_bytes(&a, &b);
        if json {
            println!("{}", serde_json::to_string_pretty(&d)?);
            return Ok(());
        }
        ui::header("DIFF (binary)");
        ui::kv("Left", &format!("{left} ({} bytes)", d.left_len));
        ui::kv("Right", &format!("{right} ({} bytes)", d.right_len));
        ui::kv("Identical", if d.is_identical() { "yes" } else { "no" });
        ui::kv("Similarity", &format!("{:.1}%", d.similarity() * 100.0));
        ui::kv("Differences", &d.differing.len().to_string());
        if !d.differing.is_empty() {
            println!("\nFirst 40 differences:");
            for diff in d.differing.iter().take(40) {
                println!(
                    "  0x{:08X}  {:02X} -> {:02X}",
                    diff.offset, diff.left, diff.right
                );
            }
        }
    }
    Ok(())
}

/// `config`: lihat/ubah konfigurasi.
pub fn config(path: bool, init: bool, json: bool, _color: bool) -> Result<()> {
    let cfg_path = core_config::config_path();

    if path {
        println!("{}", cfg_path.display());
        return Ok(());
    }

    if init {
        let cfg = core_config::Config::default();
        let written = core_config::save(&cfg)?;
        println!("Config default ditulis ke {}", written.display());
        return Ok(());
    }

    let cfg = core_config::load();
    if json {
        println!("{}", serde_json::to_string_pretty(&cfg)?);
        return Ok(());
    }

    ui::header("CONFIG");
    ui::kv("Path", &cfg_path.display().to_string());
    ui::kv("Default baud", &cfg.default_baudrate.to_string());
    ui::kv("Default format", &cfg.default_format);
    ui::kv("Scan baudrates", &format!("{:?}", cfg.scan_baudrates));
    ui::kv("Scan duration", &format!("{} ms", cfg.scan_duration_ms));
    ui::kv("Sessions dir", &cfg.sessions_dir);
    ui::kv("Captures dir", &cfg.captures_dir);
    ui::kv("Min string len", &cfg.min_string_len.to_string());
    ui::kv("Entropy block", &cfg.entropy_block_size.to_string());
    ui::kv("Color", &cfg.color.to_string());
    Ok(())
}
