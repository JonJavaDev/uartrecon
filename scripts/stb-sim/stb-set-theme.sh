#!/bin/sh
# stb-set-theme.sh - ganti tema LuCI STB (modern <-> klasik).
#
# Dijalankan DI DALAM STB (via SSH/UART). Butuh STB punya akses Internet
# (lihat docs/stb-lan-share.md untuk share Internet dari laptop).
#
# Penggunaan:
#   sh stb-set-theme.sh material   # tema modern (Material Design)
#   sh stb-set-theme.sh openwrt    # tema OpenWrt modern
#   sh stb-set-theme.sh bootstrap  # kembali ke tema lama
#   sh stb-set-theme.sh list       # lihat tema tersedia

set -e

THEME="${1:-list}"

show_list() {
    echo "=== Tema LuCI tersedia ==="
    opkg list | grep "^luci-theme-"
    echo ""
    echo "=== Tema aktif sekarang ==="
    echo "  $(uci get luci.main.mediaurlbase)"
}

set_theme() {
    name="$1"
    pkg="luci-theme-$name"

    # Cek ruang sebelum pasang.
    free_kb=$(df -k / | awk 'NR==2 {print $4}')
    echo "[*] Ruang bebas: ${free_kb} KB"
    if [ "$free_kb" -lt 200 ]; then
        echo "[!] Ruang < 200 KB. Batalkan (risiko kehabisan space)."
        exit 1
    fi

    # Pasang paket tema bila belum ada (kecuali bootstrap yang sudah ada).
    if ! opkg list-installed | grep -q "^$pkg "; then
        echo "[*] Memasang $pkg ..."
        opkg update
        opkg install "$pkg"
    else
        echo "[*] $pkg sudah terpasang."
    fi

    # Aktifkan.
    echo "[*] Mengaktifkan tema '$name' ..."
    uci set luci.main.mediaurlbase="/luci-static/$name"
    uci set luci.themes."$name"="/luci-static/$name" 2>/dev/null || true
    uci commit luci
    /etc/init.d/uhttpd restart
    sleep 2

    echo "[+] Selesai. Tema aktif: $(uci get luci.main.mediaurlbase)"
    echo "    Buka http://<IP-STB>/ lalu refresh (Ctrl+F5)."
}

case "$THEME" in
    list) show_list ;;
    bootstrap|material|openwrt|freifunk-generic) set_theme "$THEME" ;;
    *) echo "usage: sh $0 list|bootstrap|material|openwrt|freifunk-generic"; exit 1 ;;
esac
