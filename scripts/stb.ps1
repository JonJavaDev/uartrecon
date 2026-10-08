# Helper untuk kirim command ke STB via UART (read-only first).
# Penggunaan:
#   .\stb.ps1 -Cmd "cat /proc/mtd"
#   .\stb.ps1 -Cmd "ls /dev" -Baud 115200
param(
    [Parameter(Mandatory = $true)]
    [string]$Cmd,
    [string]$PortName = "COM3",
    [int]$Baud = 115200,
    [int]$WaitMs = 1500
)

$port = New-Object System.IO.Ports.SerialPort $PortName, $Baud, None, 8, One
$port.ReadTimeout = 500
$port.WriteTimeout = 1000
try {
    $port.Open()
    Start-Sleep -Milliseconds 200
    $port.DiscardInBuffer()

    # Kirim command + Enter
    $port.Write("$Cmd`r`n")
    Start-Sleep -Milliseconds $WaitMs

    # Baca semua yang tersedia
    $sb = New-Object System.Text.StringBuilder
    $deadline = (Get-Date).AddMilliseconds($WaitMs)
    while ((Get-Date) -lt $deadline) {
        try {
            $n = $port.BytesToRead
            if ($n -gt 0) {
                $bytes = New-Object byte[] $n
                $port.Read($bytes, 0, $n) | Out-Null
                [void]$sb.Append([System.Text.Encoding]::ASCII.GetString($bytes))
            } else {
                Start-Sleep -Milliseconds 50
            }
        } catch { break }
    }

    $out = $sb.ToString()
    # Bersihkan ANSI escape + CR
    $out = $out -replace "`e\[[0-9;]*m", ""
    Write-Output $out
} catch {
    Write-Output "ERROR: $_"
} finally {
    if ($port.IsOpen) { $port.Close() }
}
