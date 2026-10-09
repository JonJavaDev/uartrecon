# Set LEDE sebagai Boot Default (Permanen)

Cara membuat STB selalu boot ke LEDE (slot norm), tapi tetap aman kalau
LEDE corrupt (auto-fallback ke safe).

## Ringkasan

Bootloader MediaTek (`bootsys`) **sudah punya auto-fallback bawaan**:

```
boot system norm failed, try boot safe!
boot system safe failed, try boot norm!
```

Jadi cukup set environment `system=norm` permanen. Kalau LEDE gagal boot,
bootloader **otomatis** coba slot safe. Tidak perlu logika tambahan.

## Yang Dibutuhkan

- Akses UART (fitur `uartrecon uboot`)
- Device dalam keadaan bisa masuk U-Boot

## Langkah

```bash
uartrecon uboot COM3 --reboot --send "setenv system norm; saveenv; printenv"
```

Atau manual di prompt U-Boot:

```
STB-BOOT # setenv system norm
STB-BOOT # saveenv
STB-BOOT # printenv
```

Verifikasi: output `printenv` harus menampilkan `system=norm`.

## Yang Disentuh / Tidak

| Partisi | Isi | Disentuh? |
|---------|-----|-----------|
| mtd1 | bootloader | **TIDAK** (aman) |
| mtd2 | env U-Boot | ✅ (hanya `system=norm`) |
| mtd5/mtd8 | kernel | **TIDAK** (aman) |
| mtd6 | rootfs norm (LEDE) | TIDAK (hanya boot) |
| mtd9 | rootfs safe (ZTE) | TIDAK (fallback) |

Operasi ini **hanya mengubah satu variabel** di environment (mtd2).
Bootloader & kernel tidak disentuh.

## Failback Kalau LEDE Corrupt

Kalau LEDE (mtd6) rusak sehingga gagal boot, `bootsys` otomatis:
1. Coba boot norm (mtd5+mtd6) → gagal
2. **Otomatis** coba boot safe (mtd8+mtd9) → ZTE asli → hidup

Kalau mau manual: di U-Boot ketik `safe`.

## Kembalikan ke Default ZTE (safe)

```
STB-BOOT # setenv system safe
STB-BOOT # saveenv
```

Atau hapus variabelnya:
```
STB-BOOT # setenv system
STB-BOOT # saveenv
```

## Catatan

- `saveenv` menulis ke NAND (mtd2) — proses normal, aman.
- Backup mtd2 sudah ada di `backups/B700V5S1/mtd2_env.bin`.
- Kalau env korup lagi (bad CRC), STB balik ke default → tinggal set ulang.
