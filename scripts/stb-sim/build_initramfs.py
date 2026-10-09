#!/usr/bin/env python3
"""Bangun initramfs (cpio newc + gzip) dari rootfs Alpine armhf.

Dijalankan di Windows, di mana pembuatan symlink/hardlink native sering
diblokir (butuh Developer Mode / admin). Karena itu kita TIDAK mengekstrak
ke disk: kita baca tar.gz Alpine lalu menulis ulang menjadi arsip cpio `newc`
dengan symlink & hardlink yang BENAR. Kernel Linux di dalam emulasi yang
akan mengekstraknya, sehingga symlink valid.

Penggunaan:
    python build_initramfs.py <alpine-minirootfs.tar.gz> <out.cpio.gz> [hostname]
"""

import gzip
import io
import os
import sys
import tarfile
from collections import defaultdict

NEWC = b"070701"


def pad4(n: int) -> int:
    return (4 - (n % 4)) % 4


def norm(name: str) -> str:
    name = name.replace("\\", "/")
    while name.startswith("./"):
        name = name[2:]
    return name


def cpio_entry(name: str, mode: int, ino: int, nlink: int, uid: int, gid: int,
               mtime: int, data: bytes, devmajor: int = 0, devminor: int = 0,
               rdevmajor: int = 0, rdevminor: int = 0) -> bytes:
    """Satu entri cpio format `newc` (SVR4, tanpa checksum)."""
    nbytes = name.encode() + b"\x00"
    fields = [
        ino, mode, uid, gid, nlink, mtime, len(data),
        devmajor, devminor, rdevmajor, rdevminor, len(nbytes), 0,
    ]
    hdr = NEWC + b"".join(f"{f:08X}".encode() for f in fields)
    out = hdr + nbytes + b"\x00" * pad4(len(hdr) + len(nbytes))
    out += data + b"\x00" * pad4(len(data))
    return out


