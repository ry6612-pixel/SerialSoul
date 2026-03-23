# 🌟 Novaclaw — 完整設定教學

> **10～15 分鐘完成設定，讓你的 $10 ESP32-S3 變成全功能 AI 助手**
>
> 這份指南會一步步教你取得所有必要的 WiFi、Telegram Bot、Gemini API 設定，
> 然後把它們寫入裝置。**零程式基礎也能完成。**

---

## 你需要準備的東西

| 項目 | 取得方式 | 時間 |
|------|---------|------|
| 📶 **WiFi 名稱 + 密碼** | 你的家用 WiFi（2.4GHz） | 0 分鐘 |
| 🤖 **Telegram Bot Token** | [@BotFather](https://t.me/BotFather) → `/newbot` | 2 分鐘 |
| 🔢 **Telegram Chat ID** | [@userinfobot](https://t.me/userinfobot) → Start | 1 分鐘 |
| 🧠 **Gemini API Key** | [Google AI Studio](https://aistudio.google.com/apikey)（免費） | 2 分鐘 |

### 硬體

- **ESP32-S3-N16R8**（16MB Flash / 8MB PSRAM）— 約 NT$300
- USB-C 傳輸線

> 選配：OV3660 攝影鏡頭（~NT$100）、ST7789 LCD（~NT$60）、MAX98357 喇叭（~NT$30）、INMP441 麥克風（~NT$30）

---

## Step 1：取得 WiFi 資訊

你需要家裡 WiFi 的「名稱」和「密碼」。

| 欄位 | 你的值 |
|------|--------|
| **WIFI_SSID** | （WiFi 名稱，例如 `MyHome-WiFi`） |
| **WIFI_PASS** | （WiFi 密碼） |

> ⚠️ **重要**：ESP32 只支援 **2.4GHz** WiFi！
> 如果你的路由器有 2.4G 和 5G 兩個網路，請用 **不含 `-5G`** 的那個。

### 怎麼找到 WiFi 名稱？

- **Windows**：點右下角 WiFi 圖示 → 目前連接的就是
- **Mac**：點右上角 WiFi 圖示
- **手機**：設定 → WiFi → 目前連接的網路名稱
- **密碼**：看路由器背面的貼紙，或問設定路由器的人

---

## Step 2：建立 Telegram Bot（取得 TG_TOKEN）

Novaclaw 透過 Telegram Bot 跟你聊天。以下是建立步驟：

### 2a. 建立 Bot

1. 打開 **Telegram**（手機或電腦版都可以）
2. 搜尋 **`@BotFather`**
3. 點 **Start** 開始對話
4. 輸入 `/newbot`
5. 輸入你想要的 **Bot 顯示名稱**（中英文都行，例如：`我的AI助手`）
6. 輸入 **Bot 帳號**（必須是英文且以 `bot` 結尾，例如：`my_novaclaw_bot`）

BotFather 會回覆：
```
Done! Congratulations on your new bot.
...
Use this token to access the HTTP API:
1234567890:ABCDEFghijklMNOPQRstuv-wxyz123456
```

**複製那一長串 Token → 這就是你的 `TG_TOKEN`** ✅

### 2b. 取得你的 Chat ID

1. 在 Telegram 搜尋 **`@userinfobot`**
2. 點 **Start**
3. 它會回覆你的資訊：
```
Id: 1234567890
First: 你的名字
...
```

**那個數字 `1234567890` → 就是你的 `CHAT_ID`** ✅

> ⚠️ `CHAT_ID` 是你自己的用戶 ID，不是 Bot 的 ID。
> 這確保只有你能控制 Novaclaw，其他人發訊息會被忽略。

---

## Step 3：取得 Gemini API Key

Novaclaw 使用 Google Gemini AI 來理解你的訊息。

1. 打開瀏覽器，前往：**https://aistudio.google.com/apikey**
2. 用你的 **Google 帳號**登入
3. 點 **「Create API Key」**（建立 API 金鑰）
4. 第一次使用會自動建立 Google Cloud 專案 → 直接點確認
5. 複製產生的 API Key：
```
AIza____YOUR_KEY_HERE____
```

**這就是你的 `GEMINI_KEY`** ✅

> 💡 **免費方案**：每分鐘 15 次請求，每天 1500 次 — 個人使用完全夠。
>
> ⚠️ API Key 是你的帳號憑證，**絕對不要公開分享**。

---

## Step 4：設定設定檔

把上面取得的 5 個值寫入一個設定檔。

### 方法 A：使用自動腳本（推薦）

```powershell
# 開啟 PowerShell，進入專案目錄
cd C:\zc

# 建立安全設定路徑
.\setup-secure-config.ps1

# 打開設定檔，填入你的值
notepad $HOME\.novaclaw\secrets\user_config.txt
```

在記事本裡面，把每個引號中的範例值換成你自己的：

```ini
WIFI_SSID     = "你的WiFi名稱"
WIFI_PASS     = "你的WiFi密碼"
TG_TOKEN      = "1234567890:ABCDEFghijklMNOPQRstuv-wxyz123456"
CHAT_ID       = "1234567890"
GEMINI_KEY    = "AIza____YOUR_KEY_HERE____"
```

存檔關閉記事本。

### 方法 B：手動建立

1. 建立資料夾：`C:\Users\你的帳號\.novaclaw\secrets\`
2. 在裡面新增 `user_config.txt`，內容如上

---

## Step 5：編譯、燒錄、寫入設定

### 5a. 編譯韌體

```powershell
cd C:\zc
.\build.ps1
```

看到 `BUILD OK` 就代表成功。

### 5b. 燒錄到裝置

1. 用 USB 接上 ESP32-S3
2. 確認 COM Port：
   - 開啟「裝置管理員」→ 展開「連接埠 (COM 與 LPT)」
   - 找到 `USB-SERIAL CH340 (COMx)` → 記住那個數字

```powershell
.\flash.ps1 -Port COM7    # COM7 換成你的
```

### 5c. 寫入設定到裝置

```powershell
.\provision.ps1 -Port COM7
```

看到 **`Provisioning complete!`** 就成功了！🎉

### 方法二進階：從原始碼編譯（開發者）

```powershell
# 1. 安裝 Rust + ESP32 工具鏈（首次）
rustup install nightly
cargo install espup espflash
espup install

# 2. 下載專案
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw

# 3. 設定你的憑證
.\setup-secure-config.ps1
notepad $HOME\.novaclaw\secrets\user_config.txt

# 4. 編譯 → 燒錄 → 寫入設定
.\build.ps1
.\flash.ps1 -Port COM7
.\provision.ps1 -Port COM7
```

---

## ✅ 開始使用

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

## 🔒 安全提醒

- ⚠️ **絕對不要把 `user_config.txt` 上傳到 GitHub**（已在 `.gitignore`）
- 韌體二進位檔 **不包含任何 API Key 或密碼** → 設定走 USB Serial 寫入晶片 NVS
- 只回應你的 Chat ID，**其他人無法控制**你的 Bot
- 設定檔只存在你電腦的 `%USERPROFILE%\.novaclaw\secrets\` 目錄

---

## 📊 所有設定欄位一覽

| 欄位 | 必填 | 說明 | 範例 |
|------|------|------|------|
| `WIFI_SSID` | ✅ | WiFi 名稱（2.4GHz） | `MyHome-WiFi` |
| `WIFI_PASS` | ✅ | WiFi 密碼 | `password123` |
| `WIFI_SSID2` | ❌ | 備用 WiFi 名稱（自動切換） | `Office-WiFi` |
| `WIFI_PASS2` | ❌ | 備用 WiFi 密碼 | |
| `TG_TOKEN` | ✅ | Telegram Bot Token | `123456:ABC-xyz` |
| `CHAT_ID` | ✅ | 你的 Telegram 用戶 ID | `987654321` |
| `GEMINI_KEY` | ✅ | Google Gemini API Key | `AIza____YOUR_KEY____` |

---

## 🔍 常見問題

### WiFi 一直連不上？
- 確認是 **2.4GHz** 網路，不是 5GHz
- 確認密碼正確，沒有多餘的空格
- 把裝置靠近路由器試試

### Telegram Bot 沒有回應？
- 確認 `TG_TOKEN` 完整複製（包含冒號 `:` 前後的部分）
- 確認 `CHAT_ID` 是**你的用戶 ID**（數字），不是 Bot 的
- 先在 Telegram 跟 Bot 按一次 Start（讓它認得你）

### Gemini 回覆錯誤？
- 確認 API Key 有效：到 https://aistudio.google.com/apikey 檢查
- 免費方案有速率限制（15次/分），等一分鐘再試

### Provision 時 COM Port 找不到？
1. 開啟「裝置管理員」
2. 展開「連接埠 (COM 與 LPT)」
3. 插拔 USB 線，看哪個 COM 出現/消失
4. 記住那個編號（例如 COM7）

### Provision 逾時沒反應？
- 按一下開發板的 **RST 按鈕**重啟裝置，再重新執行 `.\provision.ps1`
- 確認沒有其他程式佔用 COM Port（例如 Arduino IDE、PuTTY）

### 忘記設定值怎麼辦？
| 忘記什麼 | 怎麼找回 |
|---------|---------|
| WiFi 密碼 | 看路由器背面貼紙，或進入路由器管理頁 |
| TG_TOKEN | Telegram → @BotFather → `/myBots` → 選你的 Bot → API Token |
| CHAT_ID | Telegram → @userinfobot → Start → 看 Id 那行 |
| GEMINI_KEY | https://aistudio.google.com/apikey → 查看已建立的 Key |

---

## 接線圖

詳細接線請見主 [README](../README.md#wiring-diagrams)。

---

**Q: Gemini API 要付費嗎？**
A: Google 免費額度很充足，個人使用完全免費。Novaclaw 內建速率限制（5次/分鐘）防止超額。

**Q: 沒有攝影鏡頭可以用嗎？**
A: 可以！只需 ESP32-S3 就能用 AI 對話、排程、PC 控制等所有非視覺功能。

**Q: 可以不接電腦嗎？**
A: 燒錄後完全獨立運行，不需要電腦。24/7 WiFi 連線自動運作。
