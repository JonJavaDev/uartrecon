# STB via LAN: Laptop sebagai Router + NAT

Panduan menghubungkan STB (ZTE B700V5S1 / LEDE) ke laptop lewat kabel LAN,
lalu **membuat STB bisa mengakses PC lain di jaringan & Internet** — dan
sebaliknya.

## Topologi

```
        Internet / WiFi LAN (192.168.18.0/24)
                     |
              [ Router WiFi 192.168.18.1 ]
                     |
              PC lain (192.168.18.6, .25, ...)
                     |
        ┌────────────┴────────────┐
        |  LAPTOP                 |
        |  WiFi  192.168.18.13    |  <- uplink (Internet)
        |  Eth   192.168.137.1    |  <- ke STB (NAT + ICS)
        └────────────┬────────────┘
                     | kabel LAN
              [ STB 192.168.137.2 ]  (eth0)
```

Laptop berperan sebagai **router + NAT** antara kabel LAN (STB) dan WiFi.

## Hasil akhir

| Arah | Hasil |
|------|-------|
| STB -> laptop | ✅ ping 192.168.137.1 |
| STB -> PC jaringan | ✅ ping 192.168.18.6 |
| STB -> gateway WiFi | ✅ ping 192.168.18.1 |
| STB -> Internet | ✅ ping 8.8.8.8 |
| STB -> DNS | ✅ nslookup openwrt.org |
| PC lain -> SSH STB | ✅ `ssh -p 2222 root@192.168.18.13` |
| PC lain -> LuCI STB | ✅ `http://192.168.18.13:8080/` |

## Sisi Windows (laptop)

Jalankan script (butuh **Administrator**):

```powershell
cd scripts\stb-sim
.\stb-share-lan.ps1
```

Script ini melakukan:
1. `IPEnableRouter = 1` (IP forwarding)
2. **ICS**: WiFi = *public* (sumber Internet), Ethernet = *private* (ke STB)
3. Pastikan service `SharedAccess` jalan
4. **Port forwarding** (netsh portproxy) + firewall rule:
   - `192.168.18.13:2222 -> 192.168.137.2:22` (SSH)
   - `192.168.18.13:8080 -> 192.168.137.2:80` (LuCI)

## Sisi STB (LEDE)

Set IP + gateway + DNS. Untuk **permanen** (tahan reboot):

```sh
uci set network.lan.type=none          # WAJIB: modul bridge tidak ada di kernel STB
uci set network.lan.ifname=eth0
uci set network.lan.proto=static
uci set network.lan.ipaddr=192.168.137.2
uci set network.lan.netmask=255.255.255.0
uci set network.lan.gateway=192.168.137.1
uci set network.lan.dns=192.168.137.1
uci commit network
/etc/init.d/network restart
```

Sementara (tanpa simpan):

```sh
ifconfig eth0 192.168.137.2 netmask 255.255.255.0 up
route add default gw 192.168.137.1
echo "nameserver 192.168.137.1" > /etc/resolv.conf
```

## Catatan penting

- **`network.lan.type=none` wajib.** Konfigurasi bawaan memakai `type='bridge'`,
  tetapi modul `bridge` tidak ada di kernel 2.6.35 STB ini, sehingga `br-lan`
  tidak pernah terbentuk dan STB **tidak punya IP**. Karena itu `type` diset
  `none` dan IP dipasang langsung di `eth0`.
- **Subnet harus sama.** Windows ICS memakai `192.168.137.1/24`, jadi STB harus
  di `192.168.137.x`. Kalau STB tetap di `192.168.1.1` (bawaan), keduanya tidak
  bisa saling ping.
- **Port forwarding** dipakai karena STB ada di belakang NAT laptop — dari
  jaringan WiFi, `192.168.137.2` tidak terlihat. PC lain mengakses lewat IP
  WiFi laptop + port forward.
- **USB-to-USB tidak bisa** untuk STB ini: port USB-nya *host-only* (tidak ada
  USB Device Controller / modul gadget di kernel 2.6.35). LAN adalah satu-satunya
  jalur jaringan (selain UART).

## Akses

```bash
# Dari laptop (langsung)
ssh root@192.168.137.2
curl http://192.168.137.2/

# Dari PC lain di jaringan WiFi (via laptop)
ssh -p 2222 root@192.168.18.13
# buka http://192.168.18.13:8080/  (LuCI)
```

Password SSH default (dari `docs/lede-optimization.md`): `UartRecon2026`.

> Catatan SSH: dropbear LEDE hanya menawarkan `ssh-rsa` (lama). OpenSSH modern
> perlu `-o HostKeyAlgorithms=+ssh-rsa -o PubkeyAcceptedAlgorithms=+ssh-rsa`,
> atau pakai PuTTY/plink.
