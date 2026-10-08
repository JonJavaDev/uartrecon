# UARTRecon

Toolkit buat ngulik perangkat embedded lewat UART (USB-to-TTL). Fokusnya
detect, capture, analyze, export. Semua read-only, nggak ada operasi
tulis/erase flash.

Dibuat pakai Rust, jalan di Windows, Linux, dan macOS.

## Kenapa

Kalau nemu STB/router/modem bekas, biasanya baudrate-nya nggak diketahui.
Manualnya coba 115200, garbage, coba 57600, garbage, coba 9600, baru ketemu.
UARTRecon otomatis nyari baudrate + format yang bener, terus nyimpen hasilnya.

## Fitur

**Detect & capture**
- Daftar serial port + deteksi chip USB-UART (CP2102, CH340, FT232, PL2303)
- Scan baudrate otomatis + scoring (confidence 0-100)
- Deteksi format UART: 8N1, 8E1, 8O1, 8N2, 7E1, 7O1
- Capture RX/TX terpisah, simpan sebagai sesi + hash SHA-256
- Fingerprint: U-Boot, Linux, BusyBox, vendor/SoC

**Analisis**
- Logic analyzer: decode UART dari waveform (bit period, baud fisik)
- Signature scanner (mirip binwalk): SquashFS, JFFS2, UBIFS, gzip, xz, uImage, dll
- Entropy: deteksi bagian compressed/encrypted
- Strings: ekstraksi teks + pola menarik (password, URL, path)
- Stats, diff dua capture, search (literal/regex/hex)

**Operasi device**
- Terminal interaktif real-time (kayak PuTTY, Ctrl+] buat keluar)
- Safety: blokir command yang nyasar ke partisi kritis (bootloader, env)
- Backup & restore partisi via SD card
- Flash LEDE/OpenWrt ke STB (workflow otomatis)

**Bahasa**: Indonesia & English (bisa diganti dari menu atau `uartrecon lang en`)

## Install

**Windows** — ada installer di folder `installer/`:
- `uartrecon-full-0.1.0-setup.exe` (CLI + TUI + GUI)
- `uartrecon-gui-0.1.0-setup.exe` (GUI saja)
- `uartrecon-cli-0.1.0-setup.exe` (CLI + TUI)
- `uartrecon-0.1.0.msi` (MSI)

**Dari source** (semua platform):
```bash
git clone https://github.com/JonJavaDev/uartrecon
cd uartrecon
cargo build --release
```

**Linux** butuh `libudev-dev`. **macOS** butuh Xcode Command Line Tools.

## Pakai

Paling gampang:
```bash
cargo run
```
Langsung buka menu interaktif.

Atau langsung:
```bash
uartrecon ports                          # daftar port
uartrecon terminal COM3 --baud 115200    # terminal interaktif
uartrecon scan COM3                      # scan baudrate otomatis
uartrecon analyze capture.raw            # analisis file
uartrecon signatures --file firmware.bin # scan format firmware
uartrecon safety                         # lihat kebijakan keamanan
```

Cek `uartrecon --help` atau `uartrecon <command> --help`.

## Catatan

USB-to-UART biasa nggak kasih timing sinyal mentah ke aplikasi, jadi deteksi
baud pakai scanning + scoring, bukan pengukuran fisik. Buat baud fisik beneran
perlu logic analyzer (fitur `logic`).

Kalau nggak ada traffic yang bisa diandalkan, tool bilang terus terang
"no reliable traffic detected" — nggak maksa nebak.

## Lisensi

MIT
