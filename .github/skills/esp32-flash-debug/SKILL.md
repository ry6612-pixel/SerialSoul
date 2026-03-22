---
name: esp32-flash-debug
description: "Debug ESP32-S3 Rust firmware build, flash, and encoding issues. Use when: build fails, flash fails, Unicode/FFFD corruption, Chinese text garbled, espflash errors, partition table problems, PSRAM/stack overflow, watchdog timeout, WiFi NVS issues, cargo build errors for xtensa target."
argument-hint: "Describe the build/flash/encoding error you're seeing"
---

# ESP32-S3 Rust 韌體燒入與編碼問題排查

## 適用場景

- `cargo build --release` 編譯失敗
- `espflash flash` 燒入失敗或裝置無回應
- 中文字元變成 `??` 或 `U+FFFD`（亂碼）
- 韌體開機後重啟（watchdog / stack overflow / panic）
- WiFi 連不上、NVS 設定遺失
- OTA 更新失敗

---

## 1. 環境建置

### 必備工具

```powershell
# 安裝 Rust ESP 工具鏈
cargo install espup
espup install          # 下載 xtensa-esp32s3 目標 + ESP-IDF

# 每次開 PowerShell 都要載入環境
. "$HOME\export-esp.ps1"

# 安裝燒入工具
cargo install espflash
```

### 工具鏈組態

[rust-toolchain.toml](./references/toolchain-example.md):
```toml
[toolchain]
channel = "esp"
components = ["rust-src"]
```

### 目標架構

- 晶片：ESP32-S3 N16R8（16MB Flash, 8MB PSRAM）
- Target triple：`xtensa-esp32s3-espidf`
- Binary 路徑：`target/xtensa-esp32s3-espidf/release/esp32`

---

## 2. 編譯問題排查

### 常見錯誤 → 解法

| 錯誤訊息 | 原因 | 解法 |
|-----------|------|------|
| `error: linker xtensa-esp32s3-elf-gcc not found` | 沒載入 ESP 環境 | `. "$HOME\export-esp.ps1"` |
| `region 'iram0_0_seg' overflowed` | Binary 太大 | 開啟 `opt-level = "s"` + `lto = true` + `strip = true` |
| `esp-idf-sys build failed` | ESP-IDF 版本衝突 | 刪除 `.embuild/` 重新建置 |
| `undefined reference to esp_camera_*` | camera component 未下載 | 刪除 `managed_components/` 並重建 |
| `undefined reference to esp_sr_*` | SR component 未下載 | 同上 |
| `error[E0308]: mismatched types` | esp-idf-svc API 變更 | 檢查 Cargo.toml 中的 git 依賴版本 |

### 完整重建（遇到詭異錯誤時）

```powershell
Remove-Item -Recurse -Force .embuild, target, managed_components -ErrorAction SilentlyContinue
cargo build --release 2>&1 | Tee-Object build_log.txt
```

> ⚠ 完整重建約需 15-30 分鐘（依 CPU），增量編譯通常 < 2 秒。

### Cargo.toml 關鍵設定

```toml
[profile.release]
opt-level = "s"     # 大小最佳化（ESP32 Flash 有限）
lto = true          # 連結時最佳化
codegen-units = 1   # 更好的最佳化
strip = true        # 移除 debug 符號
panic = "abort"     # 不用 unwind（節省空間）
```

---

## 3. 燒入問題排查

### 標準燒入指令

```powershell
espflash flash -p COM5 --partition-table partitions.csv --after watchdog-reset "target\xtensa-esp32s3-espidf\release\esp32"
```

### 常見燒入錯誤

| 狀況 | 原因 | 解法 |
|------|------|------|
| `Serial port not found` | COM Port 錯誤或被佔用 | 裝置管理員確認 Port；關閉其他 Serial Monitor |
| `Failed to connect` | 裝置未進入下載模式 | 按住 BOOT 鍵 → 按 RESET → 放開 BOOT |
| `Image too large` | Binary 超過分區大小 | 檢查 partitions.csv 的 ota_0 大小 |
| `Invalid partition table` | 分區表格式錯誤 | CSV 不能有多餘空格或 BOM |
| 燒入成功但不斷重啟 | Stack overflow / 初始化失敗 | 用 `espflash monitor -p COM5` 看 panic 訊息 |

### 分區表（16MB Flash）

```csv
# Name,   Type, SubType, Offset,   Size
nvs,      data, nvs,     0x9000,   0x6000,
otadata,  data, ota,     0xf000,   0x2000,
phy_init, data, phy,     0x11000,  0x1000,
ota_0,    app,  ota_0,   0x20000,  0x400000,   # 4MB — 主韌體
model,    data, spiffs,  0x420000, 0x200000,   # 2MB — SR 模型
ota_1,    app,  ota_1,   0x800000, 0x800000,   # 8MB — OTA 備份
```

### 觀察序列埠輸出

```powershell
espflash monitor -p COM5          # 即時 log
# 或
espflash flash -p COM5 --monitor --partition-table partitions.csv --after watchdog-reset $bin
```

---

## 4. Unicode / 中文編碼問題（FFFD 修復）

### 症狀

- Telegram 收到 `??`、`\u{FFFD}` 或亂碼
- `main.rs` 裡面出現 `U+FFFD`（REPLACEMENT CHARACTER, `\xEF\xBF\xBD`）
- 中文字被截斷或混合垃圾字元

### 根因

