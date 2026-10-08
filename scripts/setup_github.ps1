# Script bantuan untuk menyiapkan UARTRecon agar siap di-upload ke GitHub (Windows).
#
# Penggunaan:
#   .\scripts\setup_github.ps1 -Username <github-username> [-Repo uartrecon]
#
# Script ini mengganti placeholder "USER" pada file dokumentasi/workflow
# dengan username GitHub Anda.

param(
    [Parameter(Mandatory = $true)]
    [string]$Username,
    [string]$Repo = "uartrecon"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)

Write-Host "[*] Mengganti placeholder USER -> $Username di $Root"

$Files = @(
    "README.md",
    "CHANGELOG.md",
    "Cargo.toml",
    ".github/ISSUE_TEMPLATE/config.yml"
)

foreach ($f in $Files) {
    $path = Join-Path $Root $f
    if (Test-Path $path) {
        $content = Get-Content -Raw -LiteralPath $path
        $content = $content -replace "github\.com/USER/", "github.com/$Username/"
        $content = $content -replace "USER/uartrecon", "$Username/$Repo"
        Set-Content -LiteralPath $path -Value $content -NoNewline
        Write-Host "    updated: $f"
    }
}

foreach ($crate in @("uartrecon-core", "uartrecon-cli", "uartrecon-tui", "uartrecon-gui")) {
    $path = Join-Path $Root "crates/$crate/Cargo.toml"
    if ((Test-Path $path) -and (Select-String -Path $path -Pattern "USER" -Quiet)) {
        $content = Get-Content -Raw -LiteralPath $path
        $content = $content -replace "USER", $Username
        Set-Content -LiteralPath $path -Value $content -NoNewline
        Write-Host "    updated: crates/$crate/Cargo.toml"
    }
}

Write-Host ""
Write-Host "[+] Selesai. Langkah berikutnya:"
Write-Host "    git init"
Write-Host "    git add -A"
Write-Host "    git commit -m 'feat: initial release v0.1.0'"
Write-Host "    git branch -M main"
Write-Host "    git remote add origin https://github.com/$Username/$Repo.git"
Write-Host "    git push -u origin main"
Write-Host ""
Write-Host "[i] Untuk membuat release: git tag v0.1.0; git push origin v0.1.0"
