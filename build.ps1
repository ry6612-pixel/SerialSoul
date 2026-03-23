$ErrorActionPreference = "Continue"
$log = "build_log.txt"
"$(Get-Date) === BUILD START ===" | Set-Content $log

# ─── Resolve config path (prefer repo-external secret store) ───
function Resolve-ConfigFile {
    $candidates = @()

    if ($env:NOVACLAW_CONFIG) {
        $candidates += $env:NOVACLAW_CONFIG
    }
    if ($env:NOVACLAW_CONFIG_DIR) {
        $candidates += (Join-Path $env:NOVACLAW_CONFIG_DIR "user_config.txt")
    }

    $candidates += (Join-Path $HOME ".novaclaw\secrets\user_config.txt")
    $candidates += (Join-Path $PSScriptRoot "user_config.txt")

    foreach ($candidate in $candidates) {
        if ($candidate -and (Test-Path $candidate)) {
            return $candidate
        }
    }

    return $null
}

$configFile = Resolve-ConfigFile
if (-Not $configFile) {
    Write-Host "ERROR: user_config.txt not found!" -ForegroundColor Red
    Write-Host "Recommended secure path: $HOME\.novaclaw\secrets\user_config.txt"
    Write-Host "You can also set NOVACLAW_CONFIG to a custom file path."
    Write-Host "Run .\setup-secure-config.ps1 to create the secure secret store."
    exit 1
}

Write-Host "Using config: $configFile" -ForegroundColor Cyan
if ($configFile -like "$PSScriptRoot*") {
    Write-Host "WARNING: Using repo-local config. Move secrets to $HOME\.novaclaw\secrets for safer separation." -ForegroundColor Yellow
}

$configVars = @{}
Get-Content $configFile -Encoding UTF8 | ForEach-Object {
    $line = $_.Trim()
    if ($line -and -not $line.StartsWith("#")) {
        if ($line -match '^\s*(\w+)\s*=\s*"(.*)"\s*$') {
            $configVars[$Matches[1]] = $Matches[2]
        }
    }
}

# Validate required fields
$required = @("WIFI_SSID", "TG_TOKEN", "GEMINI_KEY", "CHAT_ID")
foreach ($key in $required) {
    if (-not $configVars[$key] -or $configVars[$key] -match '^\s*$' -or $configVars[$key] -match '^(你的|123|AIzaSy\.\.\.)') {
        Write-Host "ERROR: Please set $key in user_config.txt" -ForegroundColor Red
        exit 1
    }
}

# Load ESP toolchain
. "$HOME\export-esp.ps1"
"$(Get-Date) ESP env loaded" | Add-Content $log

# Security: Do NOT export secrets as env vars — they must NOT be baked into
# the binary via option_env!().  Secrets are provisioned into NVS at first
# boot via provision.ps1 (USB Serial).
# Only non-secret build hints go into the environment.
$secretKeys = @("TG_TOKEN", "GEMINI_KEY", "CHAT_ID",
                "WIFI_SSID", "WIFI_PASS", "WIFI_SSID2", "WIFI_PASS2",
                "TTS_PROXY_URL", "TTS_PROXY_VOICE")
foreach ($kv in $configVars.GetEnumerator()) {
    if ($kv.Value -and $secretKeys -notcontains $kv.Key) {
        Set-Item "env:$($kv.Key)" $kv.Value
    }
}

Write-Host "Secrets are NOT baked into binary. Use provision.ps1 after flashing." -ForegroundColor Green

Set-Location $PSScriptRoot
"$(Get-Date) Building..." | Add-Content $log

# Build
$output = cargo build --release 2>&1 | Out-String
$exitCode = $LASTEXITCODE

if ($exitCode -eq 0 -or ($output -match "Finished")) {
    $bin = Get-Item "target\xtensa-esp32s3-espidf\release\esp32" -ErrorAction SilentlyContinue
    if ($bin) {
        "$(Get-Date) BUILD OK: $($bin.Name) $([math]::Round($bin.Length/1MB,2))MB" | Add-Content $log
    } else {
        "$(Get-Date) BUILD OK but binary not found" | Add-Content $log
    }
} else {
    "$(Get-Date) BUILD FAILED (exit $exitCode)" | Add-Content $log
    # Save last 30 lines of output
    $output -split "`n" | Select-Object -Last 30 | Add-Content $log
}

"$(Get-Date) === BUILD DONE ===" | Add-Content $log
