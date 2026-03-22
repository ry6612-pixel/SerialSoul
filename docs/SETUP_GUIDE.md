# 🌟 Novaclaw — 快速設定指南

> **5 分鐘完成設定，讓你的 $10 ESP32-S3 變成全功能 AI 助手**

---

## 你需要準備的東西

| 項目 | 取得方式 | 時間 |
|------|---------|------|
| 🤖 **Telegram Bot Token** | [@BotFather](https://t.me/BotFather) → `/newbot` | 1 分鐘 |
| 🧠 **Gemini API Key** | [Google AI Studio](https://aistudio.google.com/apikey)（免費） | 1 分鐘 |
| 📶 **WiFi 名稱 + 密碼** | 你的家用 WiFi | 0 分鐘 |
| 🔢 **Telegram Chat ID** | [@userinfobot](https://t.me/userinfobot) → Start | 1 分鐘 |

### 硬體

- **ESP32-S3-N16R8**（16MB Flash / 8MB PSRAM）— 約 NT$300
- USB-C 傳輸線

> 選配：OV3660 攝影鏡頭（~NT$100）、ST7789 LCD（~NT$60）、MAX98357 喇叭（~NT$30）、INMP441 麥克風（~NT$30）

---

## 方法一：預編譯燒錄（不需寫程式）

1. 從 [Releases](https://github.com/ry6612-pixel/Novaclaw/releases) 下載最新 `.bin`
2. 建立 `user_config.txt`：
   ```
   WIFI_SSID=你的WiFi名稱
   WIFI_PASS=你的WiFi密碼
   TG_TOKEN=你的Bot_Token
   GEMINI_KEY=你的Gemini_Key
   CHAT_ID=你的Chat_ID
   ```
3. 燒錄：
   ```powershell
   espflash flash -p COM3 novaclaw.bin
   ```
4. 打開 Telegram → 跟你的 Bot 說話，完成！🎉

---

## 方法二：從原始碼編譯

```powershell
# 1. 安裝 Rust + ESP32 工具鏈（首次）
rustup install nightly
cargo install espup espflash
espup install

# 2. 下載專案
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw

# 3. 設定你的憑證
cp user_config.example.txt user_config.txt
notepad user_config.txt   # 填入 3 組 key

# 4. 編譯 & 燒錄
.\build.ps1
# 或手動：
cargo build --release
espflash flash -p COM3 --partition-table partitions.csv target/xtensa-esp32s3-espidf/release/esp32
```

---

## 開始使用

燒錄完成後，ESP32 會自動連上 WiFi，在 Telegram 發送：

| 指令 | 說明 |
|------|------|
| `/help` | 查看所有指令（40+） |
| `/status` | 系統狀態（RAM、模型、技能數） |
| `/camera snap` | 拍照傳到 Telegram |
| `/camera vision` | 拍照 + AI 影像分析 |
| 任意文字 | AI 對話（自動偵測語言） |

### 自然語言編程

不用寫程式碼，直接打字告訴 Novaclaw 你要什麼：

```
「每隔 5 分鐘拍照，如果有人進房間就通知我」
「每天早上 8 點提醒我今天天氣」
「每 30 分鐘提醒我喝水」
```

Novaclaw 會自動解析你的意圖、建立排程、執行任務。

---

## 安全提醒

- ⚠️ **絕對不要把 `user_config.txt` 上傳到 GitHub**（已在 `.gitignore`）
- 所有憑證只存在 ESP32 的 NVS 加密分區
- 原始碼 100% 不含任何 API Key 或密碼
- 只回應你的 Chat ID，其他人無法控制你的 Bot

---

## 接線圖

詳細接線請見主 [README](../README.md#wiring-diagrams)。

## 常見問題

**Q: Gemini API 要付費嗎？**
A: Google 免費額度很充足，個人使用完全免費。Novaclaw 內建速率限制（5次/分鐘）防止超額。

**Q: 沒有攝影鏡頭可以用嗎？**
A: 可以！只需 ESP32-S3 就能用 AI 對話、排程、PC 控制等所有非視覺功能。

**Q: 可以不接電腦嗎？**
A: 燒錄後完全獨立運行，不需要電腦。24/7 WiFi 連線自動運作。
