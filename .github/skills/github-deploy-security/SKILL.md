---
name: github-deploy-security
description: "Pre-push security checklist for Novaclaw. Run BEFORE every git push. Scans for API keys, credentials in tracked files and git history, validates .gitignore, checks for risky patterns. Use when: pushing code, creating releases, auditing repo safety, onboarding new contributors."
argument-hint: "Run full pre-push audit or specify: 'scan keys', 'check history', 'validate gitignore'"
---

# Novaclaw — GitHub 部署安全檢查 SKILL

## ⚠️ 核心原則

**永遠不要把真實 API Key、Token、密碼推到 GitHub。**
**每次 push 前，必須執行以下完整檢查。**

---

## 1. 密鑰掃描（追蹤檔案）

掃描所有 git 追蹤檔案，搜尋真實密鑰模式：

```powershell
# 在 C:\zc 執行
# 替換下面的 pattern 為你的真實 key 片段（前 10 字元即可）
$patterns = @(
    "AIzaSy",           # Gemini API Key 前綴
    "ghp_",             # GitHub Personal Access Token
    "gho_",             # GitHub OAuth Token
    "sk-",              # OpenAI API Key
    "\d{9,10}:AA",      # Telegram Bot Token 格式
    "AKIA[0-9A-Z]",     # AWS Access Key
    "xoxb-",            # Slack Token
    "re_[a-zA-Z0-9]"    # Resend API Key
)

$found = 0
foreach ($p in $patterns) {
    $hits = git ls-files | ForEach-Object { Select-String -Path $_ -Pattern $p -ErrorAction SilentlyContinue }
    if ($hits) {
        Write-Host "⚠️  FOUND [$p]:" -ForegroundColor Red
        $hits | ForEach-Object { Write-Host "   $($_.Filename):$($_.LineNumber) $($_.Line.Trim())" }
        $found++
    }
}
if ($found -eq 0) { Write-Host "✅ 追蹤檔案：0 密鑰" -ForegroundColor Green }
```

### 允許的例外（不算洩漏）

- `user_config.example.txt` 中的空白佔位值（`""`, `"YourWiFiName"` 等）
- README 中的教學範例（如 `TG_TOKEN=123456:ABC-YourBotToken`）
- `AIzaSy...` 後面跟著 `...` 或 `YourGeminiKey` = 範例，不是真 key

### 判斷標準

| 模式 | 描述 | 安全？ |
|------|------|--------|
| `AIzaSy` + 28+ 個隨機字元 | 真實 Gemini key | ❌ 危險 |
| `AIzaSyYourGeminiKey` | 範例佔位值 | ✅ 安全 |
| `\d{9,10}:AA` + 隨機字元 | 真實 TG Bot Token | ❌ 危險 |
| `123456:ABC-YourBotToken` | 範例佔位值 | ✅ 安全 |

---

## 2. Git 歷史掃描

掃描 **所有 commit** 的 diff（不只是目前檔案）：

```powershell
# 掃描整段歷史（含已刪除的檔案）
$count = (git log --all -p | Select-String "你的真實key片段" | Measure-Object -Line).Lines
if ($count -gt 0) {
    Write-Host "❌ 歷史中有 $count 筆密鑰！" -ForegroundColor Red
    Write-Host "   需要：刪除含密鑰的分支/tag → git reflog expire → git gc --prune=now"
} else {
    Write-Host "✅ Git 歷史：乾淨" -ForegroundColor Green
}
```

### 如果歷史有洩漏

```powershell
# 1. 刪除所有指向舊歷史的 branch/tag
git branch -D <branch-name>
git tag -d <tag-name>

# 2. 清除 stash
git stash clear

# 3. 清除 reflog + garbage collect
git reflog expire --expire=now --all
git gc --prune=now --aggressive

# 4. 驗證
git log --all -p | Select-String "密鑰片段" | Measure-Object -Line
# 必須為 0
```

---

## 3. .gitignore 驗證

確認以下檔案/模式已被排除：

```powershell
$required = @(
    "user_config.txt",    # 所有真實 API key
    "*.env",              # 環境變數
    "*.log",              # 可能含 token 的 log
    "target/",            # 編譯產物
    ".embuild/",          # ESP-IDF 快取
    "ota_serve/",         # OTA 二進制
    "*.bak"               # 備份（可能含舊版密鑰）
)

$gitignore = Get-Content .gitignore -Raw
foreach ($r in $required) {
    if ($gitignore -match [regex]::Escape($r)) {
        Write-Host "✅ $r" -ForegroundColor Green
    } else {
        Write-Host "❌ $r 未在 .gitignore 中！" -ForegroundColor Red
    }
}
```

---

## 4. 危險檔案檢查

確認沒有不該追蹤的檔案被 git add：

```powershell
# 這些檔案不應出現在 git ls-files 中
$banned = @("user_config.txt", "*.env", "*.log", "ethan_driver.py", "*.bak", "*.corrupted")
$tracked = git ls-files
foreach ($b in $banned) {
    $matches = $tracked | Where-Object { $_ -like $b }
    if ($matches) {
        Write-Host "❌ 危險檔案被追蹤: $matches" -ForegroundColor Red
    }
}
```

