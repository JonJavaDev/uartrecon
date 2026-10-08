# UARTRecon

[![CI](https://github.com/USER/uartrecon/actions/workflows/ci.yml/badge.svg)](https://github.com/USER/uartrecon/actions/workflows/ci.yml)
[![Release](https://github.com/USER/uartrecon/actions/workflows/release.yml/badge.svg)](https://github.com/USER/uartrecon/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-windows%20%7C%20linux%20%7C%20macos-blue.svg)](#instalasi)

**UART Recon & Analysis Toolkit** — toolkit native cross-platform (Rust) untuk
reconnaissance perangkat embedded melalui UART (USB-to-TTL).

> **Read-only first.** UARTRecon hanya melakukan *detect → capture → analyze →
> export*. Tidak ada operasi erase/write flash, format filesystem, reboot paksa,
> atau modifikasi bootloader/environment.

---

## Fitur

**Recon & Capture**
- Enumerasi serial port + identifikasi USB-UART (CP2102, CH340, FT232, PL2303, ...)
- Scanner baudrate adaptif + scoring 5-komponen (confidence 0–100)
- Deteksi format UART (8N1, 8E1, 8O1, 8N2, 7E1, 7O1)
- Capture engine RX/TX terpisah + sistem sesi + SHA-256
- Fingerprinting (U-Boot / Linux / BusyBox / vendor) dengan weighted evidence
- Command risk model (ReadOnly / StateChanging / Destructive)

**Analisis Lanjutan**
- **Logic analyzer**: physical-layer UART decoding dari waveform (edge detection,
  bit period, estimasi baud fisik, decoder frame lengkap)
- **Signature scanner** (binwalk-like): SquashFS, JFFS2, cramfs, UBIFS, gzip, xz,
  LZMA, zstd, LZ4, uImage, FIT/DTB, bzImage, ELF, ...
- **Entropy**: deteksi region compressed/encrypted
- **Strings**: ekstraksi teks + deteksi pola menarik (password, URL, path)
- **Stats**: histogram byte & metrik ringan
- **Diff**: bandingkan dua capture (teks & biner)
- **Search**: literal / regex / hex / case-insensitive

**Interfaces**
- CLI dengan **menu interaktif** (cukup `cargo run`)
- TUI (ratatui) dengan panel terminal & waveform
- GUI desktop (egui/eframe) dengan tab Terminal / Detection / Waveform / Sessions
- Config persisten (TOML)

---

## Instalasi

### Dari source (semua platform)

```bash
git clone https://github.com/USER/uartrecon.git
cd uartrecon
cargo build --release
```

Binary di `target/release/`:
- `uartrecon` (CLI)
- `uartrecon-tui` (TUI)
- `uartrecon-gui` (GUI)

### Prasyarat per platform

**Windows**
- Rust stable (rustup) — toolchain `x86_64-pc-windows-msvc`
- Visual Studio Build Tools 2022 (workload "Desktop development with C++")
- Driver USB-UART (CP210x, CH340, FTDI, ...)

**Linux**
- Rust stable
- `libudev-dev` (untuk serialport):
  ```bash
  sudo apt install libudev-dev pkg-config
  ```
- Untuk GUI: `libgtk-3-dev libxkbcommon-dev libwayland-dev`
- Akses port: tambahkan user ke grup `dialout`:
  ```bash
  sudo usermod -aG dialout $USER   # logout/login setelahnya
  ```

**macOS**
- Rust stable
- Xcode Command Line Tools: `xcode-select --install`
- Driver USB-UART sesuai chip

### Download binary

Lihat [Releases](https://github.com/USER/uartrecon/releases) — tersedia untuk
Windows x64, Linux x64/ARM64, dan macOS x64/ARM64.

---

## Status

| Komponen | Status |
|----------|--------|
| Port detection & USB-UART identification | ✅ |
| Baudrate scanning (adaptive) + scoring | ✅ |
| UART format detection (8N1, 8E1, …) | ✅ |
| Capture engine (RX/TX terpisah) | ✅ |
| Session system + hashing (SHA-256) | ✅ |
| Fingerprinting (U-Boot / Linux / BusyBox / vendor) | ✅ |
| Analyzer (bootlog, Linux, U-Boot, MTD/partitions) | ✅ |
| Command risk model (read-only first) | ✅ |
| CLI + **menu interaktif** | ✅ |
| TUI (ratatui) + panel waveform | ✅ |
| **Logic analyzer** (physical baud + UART decoder) | ✅ |
| **GUI desktop** (egui/eframe) | ✅ |

---

## Quick Start — cukup satu perintah

```bash
cd uartrecon
cargo run
```

`cargo run` **langsung membuka menu interaktif** — tidak perlu `-p` atau argumen:

```
UARTRecon — UART Recon & Analysis Toolkit
read-only first · detect · capture · analyze · export

MENU UTAMA
────────────────────
  [1] Daftar serial port
  [2] Scan baudrate & format (auto-detect)
  [3] Monitor data
  [4] Mode interaktif (kirim command)
  [5] Analisis file capture
  [6] Logic analyzer (waveform)
  [7] Lihat sesi tersimpan
  [8] Doctor (cek environment)
  [q] Keluar

Pilih:
```

### Menjalankan TUI & GUI

```bash
cargo run -p uartrecon-tui -- --demo    # TUI dengan data demo
cargo run -p uartrecon-tui -- COM7 --baud 115200
cargo run -p uartrecon-gui              # GUI desktop (jendela)
```

---

## Arsitektur

```
uartrecon/
├── crates/
│   ├── uartrecon-core/     # Engine inti (tanpa clap/ratatui/egui) — reusable
│   ├── uartrecon-cli/      # Binary `uartrecon` (menu interaktif + subcommand)
│   ├── uartrecon-tui/      # Binary `uartrecon-tui` (ratatui)
│   └── uartrecon-gui/      # Binary `uartrecon-gui` (egui/eframe)
├── profiles/               # Device profiles (TOML)
├── testdata/               # Dataset regression (per baudrate)
├── captures/               # Output capture (di-ignore git)
└── sessions/               # Sesi tersimpan (di-ignore git)
```

`uartrecon-core` **tidak** bergantung pada `ratatui`, `clap`, atau `egui`,
sehingga CLI, TUI, dan GUI berbagi logika yang sama.

---

## Logic Analyzer (v0.4) — physical-layer decoding

Selain mode USB-UART (yang hanya menerima byte hasil decoding), UARTRecon kini
memiliki **logic analyzer mode** yang bekerja pada **waveform digital** mentah:

```
RX waveform → edge detection → pulse measurement → bit period → baud → UART decoder
```

Fitur:
- Deteksi edge (transisi 0↔1) dengan timestamp.
- Statistik lebar pulsa (min/max/mean/median).
- **Estimasi baudrate fisik** dari bit period (di-snap ke kandidat standar).
- **Decoder UART lengkap**: start bit, data (LSB-first), parity (N/E/O),
  stop bit (1/2), dengan deteksi error framing/parity.

### Contoh (akurat 0.00%)

```bash
cargo run -- logic --demo
```

```
WAVEFORM
  Sample rate  : 921600 Hz
  Duration     : 7.14 ms

ANALYSIS
  Edges        : 502
  Bit period   : 8.681 µs
  Baud (fisik) : 115200 (raw 115200 baud, error 0.00%)

DECODED
  Format       : 8N1
  Bytes        : 82 (0 error)
  "U-Boot 2021.10\r\nDRAM: 512 MiB\r\nNAND: 256 MiB\r\nLinux version 5.10\r\nBusyBox\r\nlogin: "
```

Dari file waveform:

```bash
cargo run -- logic --file waveform.raw --sample-rate 921600 --format 8N1
cargo run -- logic --file waveform.csv --json
```

Format input: **raw biner** (byte 0/1) atau **CSV** (`index,level` per baris).

### GUI desktop (egui)

```bash
cargo run -p uartrecon-gui
```

Tab: **Terminal** (live, ASCII/HEX/RAW), **Detection** (scan + fingerprint),
**Waveform** (plot sinyal + hasil analisis logic analyzer), **Sessions**
(browser sesi). Panel kontrol di kiri: pilih port, baud, format, connect,
auto-scan, simpan sesi.

---

## Penggunaan CLI (subcommand langsung)

```bash
# Daftar serial port (USB-UART diidentifikasi)
cargo run -- ports

# Scan baudrate & format pada sebuah port
cargo run -- scan COM7
cargo run -- scan COM7 --baudrates 9600,115200 --duration-ms 800 --json

# Monitor data (simpan ke sesi)
cargo run -- monitor COM7 --baud 115200 --session stb01

# Mode interaktif (read-only, perintah destruktif diblokir)
cargo run -- connect COM7 --baud 115200
cargo run -- connect COM7 --auto

# Analisis file capture
cargo run -- analyze capture.raw
cargo run -- analyze capture.raw --format json
cargo run -- analyze capture.raw --hex

# Logic analyzer
cargo run -- logic --demo
cargo run -- logic --file waveform.raw --sample-rate 921600

# Analisis lanjutan (bisa dari file raw ATAU direktori sesi)
cargo run -- search --file capture.raw "DRAM" --mode literal
cargo run -- search --file sessions/stb01 "\d+ MiB" --mode regex
cargo run -- strings --file firmware.bin --interesting
cargo run -- entropy --file firmware.bin --threshold 7.0
cargo run -- signatures --file firmware.bin
cargo run -- stats --file capture.raw --top 16
cargo run -- diff --left capture_a.raw --right capture_b.raw
cargo run -- diff --left sessions/a --right sessions/b --text

# Lihat sesi tersimpan (tanpa capture ulang)
cargo run -- session stb01

# Konfigurasi
cargo run -- config
cargo run -- config --init
cargo run -- config --path

# Cek environment
cargo run -- doctor
```

### Opsi `--file` menerima file ATAU direktori sesi

Semua command analisis (`search`, `strings`, `entropy`, `signatures`, `stats`,
`diff`) menerima path ke file raw **atau** direktori sesi (akan otomatis
membaca `rx.raw` + `tx.raw`).

---

## Penggunaan TUI

```bash
cargo run -p uartrecon-tui -- COM7 --baud 115200
cargo run -p uartrecon-tui -- COM7 --auto
cargo run -p uartrecon-tui -- --demo        # demo tanpa hardware
```

Tombol: `Tab` ganti panel · `h` ganti mode (ASCII/HEX/RAW) · `f` fingerprint ·
`w` waveform (logic analyzer) · `c` clear · `j`/`k` scroll · `q`/`Ctrl+C` keluar.

## Penggunaan GUI

```bash
cargo run -p uartrecon-gui
```

Tab: **Terminal** (live), **Detection** (scan + fingerprint), **Waveform**
(plot + analisis), **Sessions** (browser). Panel kiri: pilih port, baud, format,
connect, auto-scan, simpan sesi.

---

## Analisis Firmware (binwalk-like)

UARTRecon dapat menganalisis file firmware yang sudah di-dump:

```bash
# 1. Cari struktur/format
cargo run -- signatures --file firmware.bin

# 2. Lihat string menarik (password, URL, path)
cargo run -- strings --file firmware.bin --interesting

# 3. Deteksi region terkompresi/terenkripsi
cargo run -- entropy --file firmware.bin --threshold 7.0

# 4. Cari pola spesifik (mis. versi kernel)
cargo run -- search --file firmware.bin "\d+\.\d+\.\d+" --mode regex

# 5. Statistik
cargo run -- stats --file firmware.bin --top 20
```

Output JSON tersedia di semua command dengan `--json` untuk diproses lebih lanjut.

---

## Batasan Teknis Penting (baca ini)

USB-to-UART biasa **tidak** memberikan raw edge timing ke aplikasi. Pipeline-nya:

```
UART signal → USB-UART hardware → decoded byte → USB → OS → Rust app
```

Timing bit-level telah diproses oleh chip USB-UART. Karena itu:

- Mode **USB-UART** memakai **adaptive baud scanning + data scoring**, **bukan**
  deteksi baud fisik.
- Angka "confidence" adalah **skor heuristik 0–100**, **bukan** probabilitas
  statistik. Dokumentasi ini tidak mengklaim "97% probability".
- Untuk deteksi baud **fisik** sebenarnya, gunakan **logic analyzer mode**
  (`logic --file <waveform>`), yang mengukur bit period dari waveform mentah.
  Mode ini memerlukan perangkat yang memberi akses sinyal digital (logic
  analyzer / capture), bukan sekadar USB-UART.

Bila tidak ada traffic yang dapat diandalkan, tool akan menyatakan dengan jujur:

```
No reliable traffic detected.
Automatic baud detection cannot determine a configuration.
```

…alih-alih memaksakan hasil.

---

## Scoring System

| Komponen                | Rentang  |
|-------------------------|----------|
| Printable ratio         | +0..40   |
| Valid line structure    | +0..15   |
| Known embedded pattern  | +0..25   |
| UTF-8 validity          | +0..10   |
| Binary randomness       | −0..30   |

Skor akhir di-clamp ke 0..100 dan ditampilkan sebagai **confidence score**.

---

## Sistem Sesi

Setiap capture disimpan sebagai direktori di `sessions/`:

```
sessions/2026-10-08_stb01/
├── metadata.json     # port, config, timestamp, hash
├── rx.raw            # raw RX (source of truth)
├── tx.raw            # raw TX (terpisah dari RX)
├── output.txt        # representasi teks
├── output.hex        # hexdump
└── events.json       # timing event per chunk
```

RX dan TX **selalu** dipisah. Raw data adalah source of truth; semua format lain
diturunkan darinya. Setiap sesi diberi SHA-256 untuk integrity/evidence tracking.

> Timestamp pada bootlog berasal dari **host capture**, bukan clock internal device.

---

## Model Risiko Perintah

| Kategori | Perilaku |
|----------|----------|
| `ReadOnly` | Diizinkan langsung |
| `StateChanging` | Butuh konfirmasi user |
| `Destructive` | **Diblokir** (erase/write/reboot/setenv/…) |

UARTRecon **hanya menyarankan** perintah read-only; user harus memilih dan
mengirim sendiri. Tidak ada eksekusi otomatis.

---

## Testing

```bash
cargo test --workspace              # semua test
cargo test --test baud_detection    # regression dataset
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Test berjalan **tanpa hardware** melalui trait `SampleSource` (mock in-memory).
Hardware test adalah layer terpisah.

---

## Fitur yang Sengaja Tidak Ada di v1

- Automatic flash writing / erase
- Automatic bootloader modification
- Automatic exploit / root
- Automatic firmware patching

Bottleneck awal proyek adalah **UART configuration + reliable capture +
reliable analysis**, bukan eksploitasi.

---

## Roadmap

- **v0.1 Foundation** — enumerasi, terminal, capture, scanner, scoring, format, sesi ✅
- **v0.2 Recon** — fingerprint U-Boot/Linux/BusyBox, bootlog parser ✅
- **v0.3 Embedded** — analyzer `/proc`, MTD, partition, command suggestion, STB profile ✅
- **v0.4 Advanced** — logic analyzer, physical baud detection, UART frame decoder ✅
- **v0.5 GUI** — desktop UI (egui/eframe), terminal live, detection, waveform viewer, session browser ✅
- **v0.6 Analysis** — signature scanner, entropy, strings, stats, diff, search, config ✅
- **Future** — plugin system, lebih banyak signature/profile, protokol lain (SWD/JTAG), GUI plugin

---

## Kontribusi

Kontribusi dipersilakan! Lihat [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Lisensi

MIT — lihat [LICENSE](LICENSE).

## Disclaimer

UARTRecon ditujukan untuk **penelitian, edukasi, dan reverse engineering
perangkat milik sendiri**. Selalu dapatkan izin sebelum menganalisis perangkat
yang bukan milik Anda. Tool ini bersifat *read-only first* dan tidak
menyediakan fitur yang menulis/merusak flash.
