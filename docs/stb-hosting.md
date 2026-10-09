# Hosting Web Statis di STB

Menjalankan STB (ZTE B700V5S1, LEDE 17.01.6) sebagai **web server** untuk
situs statis (HTML/CSS/JS). Sudah dikonfigurasi & berfungsi.

## Ringkas

| Item | Nilai |
|------|-------|
| Web server | `uhttpd` (sudah terpasang, jalan otomatis) |
| Document root | `/www` |
| Situs | `http://<IP-STB>/` |
| LuCI (admin) | `http://<IP-STB>/cgi-bin/luci` |
| Upload | SCP/SFTP (`pscp`) |
| Storage | rootfs 8 MB (~4.5 MB bebas) |

## Akses

```bash
# Dari laptop (langsung)
http://192.168.137.2/

# Dari PC lain di jaringan WiFi (via port forward laptop)
http://192.168.18.13:8080/
```

## Struktur di STB

```
/www/
├── index.html          <- halaman utama (ganti ini)
├── css/style.css       <- stylesheet
├── js/app.js           <- script
├── cgi-bin/luci        <- LuCI (JANGAN dihapus)
└── luci-static/        <- aset tema LuCI
```

**Penting**: LuCI memakai `/www` sebagai root, jadi situs ditaruh **langsung di
`/www`** (bukan subfolder). LuCI tetap aman di `/cgi-bin/luci` (namespace
terpisah). Jangan hapus `cgi-bin/` dan `luci-static/`.

## Upload konten

### Cara 1: script (gampang)

```powershell
cd scripts\stb-sim
.\stb-upload.ps1                          # upload folder .\site ke /www
.\stb-upload.ps1 -Source .\site -StbIP 192.168.137.2
```

### Cara 2: manual (pscp)

```powershell
# Upload 1 file
pscp -scp -batch -hostkey SHA256:gJ3MLICoJUPa9JZ9CxbE6wi5MPMukFuhfDT6zoiiRfg `
     -pw UartRecon2026 index.html root@192.168.137.2:/www/index.html

# Upload folder (rekursif)
pscp -scp -batch -r -hostkey <fingerprint> -pw UartRecon2026 .\site root@192.168.137.2:/www/
```

> **Fingerprint host key** didapat saat pertama konek (lihat pesan plink).
> Kalau beda, ganti sesuai output `plink` Anda.

### Cara 3: dari dalam STB (wget)

```sh
cd /www
wget -O index.html http://contoh.com/index.html
```

## Konfigurasi uhttpd (referensi)

```sh
uci show uhttpd.main
# home        = /www        (document root)
# cgi_prefix  = /cgi-bin    (WAJIB relatif ke home, kalau absolut -> LuCI 403)
# index_page  = index.html
# listen_http = 0.0.0.0:80
```

Terapkan & restart:

```sh
uci set uhttpd.main.home='/www'
uci set uhttpd.main.cgi_prefix='/cgi-bin'
uci set uhttpd.main.index_page='index.html'
uci commit uhttpd
/etc/init.d/uhttpd restart
```

## Batasan & tips

- **Ruang kecil**: rootfs hanya 8 MB. Untuk situs besar/gambar banyak, pertimbangkan:
  - Simpan di `/tmp` (RAM 180 MB, hilang saat reboot)
  - Mount SD/USB lalu symlink ke `/www`
  - Partisi `data` (mtd12, 69 MB) - tapi berformat UBI, perlu riset
- **Statis saja**: `uhttpd` bisa CGI, tapi PHP/Python tidak terpasang.
  Feed menyediakan `php7`/`python3` bila perlu (perhatikan ruang).
- **HTTPS**: uhttpd sudah listen di 443 dengan sertifikat self-signed
  (`/etc/uhttpd.crt`). Bisa diganti dengan sertifikat asli (Let's Encrypt)
  kalau nanti pakai domain.
- **Domain nanti**: saat pakai domain, arahkan DNS ke IP publik, lalu
  port-forward 80/443 dari router ke laptop (yang meneruskan ke STB), atau
  pakai tunnel (Cloudflare Tunnel / Tailscale Funnel) agar tidak perlu IP publik.

## Untuk domain (rencana)

Karena STB di belakang NAT (lihat `docs/stb-lan-share.md`), ada 2 jalur:

| Metode | Kelebihan | Kekurangan |
|--------|-----------|------------|
| Port forward router -> laptop -> STB | Sederhana | Butuh IP publik; laptop & STB harus nyala |
| Tunnel (Cloudflare/Tailscale/ngrok) | Tak perlu IP publik, bisa HTTPS | Tergantung layanan pihak ketiga |

Alur port forward: `domain:80` -> router -> `192.168.18.13:80` (laptop) ->
`192.168.137.2:80` (STB). Port forwarding laptop sudah ada (lihat
`scripts/stb-sim/stb-share-lan.ps1`); tinggal tambah rule untuk port 80.

## Verifikasi

```powershell
# Situs
curl http://192.168.137.2/

# CSS/JS
curl http://192.168.137.2/css/style.css

# LuCI masih hidup?
curl http://192.168.137.2/cgi-bin/luci
```
