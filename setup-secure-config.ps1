$ErrorActionPreference = "Stop"

# ─── Auto-install git hooks from .githooks/ (hash-verified) ───
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

$repoConfig = Join-Path $PSScriptRoot "user_config.txt"
$secureDir = Join-Path $HOME ".novaclaw\secrets"
$secureConfig = Join-Path $secureDir "user_config.txt"

New-Item -ItemType Directory -Path $secureDir -Force | Out-Null

if ((Test-Path $repoConfig) -and -not (Test-Path $secureConfig)) {
    Move-Item $repoConfig $secureConfig
    Write-Host "Moved repo-local config to $secureConfig" -ForegroundColor Green
} elseif ((Test-Path $repoConfig) -and (Test-Path $secureConfig)) {
    Write-Host "WARNING: user_config.txt found in repo root AND in secure store!" -ForegroundColor Red
    Write-Host "The repo-local copy is DANGEROUS — removing it now." -ForegroundColor Red
    cmd /c del "$repoConfig"
    Write-Host "Deleted: $repoConfig" -ForegroundColor Yellow
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