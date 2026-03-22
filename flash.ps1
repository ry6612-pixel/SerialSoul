# ETHAN — Flash to ESP32-S3
# Usage: .\flash.ps1 [COM_PORT]
#   Default port: COM5
param([string]$Port = "COM5")

$bin = "target\xtensa-esp32s3-espidf\release\esp32"
if (-Not (Test-Path $bin)) {
    Write-Host "Binary not found. Run .\build.ps1 first." -ForegroundColor Red
    exit 1
}

Write-Host "Flashing to $Port ..." -ForegroundColor Cyan
espflash flash -p $Port --partition-table partitions.csv --after watchdog-reset $bin

if ($LASTEXITCODE -eq 0) {
    Write-Host "Flash complete! Device will restart." -ForegroundColor Green
} else {
    Write-Host "Flash failed (exit code $LASTEXITCODE)" -ForegroundColor Red
}
