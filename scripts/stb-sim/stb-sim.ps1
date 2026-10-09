#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Simulasi STB (ZTE B700V5S1) dengan KERNEL MODERN di QEMU.

.DESCRIPTION
    STB asli (ZTE B700V5S1, MediaTek MT8653) memakai CPU ARM1176JZF-S
    (ARMv6K) dan kernel Linux 2.6.35 (era ~2010) - sangat jadul.

    Script ini menyimulasikan board yang sama di QEMU, tetapi memakai
    kernel Linux MODERN (Alpine 6.x) untuk arsitektur armv6l. CPU yang
    diemulasi (QEMU `-cpu arm1176`, CPU part 0xb76) IDENTIK dengan STB.

    Alur:
      1. Unduh kernel + initramfs Alpine (armhf / ARMv6).
      2. Bangun ulang initramfs menjadi rootfs Alpine lengkap (login root).
      3. Buat disk NAND simulasi 256MB (seperti flash STB).
      4. Boot QEMU: CPU ARMv6, RAM 512MB, console serial.

.PARAMETER Action
    prepare  - hanya unduh & siapkan berkas (tanpa boot)
    run      - boot interaktif (serial stdio, Ctrl+A X untuk keluar)
    info     - tampilkan info environment & berkas

.PARAMETER Mem
    Ukuran RAM guest dalam MB (default 512, sama seperti STB asli).

.PARAMETER Image
    Path image kernel Alpine (default: vmlinuz-rpi armhf).

.EXAMPLE
    .\stb-sim.ps1 prepare
    .\stb-sim.ps1 run

.NOTES
    Butuh: QEMU (qemu-system-arm), Python 3, koneksi internet (sekali saja).
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet("prepare", "run", "info")]
    [string]$Action = "info",

    [int]$Mem = 512,

    [string]$Image = ""
)

$ErrorActionPreference = "Stop"

# ---- Lokasi & konstanta ----
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkDir = Join-Path $ScriptDir "work"
$AlpineBase = "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/armhf"
$Hostname = "STB-B700V5"

function Find-Tool([string]$Name, [string[]]$Candidates) {
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    foreach ($c in $Candidates) {
        if (Test-Path $c) { return $c }
    }
    return $null
}

$Qemu = Find-Tool "qemu-system-arm" @(
    "C:\msys64\ucrt64\bin\qemu-system-arm.exe",
    "C:\msys64\mingw64\bin\qemu-system-arm.exe",
    "C:\Program Files\qemu\qemu-system-arm.exe"
)
$QemuImg = Find-Tool "qemu-img" @(
    "C:\msys64\ucrt64\bin\qemu-img.exe",
    "C:\msys64\mingw64\bin\qemu-img.exe"
)
$Python = Find-Tool "python" @(
    "C:\Program Files\Blender Foundation\Blender 5.2\5.2\python\bin\python.exe",
    "C:\Python313\python.exe"
)
if (-not $Python) {
    $py = Get-ChildItem "$env:LOCALAPPDATA\Programs\Python" -Filter "python.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($py) { $Python = $py.FullName }
}
$Git = Find-Tool "git" @(
    "C:\Program Files\Git\cmd\git.exe",
    "C:\Program Files (x86)\Git\cmd\git.exe",
    "$env:LOCALAPPDATA\Programs\Git\cmd\git.exe",
    "C:\msys64\usr\bin\git.exe"
)

function Show-Info {
    Write-Host "=== STB Simulation (kernel modern) ===" -ForegroundColor Cyan
    Write-Host "  SoC (emulasi) : ARM1176JZF-S (ARMv6K) - sama dgn STB asli"
    Write-Host "  Kernel lama   : Linux 2.6.35 (MediaTek) -> DIGANTI"
    Write-Host "  Kernel baru   : Alpine Linux 6.x (armv6l)"
    Write-Host ""
    Write-Host "  Tooling:"
    Write-Host ("    QEMU   : " + ($(if ($Qemu) { $Qemu } else { "TIDAK ADA (install: pacman -S mingw-w64-ucrt-x86_64-qemu)" })))
    Write-Host ("    qemu-img: " + ($(if ($QemuImg) { $QemuImg } else { "TIDAK ADA" })))
    Write-Host ("    Python : " + ($(if ($Python) { $Python } else { "TIDAK ADA" })))
    Write-Host ""
    Write-Host "  Berkas kerja: $WorkDir"
    if (Test-Path $WorkDir) {
        Get-ChildItem $WorkDir -File -ErrorAction SilentlyContinue |
            Select-Object Name, @{n = "Size"; e = { "{0:N0} B" -f $_.Length } } |
            Format-Table -AutoSize | Out-String | Write-Host
    }
}

function Invoke-Download([string]$Url, [string]$OutFile) {
    if (Test-Path $OutFile) {
        Write-Host "  [skip] $(Split-Path $OutFile -Leaf) sudah ada" -ForegroundColor DarkGray
        return
    }
    Write-Host "  [down] $(Split-Path $OutFile -Leaf)..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing -TimeoutSec 300
    Write-Host "         OK ($("{0:N0}" -f (Get-Item $OutFile).Length) bytes)" -ForegroundColor Green
}

