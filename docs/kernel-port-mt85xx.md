# Port Kernel Linux untuk MediaTek MT85xx (STB)

Upaya membuat **kernel Linux mainline** untuk SoC MediaTek MT85xx (MT8653) yang
dipakai STB ZTE B700V5S1.

## Status

| Tahap | Status |
|-------|--------|
| Riset dukungan mainline | ✅ Selesai (hasil: **tidak ada**) |
| Port platform `mach-mt85xx` | ✅ Skeleton dibuat |
| Device tree `zte-b700v5s1.dts` | ✅ Dibuat |
| Build kernel ARMv6 | ✅ **BERHASIL** (Linux 6.12.0) |
| Boot di QEMU (ARM1176JZF-S) | ✅ **BERHASIL** (sampai prompt login) |
| Boot di hardware asli | ❌ **BELUM** (driver SoC belum ada) |

## Temuan penting

### MT85xx TIDAK ada di mainline Linux

Riset pada source kernel mainline:

| Platform | Isi | MT85xx? |
|----------|-----|---------|
| `arch/arm/mach-mediatek` | MT2701, MT6572, MT6582, MT6589, MT6592, MT7623, MT7629, MT8127, MT8135 — **ARMv7 saja** | ❌ |
| `arch/arm/mach-gemini` | **Cortina Systems Gemini** (ARMv4/FA526) — **bukan MediaTek** | ❌ |

### Anomali: LEDE pakai target `gemini/wiligear`

STB menjalankan LEDE 17.01.6 dengan `DISTRIB_TARGET='gemini/wiligear'` dan
`DISTRIB_ARCH='arm_fa526'`, **padahal CPU-nya ARMv6** (ARM1176JZF-S, part
`0xb76`). Ini karena LEDE memakai platform **Cortina Gemini** (ARMv4) sebagai
"kendaraan": userspace ARMv4 dijalankan di atas CPU ARMv6 MediaTek.

Target `gemini/wiligear` **tidak ada** di OpenWrt 18.06–24.10 (hanya sampai
LEDE 17.01.7).

### Driver SoC bersifat proprietary

Kernel vendor (2.6.35) memuat driver yang **tidak open source**:

| Driver | Vendor | Status upstream |
|--------|--------|-----------------|
| NAND flash | `mtk_nand` | ❌ tidak ada |
| Ethernet | `star.ko` (MtkUsbHcd) | ❌ tidak ada |
| USB host | `MtkUsbHcd` | ❌ tidak ada |
| Video/HDMI | proprietary | ❌ tidak ada |
| Watchdog | `/proc/net/monitor` | ❌ tidak ada |
| Timer/clock/GPIO | custom | ❌ tidak ada |

## Yang sudah dibuat

```
scripts/stb-sim/kernel-port/
├── build-mt85xx.sh              # script build (jalankan di WSL/Linux)
├── mach-mt85xx/
│   ├── Kconfig                  # ARCH_MT85XX (ARMv6)
│   ├── Makefile
│   ├── mt85xx.c                 # DT machine descriptor
│   └── timer.c                  # stub timer (belum ada driver)
└── dts/
    ├── Makefile
    └── mt85xx-zte-b700v5s1.dts  # device tree (placeholder)
```

### Cara pasang ke source kernel

```bash
cd linux-6.12
cp -r /path/to/kernel-port/mach-mt85xx   arch/arm/
cp -r /path/to/kernel-port/dts/*         arch/arm/boot/dts/mt85xx/

# Daftarkan platform
echo 'source "arch/arm/mach-mt85xx/Kconfig"' >> arch/arm/Kconfig   # letakkan berurutan
sed -i 's|machine-$(CONFIG_ARCH_MEDIATEK).*|&\nmachine-$(CONFIG_ARCH_MT85XX)\t\t+= mt85xx|' arch/arm/Makefile
echo 'subdir-y += mt85xx' >> arch/arm/boot/dts/Makefile
```

## Cara build

### Prasyarat (Debian/Ubuntu)

```bash
sudo apt-get install -y gcc-arm-linux-gnueabi make bc bison flex \
    libssl-dev libncurses-dev device-tree-compiler gcc
```

### Build

```bash
KROOT=~/kernel/linux-6.12 bash build-mt85xx.sh
```

Hasil di `~/mt85xx-out/`:
- `zImage` — kernel (5.4 MB)
- `mt85xx-zte-b700v5s1.dtb` — device tree (1 KB)

> **Catatan Windows**: kernel Linux **tidak bisa** di-build dengan benar di
> MSYS2/MinGW (gagal di `fixdep`). Gunakan WSL Debian atau mesin Linux.
> Build di WSL memakai tarball (ekstrak di dalam WSL) jauh lebih cepat
> daripada copy 93 ribu file lewat `/mnt/c`.

## Cara test di QEMU

```bash
qemu-system-arm \
  -M raspi1ap -cpu arm1176 -m 512 \
  -kernel zImage \
  -dtb bcm2835-rpi-b-plus.dtb \
  -append "console=ttyAMA0,115200 earlycon=pl011,0x20201000 \
           initcall_blacklist=bcm2835_power_driver_init" \
  -display none -serial stdio
```

**Hasil terbukti:**

```
Booting Linux on physical CPU 0x0
Linux version 6.12.0 (uartrecon@stb-sim) (arm-linux-gnueabi-gcc 14.2.0)
CPU: ARMv6-compatible processor [410fb767] revision 7
...
Welcome to Alpine Linux 3.24
Kernel 6.12.0 on armv6l (/dev/ttyAMA0)
STB-B700V5 login:
```

Kernel **6.12.0 hasil build sendiri** boot di CPU **ARM1176JZF-S (ARMv6)** —
arsitektur yang sama dengan STB.

> `initcall_blacklist=bcm2835_power_driver_init` diperlukan karena blok PM
> BCM2835 di QEMU tidak lengkap (celah emulasi, bukan masalah kernel).

## Kenapa belum bisa di hardware asli

Untuk boot di STB sungguhan, harus ada **minimal**:

1. **Interrupt controller** — MT85xx pakai blok custom (bukan GIC).
2. **Timer/clocksource** — blok custom, perlu register manual.
3. **Clock controller** — untuk semua peripheral.
4. **UART** — untuk console (vendor pakai 8250-compatible).
5. **NAND controller** — untuk baca rootfs dari flash.
6. **Ethernet** — kalau mau jaringan.

Semuanya butuh **datasheet MT8653** (tidak publik) dan/atau **reverse
engineering driver vendor**. Ini pekerjaan berbulan-bulan oleh kernel developer
berpengalaman.

## Kesimpulan jujur

| Ingin | Bisa? |
|-------|-------|
| Build kernel mainline ARMv6 | ✅ ya |
| Boot kernel itu di QEMU | ✅ ya |
| Port skeleton MT85xx | ✅ ya |
| **Boot di STB asli** | ❌ butuh driver SoC proprietary |
| Ganti kernel STB dengan mainline | ❌ praktis mustahil tanpa datasheet |

Port ini adalah **titik awal** dan **dokumentasi** dari upaya yang diperlukan —
bukan kernel yang siap dipakai di perangkat.
