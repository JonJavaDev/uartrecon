# Changelog

Semua perubahan penting proyek ini didokumentasikan di sini.
Format berdasarkan [Keep a Changelog](https://keepachangelog.com/),
dan proyek ini mengikuti [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- **Logic analyzer** (`uartrecon-core::logic`): physical-layer UART decoding
  dari waveform digital (edge detection, pulse stats, estimasi baud fisik,
  decoder UART lengkap dengan parity & stop bits).
- **Firmware signature scanner** (binwalk-like): SquashFS, JFFS2, cramfs, UBIFS,
  ext, gzip, xz, LZMA, bzip2, zstd, LZ4, 7-Zip, tar, cpio, uImage, FIT/DTB,
  bzImage, Android boot, ELF, PE, ZIP, PNG, JPEG, SQLite.
- **Entropy analysis**: Shannon entropy, klasifikasi (structured/mixed/
  compressed/encrypted), deteksi region high-entropy.
- **String extraction**: ekstraksi printable strings + deteksi pola menarik
  (password, URL, path, kredensial).
- **Byte statistics**: histogram, distribusi, printable ratio.
- **Diff**: bandingkan dua capture (teks baris & biner byte-level).
- **Search**: pencarian literal/regex/hex/icase dalam capture.
- **Config persisten** (`~/.config/uartrecon/config.toml` atau `%APPDATA%`).
- **GUI desktop** (egui/eframe): terminal, detection, waveform viewer, session browser.
- **Menu interaktif** di CLI saat dijalankan tanpa argumen.
- TUI panel waveform.
- Subcommand CLI: `logic`, `search`, `strings`, `entropy`, `signatures`,
  `stats`, `diff`, `config`.
- CI workflow (Windows + Linux + macOS) dan release workflow.

## [0.1.0] - 2026-10-08

### Added
- Enumerasi serial port & identifikasi USB-UART (CP2102, CH340, FT232, ...).
- Scanner baudrate adaptif + scoring 5-komponen.
- Deteksi format UART (8N1, 8E1, 8O1, 8N2, 7E1, 7O1).
- Capture engine (RX/TX terpisah) + sistem sesi + SHA-256.
- Fingerprinting (U-Boot / Linux / BusyBox / vendor) dengan weighted evidence.
- Analyzer: bootlog, Linux, U-Boot, MTD/partitions.
- Command risk model (ReadOnly / StateChanging / Destructive).
- Device profiles (TOML).
- CLI + TUI (ratatui).
- Dataset regression test (tanpa hardware).

[Unreleased]: https://github.com/USER/uartrecon/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/USER/uartrecon/releases/tag/v0.1.0
