//! UARTRecon CLI entry point.

mod cli;
mod commands;
mod terminal;
mod ui;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let result = run(cli);

    if let Err(e) = result {
        ui::error(&format!("{e:#}"));
        std::process::exit(1);
    }
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| format!("uartrecon={level}"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let color = !cli.no_color;
    match cli.command {
        None => commands::interactive_menu(color),
        Some(Command::Ports { json }) => commands::ports(json, color),
        Some(Command::Scan {
            port,
            baudrates,
            duration_ms,
            json,
        }) => commands::scan(&port, baudrates, duration_ms, json, color),
        Some(Command::Connect {
            port,
            baud,
            format,
            auto,
        }) => commands::connect(&port, baud, &format, auto, color),
        Some(Command::Monitor {
            port,
            baud,
            format,
            hex,
            session,
            seconds,
        }) => commands::monitor(&port, baud, &format, hex, session, seconds, color),
        Some(Command::Analyze {
            file,
            format,
            config,
            hex,
        }) => commands::analyze(&file, format, &config, hex, color),
        Some(Command::Session { dir, name, json }) => commands::session(dir, name, json, color),
        Some(Command::Logic {
            file,
            sample_rate,
            format,
            demo,
            json,
        }) => commands::logic(file, sample_rate, &format, demo, json, color),
        Some(Command::Search {
            file,
            pattern,
            mode,
            context,
            json,
        }) => commands::search(&file, &pattern, &mode, context, json, color),
        Some(Command::Strings {
            file,
            min_len,
            interesting,
            grep,
            json,
        }) => commands::strings(&file, min_len, interesting, grep, json, color),
        Some(Command::Entropy {
            file,
            block_size,
            threshold,
            json,
        }) => commands::entropy(&file, block_size, threshold, json, color),
        Some(Command::Signatures { file, all, json }) => {
            commands::signatures(&file, all, json, color)
        }
        Some(Command::Stats { file, top, json }) => commands::stats(&file, top, json, color),
        Some(Command::Diff {
            left,
            right,
            text,
            json,
        }) => commands::diff(&left, &right, text, json, color),
        Some(Command::Config { path, init, json }) => commands::config(path, init, json, color),
        Some(Command::Safety { check, hard, json }) => commands::safety(check, hard, json, color),
        Some(Command::Backup {
            port,
            baud,
            parts,
            critical,
            out,
            device,
        }) => commands::backup(&port, baud, parts, critical, &out, &device, color),
        Some(Command::Recover {
            action,
            port,
            baud,
            device,
            subdir,
            out,
        }) => commands::recover(&action, port, baud, &device, &subdir, out, color),
        Some(Command::Terminal {
            port,
            baud,
            format,
            enter,
            spam,
            log,
        }) => commands::terminal(&port, baud, &format, &enter, spam, log, color),
        Some(Command::FlashLede {
            port,
            baud,
            image,
            target_mtd,
            action,
        }) => commands::flash_lede(&action, &port, baud, &image, target_mtd, color),
        Some(Command::Doctor) => commands::doctor(color),
        Some(Command::Lang { code }) => commands::lang(&code, color),
        Some(Command::Uboot {
            port,
            baud,
            format,
            reboot,
            send,
            seconds,
            log,
        }) => commands::uboot(
            &port,
            baud,
            &format,
            terminal::UbootOptions {
                reboot,
                spam_secs: seconds,
                send_cmd: send,
                log_path: log,
            },
        ),
    }
}
