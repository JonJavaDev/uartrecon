# Recovery Playbook — UARTRecon

Panduan lengkap failsafe & recovery untuk perangkat embedded (fokus: STB
MediaTek MT85xx seperti ZTE B700V5). **Baca sebelum melakukan operasi tulis.**

---

## Prinsip: Fail-Safe Berlapis

```
Layer 1: BACKUP partisi kritis (SEBELUM apa pun)          ← wajib
Layer 2: UARTRecon hard-block (cegah perintah destruktif)
Layer 3: Dual-boot A/B (norm ↔ safe)                      ← bawaan device
Layer 4: USB BROM recovery (kasus bootloader hilang)      ← jalur terakhir
```

**Aturan emas**: Jangan pernah menulis ke partisi kritis sebelum Layer 1 selesai.

---

## Klasifikasi Partisi (ZTE B700V5)

| mtd | Nama | Kritis | Rusak = ? | Recovery |
|-----|------|--------|-----------|----------|
| 1 | boot | 🔴 CRITICAL | **BRICKED** (butuh USB BROM) | Hampir mustahil via UART |
| 2 | env | 🟠 HIGH | bad CRC, boot tak terkendali | Restore / `saveenv` |
| 3 | conf | 🟡 MEDIUM | config hilang | Restore backup |
| 4 | logo | 🟡 MEDIUM | logo hilang | Restore backup |
| 5/8 | kernel1/2 | 🟠 HIGH | boot gagal | Switch slot / restore |
| 6/9 | rootfs1/2 | 🟠 HIGH | rootfs gagal mount | Switch slot / restore |
| 7/10 | app1/2 | 🟡 MEDIUM | app gagal | Restore |
| 11/12 | vas/data | 🟢 LOW | data user hilang | Tidak fatal |

> **Penting**: Hanya `mtd1` (bootloader) yang benar-benar fatal. Partisi lain
> bisa di-recover selama U-Boot masih hidup.

---

## FASE 0 — Backup (WAJIB PERTAMA)

### Kenapa via SD card, bukan UART?

| Metode | Kecepatan | Keandalan |
|--------|-----------|-----------|
| UART hexdump | ~0.2 KB/s | Rawan corrupt |
| **SD card (dd)** | **~7 MB/s** | Andal + verifikasi MD5 |

Transfer 166 MB via UART = ~9 hari. Via SD card = ~25 detik. **Selalu pakai SD card.**

### Langkah Backup

1. **Siapkan SD card FAT32** (device hanya baca vfat/ntfs/xfs, bukan ext4/exFAT).

2. **Colok SD card ke device**, verifikasi ter-mount:
   ```sh
   mount | grep usb        # /dev/sda1 on /var/mntt/usba1 type vfat
   ```

3. **Backup via UARTRecon** (generate script):
   ```bash
   uartrecon recover plan --device B700V5S1 --out backups/
   ```

4. **Jalankan backup** (kirim perintah via UART, data langsung ke SD):
   ```bash
   uartrecon recover run --port COM3 --device B700V5S1
   ```

   Atau manual per partisi:
   ```sh
   mkdir -p /var/mntt/usba1/uartrecon_backup
   dd if=/dev/mtd1 of=/var/mntt/usba1/uartrecon_backup/mtd1_boot.bin bs=64k
   dd if=/dev/mtd2 of=/var/mntt/usba1/uartrecon_backup/mtd2_env.bin bs=64k
   # ... dst
   cd /var/mntt/usba1/uartrecon_backup && md5sum mtd*.bin > MANIFEST.md5
   sync
   ```

5. **Verifikasi di device**:
   ```sh
   md5sum -c MANIFEST.md5    # semua harus OK
   ```

6. **Cabut SD, colok ke PC**, copy ke tempat aman & verifikasi ulang.

---

## FASE 1 — Recovery via UART + SD card

Jika kernel/rootfs/app/conf rusak:

