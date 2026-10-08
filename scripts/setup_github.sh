#!/usr/bin/env bash
# Script bantuan untuk menyiapkan UARTRecon agar siap di-upload ke GitHub.
#
# Penggunaan:
#   bash scripts/setup_github.sh <github-username> [repo-name]
#
# Script ini mengganti placeholder "USER" pada file dokumentasi/workflow
# dengan username GitHub Anda, lalu (opsional) menginisialisasi git remote.
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Penggunaan: bash scripts/setup_github.sh <github-username> [repo-name]"
  echo "Contoh:     bash scripts/setup_github.sh johndoe uartrecon"
  exit 1
fi

USERNAME="$1"
REPO="${2:-uartrecon}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "[*] Mengganti placeholder USER -> $USERNAME di $ROOT"

# File yang mengandung placeholder USER (hanya URL github).
FILES=(
  "README.md"
  "CHANGELOG.md"
  "Cargo.toml"
  ".github/ISSUE_TEMPLATE/config.yml"
)

for f in "${FILES[@]}"; do
  path="$ROOT/$f"
  if [ -f "$path" ]; then
    # Ganti "github.com/USER/" dan "USER/uartrecon" saja.
    sed -i.bak \
      -e "s|github.com/USER/|github.com/${USERNAME}/|g" \
      -e "s|USER/uartrecon|${USERNAME}/${REPO}|g" \
      "$path"
    rm -f "$path.bak"
    echo "    updated: $f"
  fi
done

# Cargo.toml crate-level (repository/homepage/documentation).
for crate in uartrecon-core uartrecon-cli uartrecon-tui uartrecon-gui; do
  path="$ROOT/crates/$crate/Cargo.toml"
  if [ -f "$path" ] && grep -q "USER" "$path"; then
    sed -i.bak -e "s|USER|${USERNAME}|g" "$path"
    rm -f "$path.bak"
    echo "    updated: crates/$crate/Cargo.toml"
  fi
done

echo ""
echo "[+] Selesai. Langkah berikutnya:"
echo "    git init"
echo "    git add -A"
echo "    git commit -m 'feat: initial release v0.1.0'"
echo "    git branch -M main"
echo "    git remote add origin https://github.com/${USERNAME}/${REPO}.git"
echo "    git push -u origin main"
echo ""
echo "[i] Untuk membuat release: git tag v0.1.0 && git push origin v0.1.0"