1. **編輯工具編碼錯誤**：用非 UTF-8 工具編輯 `.rs` 檔，造成部分字元被替換為 `U+FFFD`
2. **自動修復腳本破壞**：Python script 的 `open(..., errors='replace')` 會把無法解碼的 byte 替換為 `\uFFFD`
3. **Copy/Paste 編碼不一致**：從不同系統複製的中文可能包含不同 UTF-8 序列

### 偵測

```powershell
# 掃描 main.rs 中所有 FFFD
python -c "
import re
with open('src/main.rs', encoding='utf-8') as f:
    for i, line in enumerate(f, 1):
        if '\ufffd' in line:
            print(f'L{i}: {line.rstrip()}')" 
```

### 修復流程

1. **找到受影響的行**：用上面的掃描腳本
2. **從參考版本取得正確文字**：如果有舊版本的乾淨副本
3. **逐行修復**：根據上下文推斷正確的中文字

### Rust 中文字串最佳實踐

```rust
// ✅ 正確 — 直接使用 UTF-8 中文字面值
let msg = "已連接 WiFi ✅";

// ✅ 正確 — 用 \u{} 表示 emoji（避免編輯器問題）
let emoji = "\u{2705}";      // ✅
let headphone = "\u{1f3a7}"; // 🎧

// ❌ 危險 — 不要用 \x 手動編碼中文
let bad = "\xE5\xB7\xB2";   // 這是合法的但很脆弱

// ❌ 危險 — format!() 中的中文被截斷
// 確保 format string 和變數都是完整的 UTF-8
```

### 常見被汙染的字串模式

| 出現位置 | 通常內容 | 修復方式 |
|----------|----------|----------|
| WiFi log | `"嘗試連接"`, `"已連接"`, `"連線超時"` | 恢復正確中文 |
| Telegram 回覆 | `"✅"`, `"❌"`, `"🎧"` | 用 `\u{xxxx}` 或直接 UTF-8 |
| Gemini prompt | `"你是 ETHAN ── Gemini Vision..."` | 從參考版本複製 |
| 註解 | 各種中文說明 | 恢復或刪除 |

---

## 5. 執行時期問題

### Watchdog Timeout

```
sdkconfig.defaults:
CONFIG_ESP_TASK_WDT_TIMEOUT_S=12
CONFIG_ESP_INT_WDT_TIMEOUT_MS=1600
```

如果 TLS 握手或大型 HTTP 回應導致 watchdog reset：
- 增加 `CONFIG_ESP_TASK_WDT_TIMEOUT_S`（最大 60）
- 在長時間操作中加入 `FreeRtos::delay_ms(10)` 喂狗

### Stack Overflow

```
sdkconfig.defaults:
CONFIG_MAIN_TASK_STACK_SIZE=65536    # 64KB
CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT=8192
```

如果出現 `stack overflow` panic：增加對應 task 的 stack size。

### PSRAM 問題

```
sdkconfig.defaults:
CONFIG_SPIRAM=y
CONFIG_SPIRAM_MODE_OCT=y
CONFIG_SPIRAM_SPEED_80M=y
CONFIG_SPIRAM_USE_MALLOC=y
CONFIG_SPIRAM_MALLOC_ALWAYSINTERNAL=4096
```

- 4KB 以下的 alloc 用 internal SRAM（快）
- 4KB 以上自動用 PSRAM（8MB, 較慢但大）
- Camera buffer 和 audio buffer 通常在 PSRAM

### WiFi NVS 問題

WiFi 設定存在 NVS（Non-Volatile Storage）分區。如果 NVS 損壞：

```powershell
# 清除 NVS 分區
espflash erase-region 0x9000 0x6000 -p COM5
# 然後重新燒入
espflash flash -p COM5 --partition-table partitions.csv --after watchdog-reset $bin
```

---

## 6. 秘密管理（user_config 系統）

### 結構

- `user_config.example.txt` — 範本（提交到 Git）
- `user_config.txt` — 真正的秘密（.gitignore 排除）
- `build.ps1` — 讀取 config 設定環境變數

### 格式

```ini
# user_config.txt
WIFI_SSID     = "MyNetwork"
WIFI_PASS     = "MyPassword"
WIFI_SSID2    = "BackupNetwork"
WIFI_PASS2    = "BackupPass"
TG_TOKEN      = "123456:ABC..."
CHAT_ID       = "123456789"
GEMINI_KEY    = "AIza..."
```

### 安全檢查

推送前務必確認沒有秘密洩漏：

```powershell
# 掃描所有將被 git 追蹤的檔案
git ls-files | ForEach-Object {
    $content = Get-Content $_ -Raw -ErrorAction SilentlyContinue
    if ($content -match '(ghp_|AIzaSy|[0-9]{8,}:[A-Za-z0-9_-]{30,})') {
        Write-Host "WARNING: $_ may contain secrets!" -ForegroundColor Red
    }
}
```

---

## 7. 快速排查清單

```
□ 編譯失敗 → 有沒有載入 export-esp.ps1？
□ 編譯失敗 → .embuild/ 有沒有損壞？試刪除重建
□ 燒入失敗 → COM Port 對嗎？有其他程式佔用嗎？
□ 燒入失敗 → 試試按住 BOOT 鍵燒入
□ 開機重啟 → espflash monitor 看 panic log
□ 中文亂碼 → 用 FFFD 掃描腳本檢查 main.rs
□ WiFi 連不上 → NVS 可能損壞，試 erase-region
□ 秘密洩漏 → 推送前跑安全檢查腳本
```