def build(src: str, dst: str, hostname: str) -> None:
    entries = []                    # (name, member, data_or_None)
    hardlinks = defaultdict(list)   # target -> [nama hardlink]
    ino_of = {}                     # nama -> ino
    nlink_of = {}                   # nama -> nlink
    next_ino = 1
    shadow_data = b""
    inittab_data = b""

    # ---- Pass 1: baca semua entri tar (beserta data file) ----
    with tarfile.open(src, "r:gz") as tf:
        for m in tf:
            name = norm(m.name)
            if name in ("", "."):
                continue
            data = None
            if m.isreg():
                data = tf.extractfile(m).read()
                ino_of[name] = next_ino
                next_ino += 1
                if name == "etc/shadow":
                    shadow_data = data
                if name == "etc/inittab":
                    inittab_data = data
            entries.append((name, m, data))
            if m.islnk():
                hardlinks[norm(m.linkname)].append(name)

    # Hitung nlink untuk file yang punya hardlink.
    for name, _, _ in entries:
        if name in ino_of:
            nlink_of[name] = 1 + len(hardlinks.get(name, []))
    for target, links in hardlinks.items():
        for lname in links:
            ino_of[lname] = ino_of.get(target, 0)
            nlink_of[lname] = nlink_of.get(target, 1)

    # ---- File kustom (override/tambahan) ----
    # PENTING (fix "sering mati"): JANGAN `exec getty` sebagai PID 1. Kalau getty
    # jadi PID 1 lalu keluar (logout / salah password 3x), PID 1 mati ->
    # kernel panic "Attempted to kill init!". STB asli punya init (procd /
    # busybox-init) yang RESPAWN getty. Kita tiru perilaku itu di sini.
    #
    # WATCHDOG: STB asli memakai watchdog firmware MediaTek yang dipantau lewat
    # /proc/net/monitor (hardware watchdog chip; kalau userspace berhenti
    # "menendang"-nya, chip memutus & reboot paksa mesin). Di emulasi QEMU,
    # watchdog hardware bcm2835-wdt TIDAK bisa dipakai (open() hang - celah
    # emulasi blok PM). Jadi kita emulasikan SEMANTIK-nya secara software:
    #   - ada daemon "firmware watchdog" yang menendang tiap 5 detik,
    #   - kalau tidak ditendang (state=hang) -> mesin REBOOT PAKSA (reboot -f).
    # Hasilnya identik dari sudut pandang pengamat: mesin benar-benar reset.
    init_script = f"""#!/bin/sh
# /init - PID 1 kustom UARTRecon STB simulation.
# Kernel yang dipakai = MODERN (bukan MediaTek 2.6.35).
export PATH=/sbin:/bin:/usr/sbin:/usr/bin
mount -t proc     proc     /proc     2>/dev/null
mount -t sysfs    sysfs    /sys      2>/dev/null
mount -t devtmpfs devtmpfs /dev      2>/dev/null
mount -t devpts   devpts   /dev/pts  2>/dev/null
mount -t tmpfs    tmpfs    /run      2>/dev/null
mount -t tmpfs    tmpfs    /tmp      2>/dev/null
hostname {hostname} 2>/dev/null
echo "{hostname}" > /etc/hostname

# ---- Watchdog firmware MediaTek (emulasi) ----
# Meniru /proc/net/monitor: daemon menendang (feed) tiap 5 detik. Bila
# feed berhenti -> reset paksa. State di /run/mtk/monitor:
#   1 = aktif (feed)   0 = mati (aman)   hang = berhenti feed -> reset
mkdir -p /run/mtk
echo 1 > /run/mtk/monitor

# ---- Jalankan "firmware watchdog" (daemon tendang) ----
/sbin/mtk-watchdogd >/dev/console 2>&1 &

echo
echo "================================================================"
echo "   UARTRecon - SIMULASI STB (ZTE B700V5S1)"
echo "================================================================"
echo "  SoC (emulasi) : ARM1176JZF-S  (ARMv6K) - sama dgn STB asli"
echo "  Board         : QEMU raspi1ap (BCM2835) @ RAM 512MB"
echo "  Kernel        : $(uname -r)"
echo "  Kernel lama   : 2.6.35 (MediaTek) -> DIGANTI"
echo "  Arsitektur    : $(uname -m)"
echo "  Hostname      : $(hostname)"
echo "----------------------------------------------------------------"
echo "  CPU info:"
grep -E "Processor|model name|Features|BogoMIPS|CPU part" /proc/cpuinfo | sed 's/^/    /'
echo "----------------------------------------------------------------"
echo "  Memori:"
head -3 /proc/meminfo | sed 's/^/    /'
echo "----------------------------------------------------------------"
echo "  Storage (NAND simulasi 256MB):"
ls -la /dev/mmcblk0 2>/dev/null | sed 's/^/    /'
echo "----------------------------------------------------------------"
echo "  Distribusi:"
cat /etc/alpine-release 2>/dev/null | sed 's/^/    Alpine /'
echo "----------------------------------------------------------------"
echo "  Watchdog (firmware MTK emulasi via /proc/net/monitor):"
echo -n "    kernel dev : "; cat /sys/class/watchdog/watchdog0/identity 2>/dev/null
echo -n "    timeout    : "; cat /sys/class/watchdog/watchdog0/timeout 2>/dev/null; echo " detik"
echo "    monitor    : /run/mtk/monitor = $(cat /run/mtk/monitor 2>/dev/null)"
echo "================================================================"
echo
echo "Login sebagai: root (tanpa password)"
echo "Watchdog    : 'wdt status|on|off|hang'"
echo "  wdt hang  -> berhenti feed -> mesin RESET paksa, seperti STB hang"
echo

# ---- Loop utama: respawn getty selamanya (PID 1 tetap hidup) ----
while true; do
    /sbin/getty -L ttyAMA0 115200 vt100 </dev/ttyAMA0 >/dev/ttyAMA0 2>&1
    echo
    echo "[init] getty keluar (logout/exit). Respawn..."
    sleep 1
done
"""

    # Daemon watchdog: pembungkus yang menulis ke /run/mtk/wdt.state.
    # Daemon "watchdog firmware MediaTek" (emulasi).
    #
    # Ini meniru watchdog chip pada STB asli: sebuah timer yang HARUS ditendang
    # berkala. Bila tidak ditendang (kontrol=hang atau daemon mati), chip
    # memutus & REBOOT PAKSA mesin. Di emulasi QEMU kita pakai `reboot -f`
    # (reset seketika, setara reset hardware) sebagai aksi watchdog.
    #
    # Meniru /proc/net/monitor:
    #   1    -> aktif, tendang tiap 5 detik
    #   0    -> mati (aman, tidak reset)
    #   hang -> berhenti menendang -> reboot paksa
    watchdogd = """#!/bin/sh
# mtk-watchdogd - watchdog firmware MediaTek (emulasi software).
# Menendang tiap 5 detik; bila berhenti -> reboot paksa (reset hardware).
MON=/run/mtk/monitor
TIMEOUT=15
echo "[watchdog] daemon firmware MTK jalan (monitor=$MON, timeout=${TIMEOUT}s)"
last=0
while true; do
    v=$(cat "$MON" 2>/dev/null)
    [ -z "$v" ] && v=1
    if [ "$v" = "0" ]; then
        # Watchdog dimatikan (aman). Tidak menendang, tidak reset.
        last=0
    elif [ "$v" = "hang" ]; then
        # Berhenti menendang -> setelah TIMEOUT detik, reset paksa.
        last=$((last+5))
        if [ "$last" -ge "$TIMEOUT" ]; then
            echo ""
            echo "================================================"
            echo " WATCHDOG TIMEOUT! Firmware watchdog reset."
            echo "================================================"
            sync
            reboot -f
            sleep 3600
        fi
    else
        # Aktif: tendang (reset penghitung).
        last=0
    fi
    sleep 5
done
"""

    # Helper kontrol watchdog untuk demo (meniru /proc/net/monitor MediaTek).
    wdt_helper = """#!/bin/sh
# wdt - kontrol watchdog firmware MTK (meniru /proc/net/monitor).
#
#   wdt on      -> watchdog AKTIF (ditendang, mesin aman)
#   wdt off     -> watchdog MATI  (aman, tidak akan reset)
#   wdt hang    -> BERHENTI tendang -> mesin RESET paksa (~15 detik)
#   wdt status  -> tampilkan status
MON=/run/mtk/monitor

case "$1" in
    on)
        echo 1 > "$MON"
        echo "watchdog ON  (ditendang, mesin aman)"
        ;;
    off)
        echo 0 > "$MON"
        echo "watchdog OFF (aman, tidak akan reset)"
        ;;
    hang)
        echo hang > "$MON"
        echo "WATCHDOG FEED STOPPED -> mesin RESET dalam ~15 detik"
        echo "(simulasi userspace hang / watchdog firmware berhenti menendang)"
        ;;
    status)
        echo "monitor    : $MON = $(cat "$MON" 2>/dev/null)"
        echo "kernel dev : $(cat /sys/class/watchdog/watchdog0/identity 2>/dev/null)"
        echo "timeout    : 15 detik (emulasi firmware MTK)"
        if pidof mtk-watchdogd >/dev/null 2>&1; then
            echo "daemon     : JALAN (pid $(pidof mtk-watchdogd))"
        else
            echo "daemon     : MATI"
        fi
        case "$(cat "$MON" 2>/dev/null)" in
            1)    echo "status     : AKTIF - watchdog menjaga mesin" ;;
            0)    echo "status     : OFF - watchdog tidak aktif (aman)" ;;
            hang) echo "status     : HANG - reset akan datang!" ;;
            *)    echo "status     : tidak diketahui" ;;
        esac
        ;;
    *)
        echo "usage: wdt on|off|hang|status"
        ;;
esac
"""

    motd = (
        "\r\n"
        "  UARTRecon STB simulation - kernel modern di CPU ARMv6\r\n"
        "  Login: root  (tanpa password)\r\n"
        "  Watchdog: 'wdt status|on|off|hang'\r\n"
        "\r\n"
    )
    extra = {
        "init": init_script.encode(),
        "sbin/mtk-watchdogd": watchdogd.encode(),
        "sbin/wdt": wdt_helper.encode(),
        "etc/hostname": (hostname + "\n").encode(),
        "etc/motd": motd.encode(),
    }

    # Patch /etc/shadow -> root tanpa password (agar bisa login).
    lines = []
    for line in shadow_data.decode(errors="replace").splitlines():
        parts = line.split(":")
        if parts and parts[0] == "root":
            parts[1] = ""
            line = ":".join(parts)
        lines.append(line)
    if lines:
        extra["etc/shadow"] = ("\n".join(lines) + "\n").encode()

    # Tambah getty console ke inittab (kalau ada).
    if inittab_data:
        tab = inittab_data.decode(errors="replace")
        if "ttyAMA0" not in tab:
            tab += "\nttyAMA0::respawn:/sbin/getty -L ttyAMA0 115200 vt100\n"
        extra["etc/inittab"] = tab.encode()

    # ---- Pass 2: tulis cpio ----
    exec_files = {"init", "sbin/mtk-watchdogd", "sbin/wdt"}
    buf = io.BytesIO()
    for name, m, data in entries:
        if name in extra:
            mode = 0o100755 if name in exec_files else 0o100644
            buf.write(cpio_entry(name, mode, next_ino, 1, 0, 0,
                                 int(m.mtime), extra[name]))
            next_ino += 1
            continue

        mode = m.mode & 0o7777
        if m.isdir():
            buf.write(cpio_entry(name, mode | 0o040000, next_ino, 2, m.uid,
                                 m.gid, int(m.mtime), b""))
            next_ino += 1
        elif m.issym():
            buf.write(cpio_entry(name, mode | 0o120000, next_ino, 1, m.uid,
                                 m.gid, int(m.mtime), m.linkname.encode()))
            next_ino += 1
        elif m.islnk():
            buf.write(cpio_entry(name, 0o100644, ino_of.get(name, 0),
                                 nlink_of.get(name, 1), m.uid, m.gid,
                                 int(m.mtime), b""))
        elif m.isreg():
            buf.write(cpio_entry(name, mode | 0o100000, ino_of.get(name, 0),
                                 nlink_of.get(name, 1), m.uid, m.gid,
                                 int(m.mtime), data or b""))
        elif m.ischr() or m.isblk():
            buf.write(cpio_entry(name, mode | (0o020000 if m.ischr() else 0o060000),
                                 next_ino, 1, m.uid, m.gid, int(m.mtime), b"",
                                 rdevmajor=m.devmajor, rdevminor=m.devminor))
            next_ino += 1
        elif m.isfifo():
            buf.write(cpio_entry(name, mode | 0o010000, next_ino, 1, m.uid,
                                 m.gid, int(m.mtime), b""))
            next_ino += 1

    existing = {n for n, _, _ in entries}
    for name, payload in extra.items():
        if name not in existing:
            mode = 0o100755 if name in exec_files else 0o100644
            buf.write(cpio_entry(name, mode, next_ino, 1, 0, 0, 0, payload))
            next_ino += 1

    buf.write(cpio_entry("TRAILER!!!", 0, 0, 1, 0, 0, 0, b""))

    raw = buf.getvalue()
    with open(dst, "wb") as f:
        f.write(gzip.compress(raw, 9))

    print(f"[+] initramfs: {dst}")
    print(f"    cpio (raw) : {len(raw):,} bytes")
    print(f"    gzip       : {os.path.getsize(dst):,} bytes")


if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(__doc__)
        sys.exit(1)
    build(sys.argv[1], sys.argv[2],
          sys.argv[3] if len(sys.argv) > 3 else "STB-B700V5")
