# Analysis Tools

UARTRecon menyediakan serangkaian analyzer untuk memproses capture UART
maupun file firmware. Semua analyzer bersifat **read-only** dan bekerja di atas
byte mentah.

## Daftar Analyzer

| Analyzer | Modul | Fungsi |
|----------|-------|--------|
| Signatures | `analyzers::signatures` | Cari magic bytes (binwalk-like) |
| Entropy | `analyzers::entropy` | Deteksi region compressed/encrypted |
| Strings | `analyzers::strings` | Ekstraksi teks printable |
| Stats | `analyzers::stats` | Histogram byte & metrik |
| Diff | `analyzers::diff` | Bandingkan dua capture |
| Bootlog | `analyzers::bootlog` | Parse boot log |
| Linux | `analyzers::linux` | Deteksi Embedded Linux |
| U-Boot | `analyzers::uboot` | Deteksi U-Boot |
| MTD | `analyzers::mtd` | Parse tabel partisi |
| Search | `capture::search` | Cari pola (literal/regex/hex) |
| Logic | `logic` | Physical-layer UART decode |

## Signature Scanner

Memindai data untuk struktur yang dikenal. Kategori: filesystem, archive,
kernel, bootloader, executable, image, database.

Signature yang didukung: SquashFS, JFFS2, cramfs, UBIFS, ext2/3/4, gzip, xz,
LZMA, bzip2, Zstandard, LZ4, 7-Zip, tar, cpio, U-Boot uImage, FIT/DTB, bzImage,
Android boot, U-Boot, ELF, PE, ZIP, PNG, JPEG, SQLite.

Signature dengan magic <= 3 byte ditandai `weak` (rawan false positive) dan
hanya ditampilkan dengan `--all`.

```bash
cargo run -- signatures --file firmware.bin
cargo run -- signatures --file firmware.bin --all --json
```

## Entropy

Entropi Shannon (bit/byte). Klasifikasi:
- `< 3.5` → structured/text
- `3.5..=6.5` → mixed
- `6.5..=7.5` → compressed
- `> 7.5` → encrypted/random

Region dengan entropi tinggi menandakan data terkompresi/terenkripsi
(kandidat untuk ekstraksi lebih lanjut).

```bash
cargo run -- entropy --file firmware.bin --block-size 1024 --threshold 7.0
```

## Strings

Ekstraksi string ASCII printable. Deteksi "menarik" mencakup: password, passwd,
root:, admin, URL (http/https/ftp), telnet, ssh, path (/bin, /etc, /proc, /dev),
token, secret, key, version, busybox, login, wifi, ssid.

```bash
cargo run -- strings --file firmware.bin --interesting
cargo run -- strings --file firmware.bin --grep "http"
```

## Stats

Histogram 256-bin + metrik: total, unique bytes, most/least common, printable
ratio, nulls, high-bit.

```bash
cargo run -- stats --file capture.raw --top 20
```

## Diff

Bandingkan dua capture:
- **Teks**: diff baris (LCS-based, unified-diff-like).
- **Biner**: laporan offset byte berbeda + persentase similarity.

```bash
cargo run -- diff --left a.raw --right b.raw
cargo run -- diff --left sessions/before --right sessions/after --text
```

## Search

Pencarian pola dengan mode:
- `literal` — byte literal case-sensitive
- `icase` — literal case-insensitive
- `regex` — regex (pada teks lossy)
- `hex` — pola hex (`DE AD BE EF` / `DEADBEEF` / `de:ad:be:ef`)

```bash
cargo run -- search --file capture.raw "DRAM" --mode literal
cargo run -- search --file capture.raw "\d+ MiB" --mode regex
cargo run -- search --file capture.raw "deadbeef" --mode hex
```

## Input Fleksibel

Semua command `--file` menerima:
- File raw tunggal
- Direktori sesi (otomatis baca `rx.raw` + `tx.raw`)

## Output JSON

Semua command mendukung `--json` untuk integrasi dengan tool lain:

```bash
cargo run -- signatures --file firmware.bin --json > sigs.json
cargo run -- strings --file firmware.bin --json | jq '.[] | select(.value | test("http"))'
```