function Invoke-Prepare {
    if (-not $Qemu) { throw "QEMU tidak ditemukan. Install dulu (pacman -S mingw-w64-ucrt-x86_64-qemu)." }
    if (-not $Python) { throw "Python 3 tidak ditemukan (butuh untuk membangun initramfs)." }

    New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null

    Write-Host "`n[1/4] Mengunduh kernel & initramfs Alpine armhf (ARMv6)..." -ForegroundColor Cyan
    Invoke-Download "$AlpineBase/netboot/vmlinuz-rpi" (Join-Path $WorkDir "vmlinuz-rpi")
    Invoke-Download "$AlpineBase/netboot/initramfs-rpi" (Join-Path $WorkDir "initramfs-rpi")
    Invoke-Download "$AlpineBase/alpine-minirootfs-3.24.0-armhf.tar.gz" (Join-Path $WorkDir "alpine-armhf.tar.gz")

    Write-Host "`n[2/4] Mengunduh device tree (bcm2708-rpi-b-plus)..." -ForegroundColor Cyan
    $dtb = Join-Path $WorkDir "bcm2708-rpi-b-plus.dtb"
    if (-not (Test-Path $dtb)) {
        # Coba unduh langsung dulu (lebih cepat), fallback ke git sparse clone.
        $dtbUrl = "https://github.com/raspberrypi/firmware/raw/master/boot/bcm2708-rpi-b-plus.dtb"
        $ok = $false
        try {
            Invoke-WebRequest -Uri $dtbUrl -OutFile $dtb -UseBasicParsing -TimeoutSec 120
            $ok = $true
            Write-Host "  [down] dtb OK ($((Get-Item $dtb).Length) bytes)" -ForegroundColor Green
        } catch {
            Write-Host "  [warn] unduh langsung gagal, coba git sparse clone..." -ForegroundColor Yellow
        }
        if (-not $ok) {
            if (-not $Git) { throw "Git tidak ditemukan untuk mengambil dtb. Install Git atau letakkan dtb manual di $dtb" }
            $fw = Join-Path $WorkDir "rpi-fw"
            if (-not (Test-Path $fw)) {
                & $Git clone --depth 1 --filter=blob:none --sparse https://github.com/raspberrypi/firmware.git $fw 2>&1 | Out-Null
                Push-Location $fw
                & $Git sparse-checkout set boot 2>&1 | Out-Null
                Pop-Location
            }
            Copy-Item (Join-Path $fw "boot\bcm2708-rpi-b-plus.dtb") $dtb -Force
            Write-Host "  [git] dtb OK ($((Get-Item $dtb).Length) bytes)" -ForegroundColor Green
        }
    } else {
        Write-Host "  [skip] dtb sudah ada" -ForegroundColor DarkGray
    }

    Write-Host "`n[3/4] Membangun initramfs Alpine lengkap (login root)..." -ForegroundColor Cyan
    $rootfs = Join-Path $WorkDir "stb-rootfs.cpio.gz"
    & $Python (Join-Path $ScriptDir "build_initramfs.py") (Join-Path $WorkDir "alpine-armhf.tar.gz") $rootfs $Hostname
    if ($LASTEXITCODE -ne 0) { throw "gagal membangun initramfs" }

    Write-Host "`n[4/4] Membuat disk NAND simulasi (256MB)..." -ForegroundColor Cyan
    $nand = Join-Path $WorkDir "nand256.img"
    if (-not (Test-Path $nand)) {
        & $QemuImg create -f raw $nand 256M | Out-Null
        Write-Host "  OK ($((Get-Item $nand).Length) bytes)" -ForegroundColor Green
    } else {
        Write-Host "  [skip] image sudah ada" -ForegroundColor DarkGray
    }

    Write-Host "`n[+] Siap. Jalankan: .\stb-sim.ps1 run`n" -ForegroundColor Green
}

function Invoke-Run {
    $kernel = if ($Image) { $Image } else { Join-Path $WorkDir "vmlinuz-rpi" }
    $rootfs = Join-Path $WorkDir "stb-rootfs.cpio.gz"
    $dtb = Join-Path $WorkDir "bcm2708-rpi-b-plus.dtb"
    $nand = Join-Path $WorkDir "nand256.img"

    foreach ($f in @($kernel, $rootfs, $dtb)) {
        if (-not (Test-Path $f)) {
            throw "Berkas '$f' belum ada. Jalankan dulu: .\stb-sim.ps1 prepare"
        }
    }

    $append = "console=ttyAMA0,115200 initcall_blacklist=bcm2835_power_driver_init loglevel=7"
    Write-Host "=== Boot STB simulation (kernel modern 6.x, CPU ARMv6) ===" -ForegroundColor Cyan
    Write-Host "  RAM=$Mem MB  (Ctrl+A lalu X untuk keluar dari QEMU)" -ForegroundColor DarkGray
    Write-Host "  Watchdog aktif: 'wdt status|on|off|hang'  (hang -> mesin reset)`n" -ForegroundColor DarkGray

    # CATATAN: TANPA -no-reboot, agar watchdog bisa benar-benar me-reset mesin
    # (reboot -f). Kalau pakai -no-reboot, QEMU akan keluar saat reset.
    $qargs = @(
        "-M", "raspi1ap",
        "-cpu", "arm1176",
        "-m", "$Mem",
        "-kernel", $kernel,
        "-initrd", $rootfs,
        "-dtb", $dtb,
        "-append", $append,
        "-display", "none",
        "-serial", "stdio"
    )
    if (Test-Path $nand) {
        $qargs += @("-drive", "file=$nand,if=sd,format=raw")
    }

    & $Qemu @qargs
}

switch ($Action) {
    "info" { Show-Info }
    "prepare" { Invoke-Prepare }
    "run" { Invoke-Run }
}
