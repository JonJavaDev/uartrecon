#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Upload konten web statis ke STB (via SCP).

.DESCRIPTION
    Menyalin folder/file situs ke document root STB (/www) memakai pscp.
    STB sudah menjalankan uhttpd, jadi file langsung bisa diakses.

.PARAMETER Source
    Path folder/file lokal yang akan diupload (default: ./site).

.PARAMETER StbIP
    IP STB (default 192.168.137.2).

.PARAMETER RemotePath
    Folder tujuan di STB (default /www).

.PARAMETER Password
    Password SSH root STB (default UartRecon2026).

.EXAMPLE
    .\stb-upload.ps1
    .\stb-upload.ps1 -Source .\site -StbIP 192.168.137.2

.NOTES
    Butuh pscp.exe (dari PuTTY). STB harus bisa diakses via SSH.
#>
[CmdletBinding()]
param(
    [string]$Source = (Join-Path $PSScriptRoot "site"),
    [string]$StbIP = "192.168.137.2",
    [string]$RemotePath = "/www",
    [string]$Password = "UartRecon2026"
)

$ErrorActionPreference = "Stop"

# ---- Cari pscp ----
$pscp = @(
    "C:\Program Files\PuTTY\pscp.exe",
    "C:\Program Files (x86)\PuTTY\pscp.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $pscp) {
    $cmd = Get-Command pscp -ErrorAction SilentlyContinue
    if ($cmd) { $pscp = $cmd.Source }
}
if (-not $pscp) { throw "pscp.exe tidak ditemukan. Install PuTTY." }

if (-not (Test-Path $Source)) { throw "Sumber '$Source' tidak ada." }

# ---- Host key fingerprint STB (di-cache agar batch jalan) ----
$hostKey = "SHA256:gJ3MLICoJUPa9JZ9CxbE6wi5MPMukFuhfDT6zoiiRfg"

Write-Host "=== Upload ke STB ===" -ForegroundColor Cyan
Write-Host "  Sumber : $Source"
Write-Host "  Tujuan : root@${StbIP}:$RemotePath"
Write-Host ""

# ---- Upload ----
# -r untuk folder rekursif; pscp menangani file & folder.
& $pscp -scp -batch -r -hostkey $hostkey -pw $Password $Source "root@${StbIP}:${RemotePath}/"

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "[+] Upload selesai." -ForegroundColor Green
    Write-Host "    Buka http://${StbIP}/ (refresh: Ctrl+F5)" -ForegroundColor Cyan
} else {
    Write-Host "[!] Upload gagal (exit $LASTEXITCODE)." -ForegroundColor Red
}
