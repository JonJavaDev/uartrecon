# Optimasi LEDE di STB B700V5

Catatan setup LEDE 17.01.6 yang sudah dioptimalkan di STB.

## Yang sudah dikonfigurasi

| Item | Nilai |
|------|-------|
| Hostname | `uartrecon` |
| IP LAN | `192.168.1.1/24` (eth0) |
| Password root | `UartRecon2026` (GANTI kalau perlu) |
| SSH | dropbear (port 22), auto-start |
| Web UI | LuCI (uhttpd, port 80), auto-start |
| DHCP/DNS | dnsmasq, auto-start |
| Firewall | iptables, auto-start |

## Layanan yang jalan

```
/usr/sbin/dropbear   ← SSH server
/usr/sbin/dnsmasq    ← DHCP + DNS
/usr/sbin/uhttpd     ← web server (LuCI)
```

## Cara akses

### Via Ethernet (butuh kabel LAN)

1. Colok kabel LAN ke port eth0 STB
2. Set PC ke DHCP (atau IP static 192.168.1.2/24)
3. Akses:
   - **SSH**: `ssh root@192.168.1.1` (password: `UartRecon2026`)
   - **Web**: `http://192.168.1.1` (LuCI)

### Via UART (selalu bisa)

```bash
uartrecon terminal COM3
```

## Perintah yang dipakai

```sh
# Hostname
uci set system.@system[0].hostname='uartrecon'
uci commit system

# Enable layanan (auto-start)
/etc/init.d/dropbear enable
/etc/init.d/uhttpd enable
/etc/init.d/dnsmasq enable
/etc/init.d/firewall enable

# Set password root
printf 'UartRecon2026\nUartRecon2026\n' | passwd root

# Start sekarang
/etc/init.d/dropbear start
/etc/init.d/uhttpd start
/etc/init.d/dnsmasq start
```

## Storage

- Rootfs LEDE: 8 MB (5 MB bebas) — **sempit**, hati-hati install package
- `/tmp`: 180 MB (RAM, hilang saat reboot)
- Partisi `data` (mtd12, 70 MB): berisi UBI — belum bisa di-mount
  karena LEDE tidak punya driver UBI. Perlu riset lanjut.

## Install package (opkg)

```sh
opkg update
opkg install <package>
```

⚠️ Rootfs cuma 5 MB bebas. Cek dulu ukuran package sebelum install.

## Catatan

- LuCI = web interface lengkap (bisa setup network, firewall, dll via browser)
- SSH = akses terminal jarak jauh
- Untuk jadi router: tinggal atur WAN/LAN di LuCI
- eth0 carrier=0 kalau kabel LAN belum dicolok (normal)
