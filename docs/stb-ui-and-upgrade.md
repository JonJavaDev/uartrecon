# STB: Tampilan Modern & Analisis Upgrade

Catatan tentang "tampilan jadul" dan peluang upgrade kernel/userspace pada
STB ZTE B700V5S1 yang menjalankan LEDE 17.01.6.

## Ringkas

| Pertanyaan | Jawaban |
|------------|---------|
| STB ini pakai apa? | **LEDE 17.01.6** = **OpenWrt 17.01.6** (LEDE adalah nama OpenWrt 2016-2018) |
| Tampilan jadul bisa diperbaiki? | **Ya, mudah** - ganti tema LuCI (tanpa sentuh kernel) |
| Kernel bisa di-upgrade? | **Tidak praktis** - lihat alasan di bawah |
| Userspace bisa di-upgrade? | **Terbatas** - lihat "Setup hybrid" |

## 1. "Tampilan jadul" = tema LuCI, bukan kernel

Tampilan web LuCI diatur oleh **tema**, bukan kernel. STB memakai tema lama:

```
luci-theme-bootstrap - git-18.201.27126-7bf0367-1   (2018)
```

### Solusi: pasang tema modern (aman, sudah dilakukan)

```sh
opkg update
opkg install luci-theme-material

uci set luci.main.mediaurlbase='/luci-static/material'
uci set luci.themes.Material='/luci-static/material'
uci commit luci
/etc/init.d/uhttpd restart
```

Tema yang tersedia di feed:

| Paket | Ukuran | Tampilan |
|-------|--------|----------|
| `luci-theme-material` | 51 KB | Modern, ala Material Design |
| `luci-theme-openwrt` | 7.7 KB | Tema OpenWrt modern |
| `luci-theme-freifunk-generic` | - | Freifunk |

**Verifikasi**: HTML LuCI sekarang memuat
`/luci-static/material/css/style.css` (bukan `bootstrap`).

Untuk kembali ke tema lama:
```sh
uci set luci.main.mediaurlbase='/luci-static/bootstrap'
uci commit luci; /etc/init.d/uhttpd restart
```

### Aplikasi tambahan (opsional)

Feed menyediakan **799 paket `luci-app-*`**. Yang berguna & kecil:

```sh
opkg install luci-app-commands     # jalankan shell command dari web
opkg install luci-app-statistics   # grafik CPU/RAM/network (collectd)
opkg install luci-app-wol          # Wake-on-LAN
opkg install luci-app-ddns         # dynamic DNS
```

> **Hati-hati ruang**: rootfs hanya **8 MB** (~4.5 MB bebas). Selalu cek
> `df -h /` sebelum memasang paket besar.

## 2. Kenapa upgrade kernel TIDAK praktis

### Kernel STB itu khusus MediaTek

```
uname -r         -> 2.6.35
/proc/version    -> Linux version 2.6.35 ... (gcc 4.5.1) 2014
CPU              -> ARM1176JZF-S (ARMv6K), CPU part 0xb76
Hardware         -> mt85xx (MediaTek MT8653)
```

- Kernel **2.6.35 ini adalah kernel bawaan pabrik MediaTek** yang dikompilasi
  dengan **driver NAND/video/PHY MediaTek yang tidak open source**.
- Kernel generik (5.x/6.x) **tidak punya driver** untuk NAND & hardware STB ini.
- Kernel & rootfs ada di **partisi MTD terpisah**:
  ```
  mtd5 kernel1 (5MB)  mtd6 rootfs1 (8MB)   <- slot NORM
  mtd8 kernel2 (5MB)  mtd9 rootfs2 (8MB)   <- slot SAFE
  mtd1 boot (bootloader)                    <- JANGAN SENTUH (brick)
  ```
- Menimpa kernel = mengubah alur bootloader -> risiko **brick**. Kalau `mtd1`
  rusak, STB mati permanen (butuh USB BROM untuk pulih).

### Target `gemini` bukan untuk STB ini

OpenWrt punya target `gemini/generic` (18.06 s/d 24.10), tetapi itu untuk
device seperti **D-Link DIR-685, Itian SQ201, Storlink SL93512R** yang memakai
CPU **ARMv4/ARM926 (fa526)** — **berbeda** dari CPU STB (ARMv6).
Image itu **tidak kompatibel**.

## 3. Setup "hybrid" yang sudah ada di STB

Ada temuan penting soal STB ini:

```
Kernel berjalan     : 2.6.35
Modul LEDE tersedia : /lib/modules/4.4.153/   <- dibangun untuk kernel 4.4!
```

Artinya rootfs LEDE ini **aslinya dibangun untuk kernel 4.4**, tapi dijalankan
di atas **kernel MediaTek 2.6.35**. Ini setup **hybrid**:

- Userspace (busybox, LuCI, dropbear) = LEDE 17.01.6, kompatibel kernel 2.6+.
- Modul kernel (`kmod-*`) versi **4.4.153** -> **TIDAK BISA dimuat** ke 2.6.35.

Bukti: `modprobe bridge` gagal (`rc=255`), dan tidak ada `/lib/modules/2.6.35`.
Inilah sebabnya fitur yang butuh modul kernel (bridge, dst.) tidak jalan di
STB ini - termasuk kenapa `network.lan.type='bridge'` gagal (lihat
`docs/stb-lan-share.md`).

**Konsekuensi**: upgrade rootfs ke versi LEDE/OpenWrt yang lebih baru **tidak
akan** memberi fitur kernel baru (modul tetap tak kompatibel). Yang bisa
di-upgrade hanya aplikasi userspace yang murni berjalan di atas 2.6.35.

## 4. Kesimpulan

| Ingin | Cara | Risiko |
|-------|------|--------|
| Tampilan modern | Ganti tema LuCI (material/openwrt) | Nyaris nol |
| Aplikasi web baru | `opkg install luci-app-*` (cek ruang!) | Rendah |
| Fitur kernel baru (bridge, dll.) | **Tidak bisa** tanpa kernel baru | - |
| Kernel 5.x/6.x di hardware | Butuh port driver MediaTek | Sangat tinggi (brick) |
| Lihat kernel baru jalan | Simulasi QEMU (`docs/stb-sim-modern-kernel.md`) | Nol |

**Rekomendasi**: tetap pakai kernel 2.6.35 MediaTek. Perbaiki tampilan & fitur
lewat tema/aplikasi LuCI saja. Untuk eksperimen kernel modern, gunakan simulasi
QEMU yang sudah dibuat.

## Catatan

- Backup config sebelum mengubah: `cp /etc/config/luci /etc/config/luci.bak`
- Script pengaturan ada di `scripts/stb-sim/` (untuk share LAN, dll.)
- Akses LuCI: `http://192.168.137.2/` (dari laptop) atau
  `http://192.168.18.13:8080/` (dari PC lain, via port forward)