---

## 5. 遠端 Repo 檢查

Push 前確認只推到正確的 remote：

```powershell
# 應該只有 Novaclaw
git remote -v
# 確認：
# origin  https://github.com/ry6612-pixel/Novaclaw.git

# 如果看到舊的 esp-32 或 ZeroClaw-EdgeFlow → 立刻 git remote remove！
```

---

## 6. Diff 最終確認

Push 前看一眼即將推送的內容：

```powershell
# 看有哪些檔案會被推送
git diff origin/main --stat

# 看完整 diff，確認沒有密鑰
git diff origin/main | Select-String "AIzaSy|ghp_|gho_|\d{10}:AA" | Measure-Object -Line
# 必須為 0
```

---

## 7. 完整一鍵檢查腳本

```powershell
# === Novaclaw Pre-Push Security Check ===
Write-Host "`n=== NOVACLAW PRE-PUSH SECURITY CHECK ===" -ForegroundColor Cyan

# 7.1 掃描追蹤檔案
# ⚠️ 使用你自己的 key 前綴替換 YOUR_KEY_PREFIX（取前 8 字元）
# 絕對不要把真實 key 片段寫進追蹤檔案！
$hits = git ls-files | ForEach-Object {
    Select-String -Path $_ -Pattern "AIzaSy[A-Z][a-z0-9]{15,}|ghp_[A-Za-z0-9]{36}|gho_[A-Za-z0-9]{36}|\d{9,10}:AA[A-Za-z0-9_-]{30,}" -ErrorAction SilentlyContinue
}
if ($hits) { Write-Host "❌ 追蹤檔案有密鑰！" -ForegroundColor Red; $hits } 
else { Write-Host "✅ 追蹤檔案乾淨" -ForegroundColor Green }

# 7.2 掃描歷史
# 用通用模式掃描，不要把真實 key 寫在腳本裡
$histCount = (git log --all -p | Select-String "AIzaSy[A-Z][a-z0-9]{8,}|\d{9,10}:AA[A-Za-z]|ghp_[A-Za-z0-9]{10,}" | Measure-Object -Line).Lines
if ($histCount -gt 0) { Write-Host "❌ 歷史有 $histCount 筆洩漏！" -ForegroundColor Red }
else { Write-Host "✅ Git 歷史乾淨" -ForegroundColor Green }

# 7.3 確認 remote
$remotes = git remote -v
if ($remotes -match "esp-32|ZeroClaw") { Write-Host "❌ 有舊的 remote！" -ForegroundColor Red }
else { Write-Host "✅ Remote 正確" -ForegroundColor Green }

# 7.4 確認 .gitignore
$gi = Get-Content .gitignore -Raw
if ($gi -match "user_config\.txt" -and $gi -match "\*\.env") { Write-Host "✅ .gitignore 正確" -ForegroundColor Green }
else { Write-Host "❌ .gitignore 缺少保護！" -ForegroundColor Red }

# 7.5 diff 檢查
$diffHits = (git diff origin/main | Select-String "AIzaSy[A-Z][a-z0-9]{8,}|\d{9,10}:AA[A-Za-z]|ghp_[A-Za-z0-9]{10,}" | Measure-Object -Line).Lines
if ($diffHits -gt 0) { Write-Host "❌ Diff 含 $diffHits 筆密鑰！" -ForegroundColor Red }
else { Write-Host "✅ Diff 乾淨" -ForegroundColor Green }

Write-Host "`n=== CHECK COMPLETE ===" -ForegroundColor Cyan
```

---

## 已知安全設計（正向）

| 保護 | 位置 | 說明 |
|------|------|------|
| chat_id 授權 | `src/main.rs L2481` | 只回應指定的 Telegram 用戶 |
| user_config.txt 被 .gitignore 排除 | `.gitignore L16` | 真實密鑰不會被追蹤 |
| build.ps1 從 config 讀取 | `build.ps1 L5` | 編譯時注入，不寫死在程式碼 |
| NVS 加密儲存 | `src/main.rs L1979` | 運行時密鑰存 ESP32 NVS |
| 環境變數注入 | `src/main.rs option_env!()` | 編譯時從 env var 帶入 |

---

## 已知風險（已接受）

| 風險 | 位置 | 決策 |
|------|------|------|
| AI 回覆 tag 自動執行到 PC | `main.rs L1284` | 核心功能，保留。README 已警告 |
| tg_token 經 USB 傳到 PC | `main.rs L2266` | 等 PC driver 重構時統一改 |
| OTA 無簽名驗證 | `main.rs L4576` | 個人使用可接受，量產版再加 |
| log 含完整 URL | `main.rs L5457` | serial log 不上雲，風險可控 |
| Cargo 未 pin rev | `Cargo.toml L13` | 等穩定版再 pin |
