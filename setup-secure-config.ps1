$ErrorActionPreference = "Stop"

$repoConfig = Join-Path $PSScriptRoot "user_config.txt"
$secureDir = Join-Path $HOME ".novaclaw\secrets"
$secureConfig = Join-Path $secureDir "user_config.txt"

New-Item -ItemType Directory -Path $secureDir -Force | Out-Null

if ((Test-Path $repoConfig) -and -not (Test-Path $secureConfig)) {
    Move-Item $repoConfig $secureConfig
    Write-Host "Moved repo-local config to $secureConfig" -ForegroundColor Green
} elseif (Test-Path $secureConfig) {
    Write-Host "Secure config already exists: $secureConfig" -ForegroundColor Cyan
} else {
    Copy-Item (Join-Path $PSScriptRoot "user_config.example.txt") $secureConfig
    Write-Host "Created template at $secureConfig" -ForegroundColor Green
    Write-Host "Edit it before building." -ForegroundColor Yellow
}

[Environment]::SetEnvironmentVariable("NOVACLAW_CONFIG", $secureConfig, "User")

Write-Host "NOVACLAW_CONFIG set for current Windows user." -ForegroundColor Green
Write-Host "Secret store: $secureConfig" -ForegroundColor Cyan
Write-Host "build.ps1 will now prefer this path automatically." -ForegroundColor Cyan