```sh
# Di device (boot ke slot yang masih hidup), SD card ter-mount:
flash_erase /dev/mtd5 0 0
dd if=/var/mntt/usba1/uartrecon_backup/mtd5_kernel1.bin of=/dev/mtd5 bs=64k
sync
reboot
```

⚠️ **JANGAN** restore mtd1 (bootloader) via cara ini kecuali darurat.

---

## FASE 2 — Recovery via U-Boot

Masuk U-Boot: nyalakan device, **tekan Enter berulang** saat muncul banner U-Boot.

```
# Lihat environment
printenv

# Ganti slot boot (paling aman)
setenv system norm
saveenv
reset

# Atau langsung boot slot lain
safe
# atau
norm
```

Restore via TFTP (kalau U-Boot punya jaringan):
```
setenv ipaddr 192.168.1.100
setenv serverip 192.168.1.10
tftpboot 0x80000000 mtd5_kernel1.bin
nand erase 0x500000 0x500000
nand write 0x80000000 0x500000 0x500000
```

---

## FASE 3 — Recovery via USB BROM (bootloader hilang)

**Hanya untuk kasus terparah** (mtd1 rusak / U-Boot tidak muncul sama sekali).
UART **tidak berguna** di sini karena tidak ada yang mendengarkan.

### Prasyarat

- **USB BROM mode**: MediaTek MT85xx punya BROM yang bisa diakses via USB
- **Tool**: `mtkclient` (open source) atau SP Flash Tool (Windows)
- **Driver**: MediaTek USB VCOM driver
- **Backup mtd1**: Untuk restore, Anda butuh `mtd1_boot.bin` (dari backup!)

### Cara masuk BROM mode (umum pada MTK)

1. Matikan device total (cabut power)
2. Hubungkan USB (bukan UART) ke PC
3. Tahan tombol / short pin BROM tertentu saat power-on
4. PC mendeteksi device sebagai "MediaTek USB Port" / "MTK BROM"

### Restore dengan mtkclient

```bash
pip install mtkclient
python mtk w boot1 mtd1_boot.bin   # tulis bootloader
python mtk reset
```

> ⚠️ Prosedur BROM spesifik per SoC. Konsultasi dokumentasi MT85xx / komunitas.

---

## Checklist Sebelum Eksperimen

- [ ] Backup SEMUA partisi kritis sudah dibuat & terverifikasi MD5
- [ ] Backup tersimpan di **2 tempat** (SD card + PC/cloud)
- [ ] Sudah paham partisi mana yang fatal (mtd1)
- [ ] Tahu cara masuk U-Boot (tekan Enter saat boot)
- [ ] Tahu slot mana yang aktif (`echo $system`)
- [ ] Sudah baca skema dual-boot (norm ↔ safe)
- [ ] UARTRecon safety hard-mode aktif

---

## Perintah yang DIBLOKIR UARTRecon (hard-mode)

```
flash_erase /dev/mtd1     → BLOCKED (bootloader)
flash_erase /dev/mtd2     → BLOCKED (env)
dd of=/dev/mtd5           → BLOCKED (kernel)
dd of=/dev/mtd6           → BLOCKED (rootfs)
nand write ... /dev/mtd8  → BLOCKED (kernel safe)
setenv bootargs ...       → WARN
```

Cek command apa pun:
```bash
uartrecon safety --check "flash_erase /dev/mtd1 0 1"
# ⛔ BLOCKED — menulis ke mtd1 (boot) DIBLOKIR [CRITICAL]
```

---

## Ringkasan Cepat

| Situasi | Tindakan |
|---------|----------|
| Config salah | Restore mtd3 dari backup |
| Kernel/rootfs rusak | Switch slot A/B, atau restore |
| App rusak | Restore mtd7/mtd10 |
| Env U-Boot korup | `saveenv` di U-Boot, atau restore mtd2 |
| **Bootloader rusak** | **USB BROM + backup mtd1** (UART tak cukup) |

**Selama U-Boot masih hidup, Anda masih punya kendali. Jaga mtd1.**
