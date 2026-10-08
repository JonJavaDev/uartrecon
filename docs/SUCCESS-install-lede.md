# ✅ BERHASIL: Install Permanen LEDE ke ZTE B700V5

**Tanggal**: 2026-10-08
**Device**: ZTE B700V5S1 (MediaTek MT8653, ARM1176JZF-S / ARMv6)
**Hasil**: **LEDE 17.01.6 berjalan PERMANEN** dari flash (mtd6 / slot norm)

---

## Ringkasan

STB sekarang boot langsung ke **LEDE (OpenWrt) 17.01.6**:
```
BusyBox v1.25.1 built-in shell (ash)
Reboot (17.01.6, r3979-2252731af4)
root@LEDE:/#
DISTRIB_ARCH='arm_fa526'
Linux LEDE 2.6.35 armv6l
```

- **Kernel**: MediaTek 2.6.35 bawaan (mtd5, tidak diubah)
- **Rootfs**: LEDE di mtd6 (jffs2, rw)
- **Init**: `/sbin/procd` (PID 1)
- **Tools**: opkg, uci, ip, dropbear tersedia

---

## Langkah Install yang BENAR (final)

```sh
# Dari mode SAFE, timpa NORM (mtd6):
mkdir -p /mtd6 /test
mount -t jffs2 /dev/mtdblock6 /mtd6
mount -t squashfs /var/mntt/usba1/openwrt-gemini-b700.squashfs /test

rm -rf /mtd6/*                        # hapus ISI, bukan folder
cp -rp /test/* /mtd6                  # PAKAI /* (kunci!)

mkdir -p /mtd6/boot
cp /boot/sbin /mtd6/boot/sbin         # WAJIB

# Anti-watchdog (opsional tapi disarankan):
printf '#!/bin/sh\nif [ -f /proc/net/monitor ]; then echo 0 > /proc/net/monitor; fi\nexit 0\n' > /mtd6/etc/rc.local
chmod +x /mtd6/etc/rc.local

sync
umount /test
umount /mtd6
# Reboot → U-Boot → ketik 'norm'
```

---

## ⚠️ KESALAHAN YANG DILAKUKAN (dan pelajarannya)

### ❌ JANGAN buat `/init` → busybox!

Percobaan pertama GAGAL dengan:
```
init: applet not found
Kernel panic - not syncing: Attempted to kill init!
```

**Penyebab**: `/init` → symlink ke `bin/busybox`. Kernel menjalankan `/init` sebagai argv[0]="init", tapi **busybox LEDE tidak punya applet "init"** (LEDE pakai `procd`, bukan busybox-init).

**Solusi**: **JANGAN buat `/init`**. Biarkan kernel fallback otomatis:
```
init=/init → tidak ada? → fallback ke /sbin/init (procd) → BOOT ✅
```

Kernel Linux 2.6.35 punya fallback berurutan:
`/init` → `/sbin/init` → `/etc/init` → `/bin/init` → `/bin/sh`

Blog TIDAK menyebut `/init` karena memang tidak perlu. Saya menambahkannya = kesalahan.

---

## Perbandingan: Blog vs Implementasi Kita

| Langkah | Blog | Kita (final) |
|---------|------|--------------|
| mount mtd6 | ✅ | ✅ |
| `rm -rf /mtd6/*` | ✅ | ✅ |
| `cp -rp /test/* /mtd6` | ✅ | ✅ |
| `cp /boot/sbin` | ✅ | ✅ |
| buat `/init` | ❌ tidak ada | ❌ **dihapus** (kunci sukses) |
| rc.local anti-watchdog | (komentar) | ✅ ditambahkan |

---

## Recovery yang Terbukti Berfungsi

Saat percobaan pertama gagal (kernel panic), recovery BERHASIL:

1. **Power-cycle** STB
2. **Spam Enter** untuk tangkap U-Boot
3. Ketik **`safe`** → boot ke slot safe (asli) → **STB hidup lagi**

**Bukti failsafe bekerja**: STB gagal boot norm → pulih dalam <1 menit.

---

## Kondisi Sekarang

| Partisi | Isi | Status |
|---------|-----|--------|
| mtd1 (boot) | bootloader asli | ✅ Aman (tidak disentuh) |
| mtd5 (kernel1) | kernel MediaTek | ✅ Asli |
| **mtd6 (rootfs1)** | **LEDE 17.01.6** | ✅ **Jalan** |
| mtd9 (rootfs2/safe) | rootfs asli ZTE | ✅ Utuh (penyelamat) |
| Backup | 10 partisi | ✅ Di PC + SD |

**Slot SAFE (mtd9) masih ASLI** → kalau LEDE bermasalah, ketik `safe` di U-Boot.

---

## Cara Boot ke LEDE vs ZTE Asli

- **LEDE (norm)**: di U-Boot ketik `norm`
- **ZTE asli (safe)**: di U-Boot ketik `safe`

Kalau tidak menekan apa-apa, default boot ke... (perlu cek `bootcmd`/`bootsys`).

---

## Catatan Penting

1. **Video/HDMI**: Kemungkinan tidak tampil (LEDE tidak punya driver video MediaTek). Akses via UART/SSH.
2. **Modul kernel**: Hanya modul MediaTek yang jalan. Modul LEDE (kmod untuk 4.4.x) tidak kompatibel dengan 2.6.35.
3. **Watchdog**: `/proc/net/monitor` — sudah di-disable via rc.local.
4. **SSH**: dropbear tersedia, tapi belum ada password → set dengan `passwd`.
