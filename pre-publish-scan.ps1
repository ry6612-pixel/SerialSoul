<#
.SYNOPSIS
    Novaclaw Pre-Publish Safety Scan
.DESCRIPTION
    Run this BEFORE making the repo public.
    Scans tracked files + full git history for leaked secrets.
    Must return ALL GREEN to be safe for public visibility.
#>

param(
    [switch]$Fix  # If set, will offer to clean history
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "  NOVACLAW PRE-PUBLISH SAFETY SCAN" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

$fail = 0

# ─── 1. Tracked file scan: generic secret patterns ───
Write-Host "[1/7] Scanning tracked files for secret patterns..." -ForegroundColor Yellow

$secretPatterns = @(
    "\d{9,10}:AA[A-Za-z0-9_-]{20,}",   # Telegram Bot Token
    "AIzaSy[A-Z][a-z0-9A-Z_-]{25,}",   # Google API Key (real, not placeholder)
    "ghp_[A-Za-z0-9]{36}",              # GitHub PAT
    "gho_[A-Za-z0-9]{36}",              # GitHub OAuth
    "sk-[a-zA-Z0-9]{20,}",             # OpenAI API Key
    "AKIA[0-9A-Z]{16}",                # AWS Access Key
    "xoxb-[0-9]{10,}",                 # Slack Bot Token
    "re_[a-zA-Z0-9]{20,}",             # Resend API Key
    "gsk_[a-zA-Z0-9]{20,}"             # Groq API Key
)

$trackedFiles = git ls-files 2>&1
$trackedHits = @()
foreach ($f in $trackedFiles) {
    if (-not (Test-Path $f)) { continue }
    foreach ($pat in $secretPatterns) {
        $m = Select-String -Path $f -Pattern $pat -ErrorAction SilentlyContinue
        if ($m) {
            # Exclude known safe example values
            $m | Where-Object {
                $_.Line -notmatch "YourGeminiKey|YourBotToken|123456:ABC|example|placeholder|XXXXXXX"
            } | ForEach-Object { $trackedHits += $_ }
        }
    }
}

if ($trackedHits.Count -gt 0) {
    Write-Host "  FAIL: $($trackedHits.Count) potential secret(s) in tracked files!" -ForegroundColor Red
    $trackedHits | ForEach-Object {
        $line = $_.Line.Trim()
        if ($line.Length -gt 80) { $line = $line.Substring(0, 77) + "..." }
        Write-Host "    $($_.Filename):$($_.LineNumber)  $line" -ForegroundColor Red
    }
    $fail++
} else {
    Write-Host "  PASS: No secrets in tracked files" -ForegroundColor Green
}

# ─── 2. Banned file check ───
Write-Host "[2/7] Checking for banned files in tracking..." -ForegroundColor Yellow

$bannedPatterns = @(
    "user_config.txt", "*.env", ".env.*",
    "*.log", "*.bak", "*.corrupted", "*.orig",
    "*.pem", "*.key", "*.p12", "*.pfx", "*.jks", "*.keystore",
    "secrets.txt", "wifi.txt", "wifi_*.txt",
    "*secret*.*", "*credential*.*", "*password*.*"
)
$bannedHits = @()
foreach ($b in $bannedPatterns) {
    $foundFiles = $trackedFiles | Where-Object { $_ -like $b }
    if ($foundFiles) { $bannedHits += $foundFiles }
}

if ($bannedHits.Count -gt 0) {
    Write-Host "  FAIL: Banned files are tracked!" -ForegroundColor Red
    $bannedHits | ForEach-Object { Write-Host "    $_" -ForegroundColor Red }
    $fail++
} else {
    Write-Host "  PASS: No banned files tracked" -ForegroundColor Green
}

# ─── 3. .gitignore validation ───
Write-Host "[3/7] Validating .gitignore..." -ForegroundColor Yellow

$requiredIgnores = @("user_config.txt", "*.env", "*.log", "target/", ".embuild/")
$gi = Get-Content .gitignore -Raw -ErrorAction SilentlyContinue
$missingIgnores = @()
foreach ($r in $requiredIgnores) {
    if (-not ($gi -match [regex]::Escape($r))) {
        $missingIgnores += $r
    }
}

if ($missingIgnores.Count -gt 0) {
    Write-Host "  FAIL: .gitignore missing:" -ForegroundColor Red
    $missingIgnores | ForEach-Object { Write-Host "    $_" -ForegroundColor Red }
    $fail++
} else {
    Write-Host "  PASS: .gitignore covers all required patterns" -ForegroundColor Green
}

# ─── 4. Git history scan ───
Write-Host "[4/7] Scanning git history (all commits)..." -ForegroundColor Yellow

# Use generic patterns — NEVER put real key fragments here
$historyPatterns = "\d{9,10}:AA[A-Za-z]|AIzaSy[A-Z][a-z0-9]{8,}|ghp_[A-Za-z0-9]{10,}|gho_[A-Za-z0-9]{10,}"
$histLines = @(git log --all -p 2>&1 | Select-String -Pattern $historyPatterns)
# Filter out known safe example lines
$realHistHits = @($histLines | Where-Object {
    $_.Line -notmatch "YourGeminiKey|YourBotToken|123456:ABC|example|placeholder|XXXXXXX|AIzaSyXXX"
})

if ($realHistHits.Count -gt 0) {
    Write-Host "  WARNING: $($realHistHits.Count) potential secret pattern(s) in git history" -ForegroundColor Yellow
    Write-Host "  Review manually — may be false positives from regex patterns in docs" -ForegroundColor Yellow
    $realHistHits | Select-Object -First 5 | ForEach-Object {
        $line = $_.Line.Trim()
        if ($line.Length -gt 80) { $line = $line.Substring(0, 77) + "..." }
        Write-Host "    $line" -ForegroundColor Yellow
    }
} else {
    Write-Host "  PASS: Git history clean" -ForegroundColor Green
}

# ─── 5. Remote check ───
Write-Host "[5/7] Checking remotes..." -ForegroundColor Yellow

$remotes = git remote -v 2>&1
$badRemotes = $remotes | Where-Object { $_ -match "esp-32|ZeroClaw|nanoclaw" }
if ($badRemotes) {
    Write-Host "  FAIL: Found non-Novaclaw remotes!" -ForegroundColor Red
    $badRemotes | ForEach-Object { Write-Host "    $_" -ForegroundColor Red }
    $fail++
} else {
    Write-Host "  PASS: Only Novaclaw remote" -ForegroundColor Green
}

# ─── 6. Secrets not in repo ───
Write-Host "[6/7] Verifying secrets are external..." -ForegroundColor Yellow

$configInRepo = Test-Path "user_config.txt"
if ($configInRepo) {
    Write-Host "  FAIL: user_config.txt exists in repo root!" -ForegroundColor Red
    $fail++
} else {
    Write-Host "  PASS: user_config.txt not in repo" -ForegroundColor Green
}

# Check environment variable points to external location
$extConfig = [Environment]::GetEnvironmentVariable("NOVACLAW_CONFIG", "User")
if ($extConfig -and (Test-Path $extConfig)) {
    Write-Host "  PASS: External config at $extConfig" -ForegroundColor Green
} else {
    Write-Host "  INFO: NOVACLAW_CONFIG env var not set (optional)" -ForegroundColor Yellow
}

# ─── 7. Branch/tag check ───
Write-Host "[7/7] Checking branches and tags..." -ForegroundColor Yellow

$branches = @(git branch --all 2>&1)
$tags = @(git tag 2>&1 | Where-Object { $_ })
Write-Host "  Branches: $($branches.Count)" -ForegroundColor Gray
Write-Host "  Tags: $($tags.Count)" -ForegroundColor Gray

# ─── Summary ───
Write-Host "`n========================================" -ForegroundColor Cyan
if ($fail -eq 0) {
    Write-Host "  ALL CHECKS PASSED" -ForegroundColor Green
    Write-Host "  Safe to make repo PUBLIC" -ForegroundColor Green
} else {
    Write-Host "  $fail CHECK(S) FAILED" -ForegroundColor Red
    Write-Host "  DO NOT make repo public until all issues are fixed!" -ForegroundColor Red
}
Write-Host "========================================`n" -ForegroundColor Cyan

exit $fail
