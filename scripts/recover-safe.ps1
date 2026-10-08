# Recovery: tunggu STB power-on, spam ENTER, lalu kirim 'safe'.
# Jalankan script ini SEBELUM mencolok power STB.
param(
    [string]$PortName = "COM3",
    [int]$Baud = 115200,
    [string]$UbootCmd = "safe",
    [int]$WaitSeconds = 30,
    [int]$CaptureSeconds = 25
)

$port = New-Object System.IO.Ports.SerialPort $PortName, $Baud, None, 8, One
$port.ReadTimeout = 200
$port.WriteTimeout = 1000
$log = New-Object System.Text.StringBuilder

try {
    $port.Open()
    $port.DiscardInBuffer()
    Write-Output "[*] Port terbuka. Menunggu STB power-on & spam ENTER selama $WaitSeconds detik..."
    Write-Output "[*] SILAKAN COLOK POWER STB SEKARANG."

    $deadline = (Get-Date).AddSeconds($WaitSeconds)
    $gotUboot = $false
    while ((Get-Date) -lt $deadline) {
        $port.Write("`r`n")
        Start-Sleep -Milliseconds 120
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
        Write-Output "[+] U-Boot tertangkap! Mengirim: $UbootCmd"
        Start-Sleep -Milliseconds 400
        $port.Write("$UbootCmd`r`n")
    } else {
        Write-Output "[!] U-Boot belum tertangkap. Mungkin STB belum power-on."
        Write-Output "    Coba jalankan ulang setelah colok power."
    }

    # Tangkap output
    $deadline = (Get-Date).AddSeconds($CaptureSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $n = $port.BytesToRead
            if ($n -gt 0) {
                $buf = New-Object byte[] $n
                $port.Read($buf, 0, $n) | Out-Null
                [void]$log.Append([System.Text.Encoding]::ASCII.GetString($buf))
            } else { Start-Sleep -Milliseconds 100 }
        } catch {}
    }

    $out = $log.ToString() -replace "`e\[[0-9;]*m", ""
    Write-Output ""
    Write-Output "=== OUTPUT (ringkas: cari 'safe'/'BusyBox'/prompt) ==="
    # Tampilkan hanya baris penting
    $lines = $out -split "`n"
    $show = $lines | Where-Object { $_ -match "STB-BOOT|safe|norm|BusyBox|Kernel panic|Starting_kernel|Uncompressing|rcS|login|#" }
    if ($show) { Write-Output ($show -join "`n") } else { Write-Output "(tidak ada baris kunci)" }
    Write-Output ""
    Write-Output "=== 40 baris terakhir ==="
    Write-Output (($lines | Select-Object -Last 40) -join "`n")
} catch {
    Write-Output "ERROR: $_"
} finally {
    if ($port.IsOpen) { $port.Close() }
}
