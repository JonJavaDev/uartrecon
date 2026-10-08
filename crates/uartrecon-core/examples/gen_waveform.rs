//! Contoh: membuat waveform UART sintetis dan menuliskannya sebagai file raw.
//!
//! Jalankan dengan:
//! ```text
//! cargo run -p uartrecon-core --example gen_waveform -- testdata/waveform/demo.raw
//! ```

use std::env;
use std::fs;

use uartrecon_core::logic;
use uartrecon_core::serial::config::SerialFormat;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let out = args
        .first()
        .cloned()
        .unwrap_or_else(|| "demo.raw".to_string());

    let payload = b"U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nLinux version 5.10\r\nBusyBox\r\nlogin: ";
    let wave = logic::synth_uart(payload, 115_200, 8, SerialFormat::EIGHT_N_ONE);

    fs::write(&out, &wave.samples)?;
    println!(
        "Waveform ditulis ke {out}: {} sampel @ {} Hz ({:.2} ms)",
        wave.samples.len(),
        wave.sample_rate,
        wave.duration_ms()
    );
    Ok(())
}
