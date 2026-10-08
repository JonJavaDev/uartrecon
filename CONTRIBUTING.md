# Contributing ke UARTRecon

Terima kasih atas minat Anda! Dokumen ini menjelaskan cara berkontribusi.

## Prinsip Proyek

1. **Read-only first.** Tool tidak boleh melakukan operasi destruktif
   (erase/write flash, format, reboot paksa, modifikasi bootloader/environment).
   Fitur yang mengubah state harus meminta konfirmasi eksplisit.
2. **Raw data adalah source of truth.** Semua analisis bekerja di atas byte mentah.
3. **Core tidak bergantung pada UI.** `uartrecon-core` tidak boleh import
   `clap`, `ratatui`, `egui`, atau GUI/CLI apa pun.
4. **Testable tanpa hardware.** Algoritma harus bisa diuji dengan data sintetis
   (lihat trait `SampleSource` dan `synth_uart`).
5. **Jujur soal batasan.** Jangan mengklaim lebih dari yang bisa dibuktikan
   (mis. "probability" → gunakan "confidence score").

## Setup Development

```bash
git clone <repo>
cd uartrecon
cargo build
cargo test --workspace
```

Toolchain: Rust stable (edition 2024). Lihat `rust-toolchain.toml`.

## Sebelum Commit

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Semua harus lulus tanpa warning.

## Struktur

- `crates/uartrecon-core` — engine (serial, detector, capture, analyzers, logic)
- `crates/uartrecon-cli` — binary `uartrecon` (menu + subcommand)
- `crates/uartrecon-tui` — binary `uartrecon-tui` (ratatui)
- `crates/uartrecon-gui` — binary `uartrecon-gui` (egui/eframe)

## Menambah Fitur Analyzer

1. Buat modul di `crates/uartrecon-core/src/analyzers/`.
2. Definisikan struct hasil yang `Serialize`/`Deserialize`.
3. Tulis fungsi `analyze`/`compute`/`scan` yang menerima `&[u8]`.
4. Tambahkan unit test di modul yang sama.
5. Tambahkan integration test di `crates/uartrecon-core/tests/`.
6. Expose via `analyzers/mod.rs` dan tambahkan subcommand di CLI.

## Menambah Device Profile

Tambahkan file TOML di `profiles/`. Lihat `profiles/stb.toml` sebagai contoh.

## Menambah Firmware Signature

Edit `MAGICS` di `crates/uartrecon-core/src/analyzers/signatures.rs`.
Sertakan nama, kategori, magic bytes, dan deskripsi. Tandai magic pendek
(<= 3 byte) — akan otomatis dianggap "weak".

## Commit Message

Gunakan format konvensional:

```
feat: tambah analyzer UBIFS
fix: perbaiki deteksi stop bit 2
docs: perjelas batasan baud fisik
test: tambah dataset 921600
```

## Cross-Platform

Kode harus compile di Windows, Linux, dan macOS. Hindari path hardcoded;
gunakan `std::path`. Untuk direktori config gunakan `uartrecon_core::config::config_dir()`.

## Pertanyaan

Buka issue di GitHub.
