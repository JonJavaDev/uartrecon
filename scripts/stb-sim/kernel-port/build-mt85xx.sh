#!/bin/bash
# build-mt85xx.sh - build kernel Linux mainline untuk MediaTek MT85xx (ARMv6).
#
# Dijalankan DI DALAM Linux (WSL Debian / mesin Linux asli).
# Kernel TIDAK bisa di-build dengan benar di Windows/MSYS2.
#
# Hasil:
#   arch/arm/boot/zImage                                  (kernel)
#   arch/arm/boot/dts/mt85xx/mt85xx-zte-b700v5s1.dtb      (device tree)
#
# PENTING: kernel ini BOOT di QEMU (CPU ARM1176JZF-S, sama seperti STB),
# tetapi BELUM bisa boot di hardware asli karena driver SoC (NAND, Ethernet,
# USB, video, watchdog) belum ada. Lihat docs/kernel-port-mt85xx.md.
set -e

KVER="${KVER:-6.12}"
KROOT="${KROOT:-$HOME/kernel/linux-$KVER}"
JOBS="$(nproc)"
OUT="${OUT:-$HOME/mt85xx-out}"

export ARCH=arm
export CROSS_COMPILE="${CROSS_COMPILE:-arm-linux-gnueabi-}"
export KBUILD_BUILD_TIMESTAMP="2026-01-01"
export KBUILD_BUILD_USER=uartrecon
export KBUILD_BUILD_HOST=stb-sim

cd "$KROOT"

echo "=== [1] defconfig ARMv6 (BCM2835 base) ==="
make -j"$JOBS" bcm2835_defconfig 2>&1 | tail -2

echo "=== [2] aktifkan ARCH_MT85XX ==="
./scripts/config \
  --enable ARCH_MT85XX \
  --enable ARCH_MULTI_V6 \
  --enable CPU_V6 \
  --enable MT85XX_TIMER \
  --enable MT85XX_UART \
  --enable SERIAL_8250 \
  --enable SERIAL_8250_CONSOLE \
  --enable SERIAL_OF_PLATFORM \
  --enable USE_OF \
  --enable OF \
  --enable EARLY_PRINTK

# Matikan fitur berat (percepat build)
./scripts/config --disable DRM --disable SOUND --disable MEDIA_SUPPORT \
  --disable FB --disable WLAN --disable WIRELESS --disable BT

make -j"$JOBS" olddefconfig 2>&1 | tail -2

echo "=== [3] verifikasi config ==="
grep -E "CONFIG_ARCH_MT85XX|CONFIG_ARCH_MULTI_V6|CONFIG_MT85XX_TIMER|CONFIG_CPU_V6" .config

echo "=== [4] build zImage ==="
make -j"$JOBS" zImage 2>&1 | tail -15

echo "=== [5] build DTB ==="
make -j"$JOBS" dtbs 2>&1 | grep -iE "mt85xx|error" || true

echo "=== [6] salin hasil ==="
mkdir -p "$OUT"
cp arch/arm/boot/zImage "$OUT/"
cp arch/arm/boot/dts/mt85xx/mt85xx-zte-b700v5s1.dtb "$OUT/"
ls -la "$OUT"
echo "SELESAI. Hasil di $OUT"
