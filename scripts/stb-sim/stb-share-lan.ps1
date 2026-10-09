#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Jadikan laptop sebagai router + NAT untuk STB (share internet / akses jaringan).

.DESCRIPTION
    Skenario: STB terhubung ke laptop via kabel LAN (Ethernet), laptop terhubung
    ke jaringan/Internet via WiFi. Script ini membuat:

      1. STB -> laptop (sudah otomatis begitu kabel & IP benar)
      2. STB -> jaringan WiFi (PC lain, gateway, Internet)  via ICS + NAT
      3. PC lain -> layanan STB (SSH/LuCI)                  via port forwarding

    Setelah ini:
      - STB bisa ping/akses PC di jaringan WiFi & Internet.
      - PC lain bisa SSH/LuCI ke STB lewat <WiFi-IP-laptop>:2222 / :8080.

.PARAMETER StbIP
    IP STB di sisi kabel LAN (default 192.168.137.2).

.PARAMETER LanAdapter
    Nama adapter LAN yang terhubung ke STB (default auto-deteksi 192.168.137.x).

.PARAMETER WifiAdapter
    Nama adapter yang punya Internet (default auto-deteksi default route).

.PARAMETER SshPort / HttpPort
    Port di laptop untuk forward ke SSH(22)/HTTP(80) STB.

.EXAMPLE
    .\stb-share-lan.ps1
    .\stb-share-lan.ps1 -StbIP 192.168.137.2 -SshPort 2222 -HttpPort 8080

.NOTES
    Butuh hak Administrator (UAC). Jalankan dari PowerShell as Admin.
#>
[CmdletBinding()]
param(
    [string]$StbIP = "192.168.137.2",
    [string]$LanAdapter = "",
    [string]$WifiAdapter = "",
    [int]$SshPort = 2222,
    [int]$HttpPort = 8080
)

$ErrorActionPreference = "Stop"

# ---- Cek admin ----
$isAdmin = (New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Host "Script ini butuh Administrator. Jalankan PowerShell sebagai Admin." -ForegroundColor Red
    exit 1
}

# ---- Auto-deteksi adapter ----
if (-not $LanAdapter) {
    $lan = Get-NetIPAddress -AddressFamily IPv4 |
        Where-Object { $_.IPAddress -like '192.168.137.*' } |
        Select-Object -First 1
    if (-not $lan) { throw "Adapter LAN (192.168.137.x) tidak ditemukan. Set -LanAdapter manual." }
    $LanAdapter = $lan.InterfaceAlias
}
if (-not $WifiAdapter) {
    $def = Get-NetRoute -AddressFamily IPv4 -DestinationPrefix '0.0.0.0/0' |
        Sort-Object RouteMetric | Select-Object -First 1
    if (-not $def) { throw "Tidak ada default route (Internet). Set -WifiAdapter manual." }
    $WifiAdapter = $def.InterfaceAlias
}

$lanIP = (Get-NetIPAddress -InterfaceAlias $LanAdapter -AddressFamily IPv4).IPAddress
$wifiIP = (Get-NetIPAddress -InterfaceAlias $WifiAdapter -AddressFamily IPv4).IPAddress

Write-Host "=== STB Share LAN ===" -ForegroundColor Cyan
Write-Host "  LAN adapter  : $LanAdapter ($lanIP)"
Write-Host "  WiFi adapter : $WifiAdapter ($wifiIP)"
Write-Host "  STB IP       : $StbIP"
Write-Host "  Forward      : $wifiIP`:$SshPort -> ${StbIP}:22 (SSH)"
Write-Host "                 $wifiIP`:$HttpPort -> ${StbIP}:80 (LuCI)"
Write-Host ""

# ---- 1. IP forwarding ----
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters" `
    -Name IPEnableRouter -Value 1 -Type DWord -Force
Write-Host "[1] IPEnableRouter=1" -ForegroundColor Green

# ---- 2. ICS: WiFi (public) -> LAN (private) ----
$hs = New-Object -ComObject HNetCfg.HNetShare
$props = $hs.NetConnectionProps
$wifiConn = $null; $lanConn = $null
foreach ($c in $hs.EnumEveryConnection) {
    $n = $props.Invoke($c).Name
    if ($n -eq $WifiAdapter) { $wifiConn = $c }
    if ($n -eq $LanAdapter) { $lanConn = $c }
}
if (-not $wifiConn) { throw "Koneksi ICS untuk '$WifiAdapter' tidak ditemukan." }
if (-not $lanConn) { throw "Koneksi ICS untuk '$LanAdapter' tidak ditemukan." }

$wifiCfg = $hs.INetSharingConfigurationForINetConnection.Invoke($wifiConn)
$lanCfg = $hs.INetSharingConfigurationForINetConnection.Invoke($lanConn)
if ($wifiCfg.SharingEnabled) { $wifiCfg.DisableSharing() }
if ($lanCfg.SharingEnabled) { $lanCfg.DisableSharing() }
Start-Sleep -Seconds 2
$wifiCfg.EnableSharing(0)   # PUBLIC
$lanCfg.EnableSharing(1)    # PRIVATE
Write-Host "[2] ICS aktif ($WifiAdapter = public, $LanAdapter = private)" -ForegroundColor Green

# ---- 3. SharedAccess service ----
Set-Service -Name SharedAccess -StartupType Automatic -ErrorAction SilentlyContinue
Start-Service -Name SharedAccess -ErrorAction SilentlyContinue
Write-Host "[3] SharedAccess: $((Get-Service SharedAccess).Status)" -ForegroundColor Green

# ---- 4. Port forwarding (netsh portproxy) ----
$netsh = "$env:SystemRoot\System32\netsh.exe"
$forwards = @(
    @{ listen = $SshPort;  connect = 22; desc = "SSH" },
    @{ listen = $HttpPort; connect = 80; desc = "LuCI" }
)
foreach ($f in $forwards) {
    & $netsh interface portproxy delete v4tov4 listenaddress=$wifiIP listenport=$($f.listen) 2>&1 | Out-Null
    & $netsh interface portproxy add v4tov4 listenaddress=$wifiIP listenport=$($f.listen) connectaddress=$StbIP connectport=$($f.connect) 2>&1 | Out-Null

    $fwName = "UARTRecon-STB-$($f.listen)"
    Remove-NetFirewallRule -DisplayName $fwName -ErrorAction SilentlyContinue
    New-NetFirewallRule -DisplayName $fwName -Direction Inbound -Protocol TCP `
        -LocalPort $f.listen -Action Allow -Profile Any -ErrorAction SilentlyContinue | Out-Null
    Write-Host "[4] forward $($f.desc): ${wifiIP}:$($f.listen) -> ${StbIP}:$($f.connect)" -ForegroundColor Green
}

Write-Host ""
Write-Host "=== Selesai ===" -ForegroundColor Cyan
Write-Host "  STB -> jaringan/Internet : aktif (via $WifiAdapter)"
Write-Host "  PC lain -> SSH STB       : ssh -p $SshPort root@$wifiIP"
Write-Host "  PC lain -> LuCI STB      : http://${wifiIP}:$HttpPort/"
Write-Host ""
Write-Host "CATATAN: pastikan di STB sudah diset:" -ForegroundColor Yellow
Write-Host "  route add default gw $lanIP"
Write-Host "  echo 'nameserver $lanIP' > /etc/resolv.conf"
Write-Host "  (atau permanen: uci set network.lan.gateway=$lanIP; uci commit network)"
