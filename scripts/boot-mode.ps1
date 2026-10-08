# Reboot STB, masuk U-Boot, jalankan 'norm' untuk boot ke slot norm.
param(
    [string]$PortName = "COM3",
    [int]$Baud = 115200,
    [string]$UbootCmd = "norm",
    [int]$SpamSeconds = 20,
    [int]$CaptureSeconds = 30
)

$port = New-Object System.IO.Ports.SerialPort $PortName, $Baud, None, 8, One
$port.ReadTimeout = 200
$port.WriteTimeout = 1000
$log = New-Object System.Text.StringBuilder

try {
    $port.Open()
    Start-Sleep -Milliseconds 300
    $port.DiscardInBuffer()

    # Konfirmasi koneksi
    $port.Write("`r`n")
    Start-Sleep -Milliseconds 500

    Write-Output "[*] Reboot STB..."
    $port.Write("reboot`r`n")

    # Spam Enter untuk menangkap U-Boot
    Write-Output "[*] Spam ENTER selama $SpamSeconds detik..."
    $deadline = (Get-Date).AddSeconds($SpamSeconds)
    $gotUboot = $false
    while ((Get-Date) -lt $deadline) {
        $port.Write("`r`n")
        Start-Sleep -Milliseconds 100
        try {
            $n = $port.BytesToRead
            if ($n -gt 0) {
                $buf = New-Object byte[] $n
                $port.Read($buf, 0, $n) | Out-Null
                $txt = [System.Text.Encoding]::ASCII.GetString($buf)
                [void]$log.Append($txt)
                if ($txt -match "STB-BOOT #") { $gotUboot = $true; break }
            }
        } catch {}
    }

    if ($gotUboot) {
        Write-Output "[+] U-Boot tertangkap! Mengirim perintah: $UbootCmd"
        Start-Sleep -Milliseconds 300
        $port.Write("$UbootCmd`r`n")
    } else {
        Write-Output "[!] U-Boot tidak tertangkap, STB mungkin boot normal"
    }

    # Tangkap output boot
    Write-Output "[*] Menangkap output selama $CaptureSeconds detik..."
    $deadline = (Get-Date).AddSeconds($CaptureSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $n = $port.BytesToRead
            if ($n -gt 0) {
                $buf = New-Object byte[] $n
                $port.Read($buf, 0, $n) | Out-Null
                [void]$log.Append([System.Text.Encoding]::ASCII.GetString($buf))
            } else {
                Start-Sleep -Milliseconds 100
            }
        } catch {}
    }

    $out = $log.ToString() -replace "`e\[[0-9;]*m", ""
    Write-Output ""
    Write-Output "=== OUTPUT ==="
    Write-Output $out
} catch {
    Write-Output "ERROR: $_"
} finally {
    if ($port.IsOpen) { $port.Close() }
}
