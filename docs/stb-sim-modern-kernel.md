# Simulasi STB dengan Kernel Modern (QEMU)

Simulasi **ZTE B700V5S1** di QEMU memakai **kernel Linux modern (6.x)**, bukan
kernel bawaan MediaTek 2.6.35 yang jadul.

## Kenapa

STB asli:

| Aspek | STB asli | Simulasi ini |
|-------|----------|--------------|
| SoC | MediaTek MT8653 | QEMU `raspi1ap` (BCM2835) |
| CPU | **ARM1176JZF-S (ARMv6K)** | **ARM1176JZF-S** (`-cpu arm1176`, part `0xb76`) |
| Kernel | Linux **2.6.35** (MediaTek, ~2010) | Linux **6.18.x** (Alpine, modern) |
| RAM | 512 MB | 512 MB |
| Flash | NAND 256 MB | disk image 256 MB |
| Userspace | LEDE 17.01.6 (musl, armv6) | Alpine 3.24 (musl, armv6l) |

CPU yang diemulasi **identik** dengan STB (`CPU part 0xb76`), jadi hasilnya
representatif untuk uji kompatibilitas armv6l. Yang diganti hanya kernel
(2.6.35 -> 6.x), sesuai permintaan.

## Prasyarat

- **QEMU** (`qemu-system-arm`) + `qemu-img`
  - MSYS2: `pacman -S mingw-w64-ucrt-x86_64-qemu mingw-w64-ucrt-x86_64-qemu-image-util`
- **Python 3** (untuk membangun initramfs)
- **Koneksi internet** (sekali saja, untuk unduh kernel/rootfs)
- (opsional) **Git**, hanya jika unduhan DTB langsung gagal

## Pakai

```powershell
cd scripts\stb-sim

# 1. Unduh kernel/rootfs + bangun initramfs + buat disk (sekali saja)
.\stb-sim.ps1 prepare

# 2. Boot (console serial interaktif)
.\stb-sim.ps1 run

# Lihat environment & berkas
.\stb-sim.ps1 info
```

Saat boot: tunggu prompt `STB-B700V5 login:`, lalu login `root` (tanpa password).
Keluar dari QEMU: `Ctrl+A` lalu `X`.

## Contoh keluaran

```
================================================================
   UARTRecon - SIMULASI STB (ZTE B700V5S1)
================================================================
  SoC (emulasi) : ARM1176JZF-S  (ARMv6K) - sama dgn STB asli
  Board         : QEMU raspi1ap (BCM2835) @ RAM 512MB
  Kernel        : 6.18.52-0-rpi
  Kernel lama   : 2.6.35 (MediaTek) -> DIGANTI
  Arsitektur    : armv6l
  Hostname      : STB-B700V5
----------------------------------------------------------------
  CPU info:
    model name  : ARMv6-compatible processor rev 7 (v6l)
    BogoMIPS    : 383.38
    Features    : half thumb fastmult vfp edsp java tls
    CPU part    : 0xb76
----------------------------------------------------------------
  Memori:
    MemTotal:         437464 kB
----------------------------------------------------------------
  Storage (NAND simulasi 256MB):
    brw-------    1 root     root      179,   0 /dev/mmcblk0
----------------------------------------------------------------
  Distribusi:
    Alpine 3.24.0
================================================================
```

```
STB-B700V5:~# uname -a
Linux STB-B700V5 6.18.52-0-rpi #1-Alpine armv6l Linux
STB-B700V5:~# cat /proc/cpuinfo | grep -E 'CPU part|model name'
model name  : ARMv6-compatible processor rev 7 (v6l)
CPU part    : 0xb76
```

## Struktur berkas

```
scripts/stb-sim/
├── stb-sim.ps1          # runner (prepare/run/info)
├── build_initramfs.py   # tar.gz Alpine -> cpio.gz (symlink/hardlink benar)
└── work/                # (git-ignored) kernel, rootfs, dtb, image
    ├── vmlinuz-rpi
    ├── stb-rootfs.cpio.gz
    ├── bcm2708-rpi-b-plus.dtb
    └── nand256.img
```

## Catatan teknis

- **Kenapa initramfs dibangun ulang?** Windows (tanpa Developer Mode) tidak
  bisa membuat symlink native. `build_initramfs.py` membaca tar Alpine lalu
  menulis ulang jadi arsip cpio `newc` dengan symlink & hardlink yang benar;
  kernel di dalam emulasi yang mengekstraknya, jadi symlink valid.
- **`initcall_blacklist=bcm2835_power_driver_init`**: driver power-management
  BCM2835 di QEMU tidak lengkap sehingga memicu fault. Di-*blacklist* agar
  boot lanjut. Ini keterbatasan emulasi, bukan kernel.
- Kernel yang dipakai: `vmlinuz-rpi` (Alpine armhf) — build resmi untuk
  Raspberry Pi/ARMv6, sehingga cocok dengan CPU ARM1176.
- QEMU `-cpu arm1176` membuat guest melihat CPU part `0xb76` = ARM1176JZF-S,
  sama persis dengan STB asli.
