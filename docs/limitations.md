# Batasan Teknis & Metodologi Deteksi

Dokumen ini menjelaskan **mengapa** UARTRecon bekerja seperti sekarang, dan apa
yang **tidak** dapat dilakukannya.

## 1. Mengapa deteksi baud tidak bisa "fisik" via USB-UART

USB-to-UART bridge (CP2102, CH340, FT232, PL2303, …) melakukan sampling dan
decoding UART **di dalam chip**. Yang sampai ke aplikasi adalah byte yang sudah
di-decoded:

```
UART signal (RX pin)
    ↓  sampling & framing oleh chip USB-UART
decoded byte
    ↓
USB (bulk transfer)
    ↓
OS driver (COM port / tty)
    ↓
Rust application
```

Konsekuensi: aplikasi **tidak** menerima informasi edge timing, sehingga tidak
mungkin mengukur bit period secara langsung dari jalur ini. Baudrate yang salah
hanya terlihat sebagai "garbage" (byte ter-framing salah), bukan sebagai timing.

## 2. Strategi yang dipakai (adaptive baud scanning)

```
candidate baudrate
        ↓
open/configure UART
        ↓
capture bytes (durasi tetap)
        ↓
analyze bytes (scoring)
        ↓
hitung skor 0..100
        ↓
close/reset
        ↓
next candidate
```

Semua kandidat disimpan dan di-ranking, bukan hanya yang terbaik. Output
menampilkan ranking lengkap + confidence.

## 3. Scoring (confidence, bukan probabilitas)

| Komponen                | Rentang  | Penjelasan |
|-------------------------|----------|------------|
| Printable ratio         | +0..40   | proporsi byte printable |
| Valid line structure    | +0..15   | newline, CR/LF konsisten, panjang baris wajar |
| Known embedded pattern  | +0..25   | U-Boot, Linux, BusyBox, login:, DRAM, NAND, … |
| UTF-8 validity          | +0..10   | validasi UTF-8 ketat |
| Binary randomness       | −0..30   | penalti byte high-bit & entropi |

Skor akhir di-clamp ke 0..100. **Ini skor heuristik**, bukan probability hasil
kalibrasi statistik. Karena itu output memakai istilah *confidence score*, bukan
"97% probability".

## 4. Kapan tool menyatakan gagal

Bila tidak ada kandidat dengan traffic yang cukup dan skor di atas ambang:

```
No reliable traffic detected.
Automatic baud detection cannot determine a configuration.
```

Tool **tidak** memaksakan hasil. Ini lebih jujur daripada menebak.

## 5. Logic analyzer mode (sudah tersedia, v0.4)

Dengan perangkat yang memberi akses waveform mentah (logic analyzer / capture
digital), UARTRecon dapat melakukan deteksi baud **fisik**:

```
RX waveform → edge detection → pulse measurement → bit period → baud → UART decoder
```

Contoh: bit period ≈ 8.681 µs → baud ≈ 1 / 8.681 µs ≈ 115200 (error 0.00%).

Implementasi ada di `uartrecon-core::logic` dan dapat diakses via:

```bash
cargo run -- logic --demo
cargo run -- logic --file waveform.raw --sample-rate 921600
cargo run -- logic --file waveform.csv --json
```

Dua mode (USB-UART dan logic analyzer) berbagi konsep *unified UART decoder*:
hasil decode sama-sama menghasilkan byte yang siap dianalisis lebih lanjut.

## 6. Fingerprinting: weighted evidence

Fingerprint (bootloader, OS, shell, vendor) memakai **bobot** per pola, bukan
satu string. Kategori:

- `Detected` — skor ≥ 60
- `Probable` — skor ≥ 25
- `Unknown` — di bawahnya

Contoh: string "Broadcom" sendirian tidak cukup untuk mengklaim SoC; hasilnya
`Probable` atau `Unknown`. Ini mencegah klaim berlebihan.

## 7. Timestamp

Timestamp bootlog berasal dari **host capture** (waktu saat byte diterima di
PC), **bukan** clock internal device. Jangan menganggapnya sebagai waktu device.
