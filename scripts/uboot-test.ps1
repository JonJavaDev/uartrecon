# Test akses U-Boot: reboot STB sambil spam ENTER untuk menangkap prompt U-Boot.
# READ-ONLY terhadap flash — hanya reboot + kirim Enter.
param(
    [string]$PortName = "COM3",
    [int]$Baud = 115200,
    [int]$SpamSeconds = 25
)

$port = New-Object System.IO.Ports.SerialPort $PortName, $Baud, None, 8, One
$port.ReadTimeout = 200
$port.WriteTimeout = 1000
$log = New-Object System.Text.StringBuilder

try {
    $port.Open()
    Start-Sleep -Milliseconds 300
    $port.DiscardInBuffer()

    # 1. Konfirmasi koneksi ke shell
    $port.Write("`r`n")
    Start-Sleep -Milliseconds 800
    $n = $port.BytesToRead
    if ($n -gt 0) {
        $buf = New-Object byte[] $n
        $port.Read($buf, 0, $n) | Out-Null
        [void]$log.Append([System.Text.Encoding]::ASCII.GetString($buf))
    }

    # 2. Trigger reboot
    Write-Output "[*] Mengirim 'reboot'..."
    $port.Write("reboot`r`n")

    # 3. Spam ENTER untuk menangkap U-Boot
    Write-Output "[*] Spam ENTER selama $SpamSeconds detik untuk menangkap U-Boot..."
    $deadline = (Get-Date).AddSeconds($SpamSeconds)
    $spamCount = 0
    while ((Get-Date) -lt $deadline) {
        $port.Write("`r`n")
        $spamCount++
        Start-Sleep -Milliseconds 80
        try {
            $n = $port.BytesToRead
            if ($n -gt 0) {
                $buf = New-Object byte[] $n
                $port.Read($buf, 0, $n) | Out-Null
                [void]$log.Append([System.Text.Encoding]::ASCII.GetString($buf))
            }
        } catch {}
    }

    Write-Output "[*] Total spam: $spamCount kali"
    Write-Output ""
    Write-Output "=== OUTPUT (bersih dari ANSI) ==="
    $out = $log.ToString() -replace "`e\[[0-9;]*m", ""
    Write-Output $out
} catch {
    Write-Output "ERROR: $_"
} finally {
    if ($port.IsOpen) { $port.Close() }
}
