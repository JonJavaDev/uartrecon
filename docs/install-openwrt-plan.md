# Riset: Install OpenWrt/LEDE Permanen ke ZTE B700V5

**Status**: RISET & RENCANA — belum dieksekusi
**Tanggal**: 2026-10-08
**Device**: ZTE B700V5S1 (MediaTek MT8653, ARM1176JZF-S / ARMv6, NAND 256MB, RAM 512MB)

---

## 1. Fakta Terverifikasi

### 1.1 Partisi & Mode
```
mtd5 kernel1 (5MB)  ┐
mtd6 rootfs1 (8MB)  ├─ SLOT NORM
mtd7 app1    (65MB) ┘
mtd8 kernel2 (5MB)  ┐
mtd9 rootfs2 (8MB)  ├─ SLOT SAFE  ← SEDANG DIPAKAI
mtd10 app2   (65MB) ┘
mtd1 boot (bootloader)  ← JANGAN SENTUH (brick)
```

### 1.2 Ukuran LEDE — MUAT di 8MB
- `openwrt-gemini-b700.squashfs` = 1.84 MB (compressed, gzip)
- Setelah diekstrak: **~4.5 MB nyata** (dedup hardlink busybox)
- Ukuran "naif" 48 MB itu salah hitung: 114 file adalah hardlink ke busybox
- **Kesimpulan: MUAT di rootfs 8MB** ✅

### 1.3 Kompatibilitas — TERUJI
- chroot LEDE 17.01.6 berhasil jalan di STB (sudah dites)
- Arsitektur: ARM soft-float EABI5, musl (cocok ARMv6 STB)
- Kernel TIDAK diubah (tetap MediaTek 2.6.35)

### 1.4 Fakta Krusial dari Blog & Komunitas
| Fakta | Detail |
|-------|--------|
| Kernel tetap | `root=/dev/mtdblock6/9 rootfstype=jffs2 init=/init` |
| `/init` STB | symlink ke `bin/busybox` |
| `/boot/sbin` | File wajib (320KB), asal `/mnt/app/mtk_modules/sbin` |
| Watchdog | `echo 0 > /proc/net/monitor` (kalau reboot melulu) |
| copy | HARUS `cp -rp /test/* /mtd6` (pakai `*`) |
| hapus | HARUS `rm -rf /mtd6/*` (bukan `rm -rf /mtd6`) |
| squashfs | WAJIB gzip (xz tidak didukung) |

---

## 2. Rencana Install (BELUM DIEKSEKUSI)

### Strategi: Timpa SLOT NORM (mtd6), biarkan SLOT SAFE utuh

Karena sekarang boot dari **SAFE (mtd9)**, kita timpa **NORM (mtd6)** yang dorman.
Kalau gagal → boot kembali ke SAFE → masih hidup.

### Langkah

```sh
# 0. PRA: pastikan SD card ter-mount
mount | grep usbb        # /dev/sdb1 on /var/mntt/usbb1

# 1. Mount rootfs NORM (mtd6) yang akan ditimpa
mkdir -p /mtd6
mount -t jffs2 /dev/mtdblock6 /mtd6

# 2. Backup cepat rootfs lama (opsional, sudah ada di PC)
# dd if=/dev/mtd6 of=/var/mntt/usbb1/uartrecon_backup/mtd6_preinstall.bin

# 3. Mount OS baru
mkdir -p /test
mount -t squashfs /var/mntt/usbb1/openwrt-gemini-b700.squashfs /test

# 4. Hapus isi rootfs lama
rm -rf /mtd6/*

# 5. Copy OS baru
cp -rp /test/* /mtd6

# 6. File WAJIB: /boot/sbin
mkdir -p /mtd6/boot
cp /boot/sbin /mtd6/boot/sbin

# 7. Set /init (kernel cari init=/init)
# LEDE punya /sbin/init; STB cari /init
# -> perlu verifikasi: apakah LEDE perlu symlink /init?

# 8. Sync & verifikasi
sync
ls -la /mtd6/
ls -la /mtd6/boot/sbin

# 9. Reboot ke NORM (dari U-Boot: ketik 'norm')
```

---

## 3. Risiko & Mitigasi

| Risiko | Tingkat | Mitigasi |
|--------|---------|----------|
| mtd6 rusak | Rendah | Backup ada; boot ke SAFE |
| mtd1 tersentuh | **FATAL** | TIDAK menyentuh mtd1 sama sekali |
| LEDE tidak boot (init) | Sedang | Boot balik ke SAFE, restore mtd6 |
| Watchdog reboot loop | Sedang | `echo 0 > /proc/net/monitor` via rc.local |
| Tidak ada output video | Tinggi | Diketahui; akses tetap via UART |
| Modul kernel tidak cocok | Tinggi | Batasan; hanya modul MediaTek yang jalan |

---

## 4. Pertanyaan yang Masih Perlu Jawaban

1. **`/init`**: Kernel boot dengan `init=/init`. LEDE punya `/sbin/init`, tidak ada `/init`.
   - Apakah perlu buat `/mtd6/init` → symlink ke `sbin/init`?
   - Atau kernel STB punya fallback?

2. **Slot mana yang ditimpa?** NORM (mtd6) atau SAFE (mtd9)?
   - Menimpa NORM = lebih aman (SAFE jadi penyelamat)
   - Tapi NORM bukan slot aktif sekarang

3. **Backup mtd6 terbaru**: Perlu re-dump sebelum timpa (untuk jaga-jaga).

---

## 5. Sumber
- Blog: https://kujadi-tahu.blogspot.com/2019/07/mengobrak-abrik-stb-jadul-zte-b700v5.html
- GitHub: ndunks/STB-ZTE-B700V5
- OpenWrt archive: gemini/wiligear LEDE 17.01.6
