# Novaclaw — Provision secrets into ESP32-S3 NVS via USB Serial
# Usage: .\provision.ps1 [-Port COM5] [-Config path\to\user_config.txt]
#
# This script sends WiFi / Telegram / Gemini credentials to the device
# over serial AFTER flashing.  Secrets are stored in NVS (on-chip flash)
# and are NEVER baked into the firmware binary.
#
# Flow:
#   1. .\build.ps1          (compile — zero secrets in .bin)
#   2. .\flash.ps1          (flash clean binary)
#   3. .\provision.ps1      (send secrets via USB serial → NVS)

param(
    [string]$Port = "COM5",
    [string]$Config = ""
)

$ErrorActionPreference = "Stop"

# ── Auto-install git hooks from .githooks/ (hash-verified) ──
$hooksSource = Join-Path $PSScriptRoot ".githooks"
$hooksDest   = Join-Path $PSScriptRoot ".git\hooks"
if (Test-Path $hooksSource) {
    foreach ($hookFile in Get-ChildItem $hooksSource -File) {
        $dest = Join-Path $hooksDest $hookFile.Name
        $srcHash = (Get-FileHash $hookFile.FullName -Algorithm SHA256).Hash
        $needCopy = (-not (Test-Path $dest)) -or ($srcHash -ne (Get-FileHash $dest -Algorithm SHA256).Hash)
        if ($needCopy) {
            Copy-Item $hookFile.FullName $dest -Force
            Write-Host "Hook installed: $($hookFile.Name) (hash mismatch — repaired)" -ForegroundColor Green
        }
    }
}

# ── Secret-file guard: abort if dangerous files exist in repo root ──
$dangerousFiles = @("user_config.txt", ".env")
foreach ($df in $dangerousFiles) {
    $dfPath = Join-Path $PSScriptRoot $df
    if (Test-Path $dfPath) {
        Write-Host "FATAL: '$df' found in repo root! Remove it immediately." -ForegroundColor Red
        Write-Host "Secrets must live OUTSIDE the repo: $HOME\.novaclaw\secrets\user_config.txt" -ForegroundColor Red
        exit 1
    }
}

# ── Resolve config file (repo-external ONLY — no repo-local fallback) ──
function Resolve-ConfigFile {
    $candidates = @()
    if ($Config) { $candidates += $Config }
    if ($env:NOVACLAW_CONFIG) { $candidates += $env:NOVACLAW_CONFIG }
    if ($env:NOVACLAW_CONFIG_DIR) {
        $candidates += (Join-Path $env:NOVACLAW_CONFIG_DIR "user_config.txt")
    }
    $candidates += (Join-Path $HOME ".novaclaw\secrets\user_config.txt")
    # NO repo-local fallback — secrets must be external

    foreach ($c in $candidates) {
        if ($c -and (Test-Path $c)) { return $c }
    }
    return $null
}

$configFile = Resolve-ConfigFile
if (-not $configFile) {
    Write-Host "ERROR: user_config.txt not found." -ForegroundColor Red
    Write-Host "Required path: $HOME\.novaclaw\secrets\user_config.txt" -ForegroundColor Red
    Write-Host "Run .\setup-secure-config.ps1 to create it."
    exit 1
}
Write-Host "Config: $configFile" -ForegroundColor Cyan

# ── Parse config ──
$vars = @{}
Get-Content $configFile -Encoding UTF8 | ForEach-Object {
    $line = $_.Trim()
    if ($line -and -not $line.StartsWith("#")) {
        if ($line -match '^\s*(\w+)\s*=\s*"(.*)"\s*$') {
            $vars[$Matches[1]] = $Matches[2]
        }
    }
}

# Validate required
foreach ($key in @("WIFI_SSID", "TG_TOKEN", "GEMINI_KEY", "CHAT_ID")) {
    if (-not $vars[$key] -or $vars[$key] -match '^\s*$') {
        Write-Host "ERROR: $key not set in config" -ForegroundColor Red
        exit 1
    }
}

# ── Build JSON payload ──
$payload = @{}
foreach ($kv in $vars.GetEnumerator()) {
    if ($kv.Value) { $payload[$kv.Key] = $kv.Value }
}
$json = ($payload | ConvertTo-Json -Compress)

# Mask secrets in display
$display = $json -replace '"TG_TOKEN":"[^"]*"', '"TG_TOKEN":"***"'
$display = $display -replace '"GEMINI_KEY":"[^"]*"', '"GEMINI_KEY":"***"'
$display = $display -replace '"WIFI_PASS\d*":"[^"]*"', '"WIFI_PASS*":"***"'
$display = $display -replace '"CHAT_ID":"[^"]*"', '"CHAT_ID":"***"'
Write-Host "Payload: $display" -ForegroundColor DarkGray

# ── Open serial port ──
Write-Host "Opening $Port (115200 baud)..." -ForegroundColor Cyan
try {
    $serial = New-Object System.IO.Ports.SerialPort $Port, 115200
    $serial.ReadTimeout = 3000
    $serial.WriteTimeout = 3000
    $serial.DtrEnable = $false
    $serial.RtsEnable = $false
    $serial.Open()
} catch {
    Write-Host "ERROR: Cannot open $Port — $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Make sure the device is connected and no other program holds the port."
    exit 1
}

try {
    # Wait for the device to signal provisioning readiness
    Write-Host "Waiting for device provisioning mode..." -ForegroundColor Yellow
    $deadline = (Get-Date).AddSeconds(30)
    $ready = $false

    while ((Get-Date) -lt $deadline) {
        try {
            $line = $serial.ReadLine()
            if ($line -match '"provision"\s*:\s*"ready"') {
                $ready = $true
                break
            }
            # Also show device log lines for debugging
            if ($line.Trim()) {
                Write-Host "  device> $($line.Trim())" -ForegroundColor DarkGray
            }
        } catch [System.TimeoutException] {
            # Normal — keep waiting
        }
    }

    if (-not $ready) {
        Write-Host ""
        Write-Host "Device did not enter provisioning mode within 30s." -ForegroundColor Yellow
        Write-Host "This is normal if the device already has config in NVS." -ForegroundColor Yellow
        Write-Host "To force re-provision, send /reset via Telegram first." -ForegroundColor Yellow
        Write-Host ""
        $answer = Read-Host "Send config anyway? (y/N)"
        if ($answer -ne "y") {
            Write-Host "Aborted." -ForegroundColor Yellow
            return
        }
    }

    # Send the JSON config
    Write-Host "Sending config..." -ForegroundColor Cyan
    $serial.WriteLine($json)
    Start-Sleep -Milliseconds 500

    # Read response
    $timeout = (Get-Date).AddSeconds(5)
    while ((Get-Date) -lt $timeout) {
        try {
            $resp = $serial.ReadLine()
            if ($resp -match '"provision"\s*:\s*"ok"') {
                Write-Host ""
                Write-Host "Provisioning complete! Device will connect to WiFi now." -ForegroundColor Green
                Write-Host "You can close this window."
                return
            } elseif ($resp -match '"provision"\s*:\s*"incomplete"') {
                Write-Host "WARNING: $resp" -ForegroundColor Yellow
            } elseif ($resp -match '"provision"\s*:\s*"error"') {
                Write-Host "ERROR: $resp" -ForegroundColor Red
            } else {
                Write-Host "  device> $($resp.Trim())" -ForegroundColor DarkGray
            }
        } catch [System.TimeoutException] { }
    }

    Write-Host "No confirmation received — check device serial output." -ForegroundColor Yellow
} finally {
    if ($serial.IsOpen) { $serial.Close() }
}
