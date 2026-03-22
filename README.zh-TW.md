<h1 align="center">🤖 Novaclaw — 你的 $10 AI 助手</h1>
<h3 align="center">跑在 ESP32-S3 上的獨立 AI 韌體 — 免電腦、免編成、只要 WiFi</h3>

<p align="center">
  <b>🌐 Language / 語言：</b>
  <a href="README.md">English</a> ·
  <a href="README.zh-TW.md"><b>繁體中文</b></a> ·
  <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <a href="https://github.com/ry6612-pixel/Novaclaw/stargazers"><img src="https://img.shields.io/github/stars/ry6612-pixel/Novaclaw?style=for-the-badge&color=gold" alt="Stars"></a>
  <a href="#快速開始"><img src="https://img.shields.io/badge/⚡_5分鐘上線-blue?style=for-the-badge" /></a>
  <a href="#hardware"><img src="https://img.shields.io/badge/💰_Hardware_~US$10-green?style=for-the-badge" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-orange?style=for-the-badge" /></a>
</p>

<p align="center">
  <b>一塊 $10 的板子，跑完整的 AI 助手。</b><br/>
  不需要伺服器、不需要電腦、不需要寫程式。插電、連 WiFi、開始對話。
</p>

---

## 💡 這是什麼？

**Novaclaw**（**E**dge **T**echnology **H**ardware **A**I **N**ode）是一套跑在 ESP32-S3 上的**獨立 AI 韌體**。不需要電腦、不需要雲端伺服器——只要一塊 NT$300 的開發板 + WiFi，就能擁有：

- 🤖 **Telegram AI 助手**（繁體中文對話、語音辨識、圖片分析）
- 📷 **邊緣視覺偵測**（OV3660 + Gemini Vision，本地拍照、雲端分析）
- 🔌 **低成本自動化控制**（GPIO 控制、排程系統、自然語言建立自動任務）
- 🖥️ **PC 遠端控制**（透過 USB Serial 操控電腦：截圖、開程式、執行命令）
- 🔄 **OTA 空中更新**（Telegram 傳 .bin 就能升級韌體）
- 💬 **自然語言編程** — 不用寫程式碼，直接說出你要的自動化：
  - `"每隔5分鐘拍照，如果有人進入就通知我"`
  - `"每天早上8點提醒我今天的天氣"`
  - `"幫我每小時截圖電腦畫面並分析有沒有異常"`

> **不管是低價自動化工廠、工地安全監控，還是智能居家單元，Novaclaw 讓你用幾百塊的硬體成本，實現過去需要十幾萬才能做到的 AI 進程管理。**

---

## ⭐ 核心亮點 — Why it deserves a Star

| | 特色 | 說明 |
|---|---|---|
| ⚡ | **免編成，即時導入** | 不需撰寫複雜邏輯。填入 WiFi + API Key → 燒入 → 5 分鐘內完成邊緣 AI 部署 |
| 👁️ | **AI 圖片讀寫** | 內建輕量級 AI 視覺能力：辨識物件、讀取文字/條碼、安全偵測，邊緣端拍照 + 雲端分析 |
| 🛠️ | **低配版 PLC 殺手** | 以 NT$300 取代傳統輕量級 PLC，排程控制、GPIO 開關、自動化進程管理 |
| 🧠 | **可擴充 AI 技能系統** | 內建 16+ AI 技能，支援自訂技能擴充與跨設備連動 |
| 🎙️ | **語音互動** | 支援語音辨識 + TTS 播報 + 喚醒詞偵測（ESP-SR WakeNet） |
| 📡 | **WiFi 自動恢復** | 主備雙網路自動切換，斷線自動重連，120 秒健康檢查 |
| 🔒 | **安全控制** | PC 安全模式（AI 指令預設鎖定）、OTA 僅限 HTTPS、串流 token 認證、聊天內容 log 遮蔽 |

---

## 🏗️ 應用場景

| 場景 | 怎麼用 | 實際應用 |
|------|--------|----------|
| 🏭 **低價自動化工廠** | 取代輕量 PLC，排程 + GPIO 控制產線進程 | 瑕疵品視覺篩選、設備啟停連動 |
| 🏗️ **工地安全** | 邊緣即時影像偵測，定時拍照分析 | 未戴安全帽警報、危險區域入侵偵測 |
| 🏢 **行政助手** | 自然語言建立任務，Telegram 直接操控 | 單據拍照識別、自動排程提醒 |
| 🏠 **智能居家** | 作為獨立的智能居家單元設備 | 門禁辨識、長者安全偵測、包裹監控 |
| 💻 **遠端 PC 控制** | USB 連電腦即可遠端操控桌面 | 遠端截圖、開程式、發郵件、執行 Python |

---

## 📸 實機展示 Demo

> 以下為 Novaclaw v5.1.0 跑在 ~NT$300 ESP32-S3-N16R8 開發板上的真實截圖。

### 系統狀態與指令選單

Bot 開機後自動報告完整系統狀態——韌體版本、AI 模型、記憶體、已連線的外接設備、PC Driver 連結狀態，以及 16 個已安裝的 AI 技能。發送 `/help` 即可查看完整命令手冊。

**一眼看完全部功能：** Gemini 3 Flash AI、相機視覺、語音輸入/輸出、排程提醒、TTS 快取、OTA 空中更新、USB 連接的 PC Driver——全部跑在一顆微控制器上。

### AI 驅動的相機視覺

說「拍照」，Novaclaw 立刻用 OV3660 拍照、上傳至 Gemini Vision 進行分析，並回覆詳細報告——辨識物件、偵測異常（如保溫棉破損、固定架鬆動），並給出可執行的維護建議。這一切只需要一顆 NT$100 的攝像頭模組。

### 遠端 PC 桌面控制

輸入 `/pc screenshot`，ESP32 透過 USB Serial 通知 PC Driver 擷取桌面截圖，再送回**同一個 Telegram 對話窗**。也支援 `/pc shell`、`/pc type`、`/pc click`、`/pc hotkey`，實現完整的遠端桌面自動化。

### 智能排程與記憶

- **自動任務** — 用自然語言建立重複性 AI 任務（例：「每分鐘分析攝像頭畫面並報告異常」）
- **提醒** — 每日三餐定時提醒，支援自訂訊息
- **記憶** — 透過 `/remember` 和 `/memories` 存取持久化鍵值記憶

### 16 個內建 AI 技能

程式碼生成與執行、圖片生成、Excel 讀寫、電子郵件、計算機、翻譯、喚醒詞偵測等——AI 自動判斷呼叫或透過 Telegram 斜線命令手動觸發。

---

<a id="hardware"></a>
## 📦 Hardware — 你需要什麼

### 核心板（必備）

| 元件 | 規格 | 參考價格 | 備註 |
|------|------|---------|------|
| **ESP32-S3-N16R8 開發板** | 16MB Flash / 8MB PSRAM | ~NT$250 | 果雲 (GuoYun) 款已驗證 |

> **最低配置：只要一塊 ESP32-S3 + USB 線，就能跑完整 Telegram AI 助手！**

### 選配模組（即插即用，接線後自動偵測）

| 模組 | 型號 | 價格 | 功能 | 沒有的話？ |
|------|------|------|------|-----------|
| 📷 攝影機 | OV3660 | ~NT$80 | `/camera snap`、`/camera vision`、AI 視覺 | 視覺功能停用，其餘正常 |
| 🖥️ LCD 螢幕 | ST7789 240×320 SPI | ~NT$60 | 顯示表情、狀態、IP | 無顯示，不影響功能 |
| 🔊 喇叭 | MAX98357 I2S 3W | ~NT$30 | TTS 語音播報、提示音 | 無聲音輸出 |
| 🎤 麥克風 | INMP441 I2S | ~NT$25 | 語音辨識、喚醒詞、PTT | 無語音輸入，文字正常 |

### 接線圖

<details>
<summary><b>📷 OV3660 攝影機接線（DVP 並列介面）</b></summary>

| OV3660 Pin | ESP32-S3 GPIO | 說明 |
|------------|---------------|------|
| PWDN | -1 (不接) | 省電控制 |
| RESET | -1 (不接) | 硬體重置 |
| XCLK | GPIO 15 | 時鐘輸出 |
| SDA | GPIO 4 | I2C 資料 |
| SCL | GPIO 5 | I2C 時鐘 |
| D0 | GPIO 11 | 資料位元 0 |
| D1 | GPIO 9 | 資料位元 1 |
| D2 | GPIO 8 | 資料位元 2 |
| D3 | GPIO 10 | 資料位元 3 |
| D4 | GPIO 12 | 資料位元 4 |
| D5 | GPIO 18 | 資料位元 5 |
| D6 | GPIO 17 | 資料位元 6 |
| D7 | GPIO 16 | 資料位元 7 |
| VSYNC | GPIO 6 | 垂直同步 |
| HREF | GPIO 7 | 水平參考 |
| PCLK | GPIO 13 | 像素時鐘 |

> 💡 **使用不同攝影機模組？** 透過 Telegram 發送 `/camera pins d0=11 d1=9 ...` 即可動態調整腳位，設定會存入 NVS 永久保存。

</details>

<details>
<summary><b>🖥️ ST7789 LCD 接線（SPI）</b></summary>

| ST7789 Pin | ESP32-S3 GPIO | 說明 |
|------------|---------------|------|
| RST | GPIO 21 | 重置 |
| DC | GPIO 47 | 資料/命令 |
| BL | GPIO 38 | 背光 |
| SCLK | GPIO 19 | SPI 時鐘 |
| SDA (MOSI) | GPIO 20 | SPI 資料 |
| CS | GPIO 45 | 晶片選擇 |

> 💡 **不同解析度或腳位？** 用 `/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45 w=240 h=320` 調整。

</details>

<details>
<summary><b>🔊 MAX98357 喇叭 + 🎤 INMP441 麥克風接線（I2S）</b></summary>

**喇叭 (MAX98357A 或相容 I2S DAC)：**

| MAX98357 Pin | ESP32-S3 GPIO | 說明 |
|--------------|---------------|------|
| BCLK | GPIO 40 | 位元時鐘 |
| LRC (WS) | GPIO 41 | 字組選擇 |
| DIN | GPIO 39 | 音訊資料輸入 |

**麥克風 (INMP441 或相容 I2S MEMS)：**

| INMP441 Pin | ESP32-S3 GPIO | 說明 |
|-------------|---------------|------|
| SCK | GPIO 2 | 位元時鐘 |
| WS | GPIO 1 | 字組選擇 |
| SD (DOUT) | GPIO 42 | 音訊資料輸出 |

> 💡 **不同腳位？** 用 `/audio pins bclk=40 ws=41 dout=39 mic_ws=1 mic_sck=2 mic_din=42 rate=24000` 調整。

</details>

### 🔄 替代硬體方案

<details>
<summary><b>我的板子腳位不一樣怎麼辦？</b></summary>

**所有硬體腳位都可以透過 Telegram 指令即時調整**，不需要改程式碼或重新編譯：

```
/camera pins d0=11 d1=9 xclk=15 ...
/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45
/audio pins bclk=40 ws=41 dout=39 mic_ws=1 mic_sck=2 mic_din=42
```

調整後的設定會永久存入 NVS，重啟後依然有效。

</details>

<details>
<summary><b>我用 OV2640 而不是 OV3660？</b></summary>

韌體的 camera driver 同時支援 OV2640 和 OV3660，會自動偵測。只要接線正確（DVP 介面腳位相同），直接使用即可。

</details>

<details>
<summary><b>我不想用 ST7789 LCD？</b></summary>

LCD 是選配的。不接 LCD，韌體會自動跳過顯示功能，不影響任何其他功能。如果你用其他 SPI LCD（如 ILI9341），只要支援 SPI 協定，可以嘗試相同的接線方式。

</details>

<details>
<summary><b>我用 ESP32-S3-CAM 一體板？</b></summary>

如果你用的是帶 OV3660/OV2640 的一體板（如果雲 ESP32-S3-CAM），攝影機腳位通常已經固定在 PCB 上。

1. 燒入韌體
2. 在 Telegram 發送 `/camera` 看預設腳位
3. 如果不匹配，用 `/camera pins ...` 調整為你板子的實際腳位

</details>

<details>
<summary><b>我沒有 PSRAM（N8R0 或 N4R0）？</b></summary>

**不建議**。攝影機和 TLS 連線需要大量記憶體。8MB PSRAM (N16R8) 是建議的最低配置。沒有 PSRAM 的話，攝影機功能會無法使用，且 HTTPS 連線可能不穩定。

</details>

---

<a id="快速開始"></a>
## 🎯 事前準備 — 三把鑰匙

開始之前，你需要取得以下三組免費的金鑰。**全部免費，大約 5 分鐘可搞定。**

---

### 🔑 金鑰 1：Telegram Bot Token + Chat ID

<details>
<summary><b>📱 點我展開 → 完整圖文教學</b></summary>

#### 步驟 A：建立你的 Bot

1. 打開 Telegram App（手機或電腦都行）
2. 搜尋 **`@BotFather`**（藍色勾勾認證帳號）
3. 點進去，按 **Start**
4. 發送 `/newbot`
5. BotFather 會問你 Bot 的**顯示名稱**（隨便取，例如 `我的AI助手`）
6. 再問你 Bot 的 **username**（必須以 `bot` 結尾，例如 `my_ai_helper_bot`）
7. 建立成功！BotFather 會回覆一段 Token，長這樣：

```
1234567890:ABCdefGHIjklMNOpqrSTUvwxYZ123456789
```

> ⚠️ **這串 Token 就是你的 `TG_TOKEN`，複製起來！不要給別人看！**

#### 步驟 B：取得你的 Chat ID

1. 在 Telegram 搜尋 **`@userinfobot`**
2. 按 **Start**
3. 它會立刻回覆你的資訊，裡面有一個數字 **Id**，例如：

```
Id: 123456789
```

> 這個數字就是你的 `CHAT_ID`。

#### 步驟 C：對你的 Bot 說 Hi

1. 回到 Telegram，搜尋你剛剛建立的 Bot（用 username 搜尋）
2. 點進去，按 **Start**
3. 隨便發一句話（例如 `hello`）
4. 這樣 Bot 才知道你是誰，之後才能發訊息給你

</details>

---

### 🔑 金鑰 2：Google Gemini API Key

<details>
<summary><b>🧠 點我展開 → 完整教學</b></summary>

1. 打開瀏覽器，前往 **[aistudio.google.com/apikey](https://aistudio.google.com/apikey)**
2. 用 Google 帳號登入
3. 點擊 **「Create API Key」**（建立 API 金鑰）
4. 選擇任一個 Google Cloud 專案（或讓它自動建立）
5. 複製產生的金鑰，長這樣：

```
AIzaSyXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

> ⚠️ **這就是你的 `GEMINI_KEY`。免費方案每分鐘 15 次請求，日常使用綽綽有餘。**

</details>

---

### 🔑 金鑰 3：你家的 WiFi

你需要知道：
- **WiFi 名稱（SSID）**：就是你手機連的那個 WiFi 名
- **WiFi 密碼**：你家 WiFi 的密碼

> 💡 支援 2.4GHz 和 5GHz 雙頻 WiFi。建議使用 2.4GHz（收訊範圍較廣）。

---

準備好三把鑰匙了嗎？接下來選擇你的安裝方式 ⬇️

---

## ⚡ 方法一：直接燒入 .bin（不需編譯，最簡單！）

> 適合：**只想用，不想搞開發環境的人。** 5 分鐘搞定。

### Step 1：下載燒入工具

選擇其中一種（推薦第一種）：

| 工具 | 平台 | 下載 | 難度 |
|------|------|------|------|
| **🌟 Flash Download Tool**（推薦） | Windows | [Espressif 官方下載](https://www.espressif.com/en/support/download/other-tools) | ⭐ 最簡單，有圖形介面 |
| **espflash**（命令行） | Win / Mac / Linux | `cargo install espflash` | ⭐⭐ 需安裝 Rust |
| **esptool.py** | Win / Mac / Linux | `pip install esptool` | ⭐⭐ 需安裝 Python |

### Step 2：下載韌體 .bin

到 [GitHub Releases](https://github.com/ry6612-pixel/Novaclaw/releases) 下載最新版的 `Novaclaw-vX.X.X.bin`。

### Step 3：連接板子

1. 用 **USB-C 傳輸線**（不是充電線！）插上 ESP32-S3
2. 電腦應該會自動安裝驅動
3. 打開 **裝置管理員** → 展開 **連接埠 (COM & LPT)** → 記下 COM 編號（例如 `COM3`）

> 如果看不到 COM Port：
> - 換一條 USB 線（很多線只能充電）
> - 嘗試安裝 [CH340 驅動](https://www.wch-ic.com/downloads/CH341SER_EXE.html) 或 [CP2102 驅動](https://www.silabs.com/developers/usb-to-uart-bridge-vcp-drivers)

### Step 4A：用 Flash Download Tool 燒入（圖形介面）

1. 解壓並開啟 Flash Download Tool
2. 選擇 **ESP32-S3**
3. 在第一行填入 .bin 檔案路徑，地址填 **`0x0`**
4. 選擇正確的 **COM Port**
5. Baud Rate 選 **460800**
6. 按 **START** 開始燒入
7. 等待進度條跑完，出現 `FINISH` 表示成功

### Step 4B：用 espflash 燒入（命令行）

```powershell
# Windows PowerShell
espflash flash -p COM3 zeroclaw-edgeflow-v5.0.1.bin

# 或用 esptool
esptool.py --chip esp32s3 -p COM3 -b 460800 write_flash 0x0 zeroclaw-edgeflow-v5.0.1.bin
```

### Step 5：首次設定

燒入完成後，板子會自動重啟。**第一次啟動時，WiFi 和 Token 都還沒設定**，你需要透過 Serial（序列埠）來設定：

1. 下載 [PuTTY](https://www.putty.org/)（免費 Serial 終端機）或使用任何 Serial 終端
2. 打開 PuTTY → Connection type 選 **Serial** → Serial line 填你的 **COM Port**（如 COM3）→ Speed 填 **115200** → 按 Open
3. 你會看到裝置的開機 log

> **或者：** 直接重新從源碼編譯（方法二），在 `user_config.txt` 裡預先填好所有設定，更乾淨。

### Step 6：透過 Telegram 設定（韌體已有預設值時）

如果韌體已經包含 WiFi / Token 設定（例如你用 `user_config.txt` 編譯的），板子會自動連上 WiFi 並開始運作。打開 Telegram 找你的 Bot，你會收到開機通知。

之後要更換 WiFi 只需：
```
/wifi set 新的WiFi名稱 新的密碼
```
板子會自動重啟並連上新的 WiFi。

---

## 🔧 方法二：從源碼編譯（完整控制）

> 適合：**想自訂功能、改引腳、調整參數的人。**

### Step 1：安裝開發環境（只要做一次）

```powershell
# 1. 安裝 Rust
winget install Rustlang.Rust.MSVC

# 2. 安裝 ESP32 工具鏈
cargo install espup espflash
espup install

# 3. 載入工具鏈環境（每次開新終端都要）
. "$HOME\export-esp.ps1"
```

> 💡 **Mac / Linux 使用者：** 把 `winget` 換成你系統的套件管理器，`export-esp.ps1` 換成 `. $HOME/export-esp.sh`。

### Step 2：下載專案

```powershell
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw
```

### Step 3：填寫設定檔

```powershell
# 建議做法：把 secrets 放到 repo 外
.\setup-secure-config.ps1
notepad $HOME/.novaclaw/secrets/user_config.txt

# 舊做法（相容保留，但較不安全）
Copy-Item user_config.example.txt user_config.txt
notepad user_config.txt
```

建議你編輯 `%USERPROFILE%\.novaclaw\secrets\user_config.txt`，填入你的三把鑰匙：

```ini
# ===== WiFi（必填）=====
WIFI_SSID     = "你家的WiFi名稱"
WIFI_PASS     = "你家的WiFi密碼"

# ===== 備用 WiFi（選填）=====
WIFI_SSID2    = ""
WIFI_PASS2    = ""

# ===== Telegram Bot（必填）=====
TG_TOKEN      = "從 @BotFather 拿到的 Token"
CHAT_ID       = "從 @userinfobot 拿到的數字"

# ===== Gemini API（必填）=====
GEMINI_KEY    = "從 aistudio.google.com 拿到的 Key"
```

> ⚠️ **每個值都要用雙引號包住！**

### Step 4：編譯 + 燒入

```powershell
.\build.ps1              # 編譯（第一次約 15-25 分鐘，之後約 1-3 分鐘）
.\flash.ps1              # 燒入（預設 COM5）
.\flash.ps1 -Port COM3   # 如果你的板子不是 COM5
```

`build.ps1` 會依序尋找：
1. `NOVACLAW_CONFIG`
2. `NOVACLAW_CONFIG_DIR\user_config.txt`
3. `%USERPROFILE%\.novaclaw\secrets\user_config.txt`
4. 專案內的 `user_config.txt`（舊相容模式）

### Step 5：驗證

板子重啟後，打開 Telegram：
1. 找到你的 Bot
2. 你應該會收到一條開機通知  
3. 發送 `/help` → 看到指令列表 = 成功！🎉

---

## 📖 完整指令手冊

> 所有指令都在 Telegram 中對你的 Bot 發送。

### 💬 AI 對話

直接傳文字就好，不需要斜線。Novaclaw 會用繁體中文回覆。

| 類型 | 範例 | 說明 |
|------|------|------|
| 文字對話 | `今天天氣如何？` | Gemini AI 回覆 |
| 傳送圖片 | 直接傳照片 | AI 自動分析圖片內容 |
| 傳送語音 | 直接傳語音訊息 | AI 自動語音轉文字再回覆 |
| 傳送 .bin 檔案 | 直接傳 .bin 檔 | 自動 OTA 韌體更新 |

---

### 📶 WiFi 管理

| 指令 | 功能 | 備註 |
|------|------|------|
| `/wifi` | 查看目前 WiFi 狀態 | 顯示主要+備用 |
| `/wifi set SSID 密碼` | 設定主要 WiFi | 設定後**自動重啟** |
| `/wifi set2 SSID 密碼` | 設定備用 WiFi | 主要斷線時自動切換 |
| `/wifi swap` | 主備互換 | 互換後**自動重啟** |
| `/wifi clear` | 清空全部 WiFi 設定 | 清空後**自動重啟** |

> 💡 **密碼分隔方式：** 空格 (`/wifi set MyWiFi 12345`) 或管道符 (`/wifi set MyWiFi|12345`)，WiFi 名稱有空格時請用管道符。

---

### 🧠 AI 模型切換

| 指令 | 功能 |
|------|------|
| `/model` | 查看目前使用的模型和硬體資訊 |
| `/model set flash` | 切換為 Gemini 3 Flash（快速） |
| `/model set pro` | 切換為 Gemini 3 Pro（高品質） |
| `/model set 3.1-pro` | 切換為 Gemini 3.1 Pro |
| `/model set 3.1-flash-lite` | 切換為 Gemini 3.1 Flash Lite（最省資源） |

---

### 📷 攝影機

| 指令 | 功能 |
|------|------|
| `/camera` | 查看攝影機狀態及腳位設定 |
| `/camera snap` | 拍照並傳回 Telegram |
| `/camera vision` | 拍照 + Gemini AI 自動分析 |
| `/camera vision 這是什麼？` | 拍照 + 用你的問題問 AI |
| `/camera selftest` | 攝影機自我測試（只拍不傳） |
| `/camera stream` | 啟動 MJPEG 影像串流（port 8080） |
| `/camera stream stop` | 停止影像串流 |
| `/camera pins d0=11 d1=9 ...` | 自訂攝影機腳位（存入 NVS） |
| `/camera default` | 恢復預設腳位 |

---

### 🖥️ LCD 螢幕

| 指令 | 功能 |
|------|------|
| `/lcd` | 查看 LCD 狀態及腳位設定 |
| `/lcd test` | 顯示測試圖案 |
| `/lcd draw 你好` | 在 LCD 上顯示自訂文字 |
| `/lcd pins rst=21 dc=47 ...` | 自訂 LCD 腳位（存入 NVS） |
| `/lcd default` | 恢復預設腳位 |

---

### 🔊 音訊系統

| 指令 | 功能 |
|------|------|
| `/audio` | 查看音訊狀態（喇叭/麥克風/喚醒詞） |
| `/audio tone` | 播放測試音效 |
| `/audio say 你好` | TTS 語音合成播放 |
| `/audio mic` | 麥克風快照（RMS / Peak） |
| `/audio transcribe` | 錄音 2.5 秒 → Gemini 語音轉文字 |
| `/audio level` | 偵測環境音量 |
| `/audio pins bclk=40 ws=41 ...` | 自訂音訊腳位（存入 NVS） |
| `/audio default` | 恢復預設腳位 |

**語音播報模式：**

| 指令 | 功能 |
|------|------|
| `/voice` | 查看目前語音模式 |
| `/voice off` | 關閉語音播報 |
| `/voice normal` | 完整語音播報 |
| `/voice brief` | 簡短語音播報 |

**喚醒詞（Wake Word）：**

| 指令 | 功能 |
|------|------|
| `/audio wake` | 查看喚醒詞狀態 |
| `/audio wake on` | 啟用喚醒詞監聽 |
| `/audio wake off` | 停用 |
| `/audio wake set ethan` | 設定喚醒詞 |
| `/audio wake now` | 立即測試（錄 2.5 秒） |

**TTS Proxy（外部語音合成）：**

| 指令 | 功能 |
|------|------|
| `/audio proxy` | 查看 TTS proxy 狀態 |
| `/audio proxy set <url>` | 設定 TTS proxy URL |
| `/audio proxy off` | 關閉 TTS proxy |
| `/audio proxy voice <name>` | 設定語音名稱 |

---

### 💾 記憶系統

| 指令 | 功能 |
|------|------|
| `/memories` 或 `/mem` | 列出所有記憶 |
| `/remember key value` | 儲存記憶（例：`/remember 咖啡 黑咖啡不加糖`） |
| `/forget key` | 刪除記憶 |

> AI 對話時也會自動從記憶中提取相關資訊。

---

### ⏰ 提醒排程

| 指令 | 功能 | 範例 |
|------|------|------|
| `/reminders` | 列出所有提醒 | |
| `/remind Nm 內容` | 每 N 分鐘提醒 | `/remind 30m 喝水` |
| `/remind Nh 內容` | 每 N 小時提醒 | `/remind 2h 站起來走走` |
| `/remind HH:MM 內容` | 每天定時提醒 | `/remind 08:00 早安` |
| `/remind del ID` | 刪除指定提醒 | `/remind del 3` |
| `/reminders clear` | 清空全部提醒 | |

---

### 🧩 技能系統

| 指令 | 功能 |
|------|------|
| `/skills` | 列出所有技能 |
| `/skill add 名稱\|描述\|觸發條件` | 新增自訂技能 |
| `/skill del 名稱` | 刪除技能 |

---

### 🤖 自動任務

| 指令 | 功能 |
|------|------|
| `/tasks` 或 `/autotask` | 列出背景任務 |
| `/tasks del ID` | 刪除指定任務 |
| `/tasks clear` | 清除全部 |
| `/stop` | **緊急停止**所有任務 |
| `/sleepwatch on` | 每 60 秒拍照偵測是否有人睡覺 |
| `/sleepwatch off` | 停止睡眠監視 |

---

### 💻 PC 遠端控制

> 需搭配 PC Driver（USB 串列連線），詳見 `/driver` 指令說明。

| 指令 | 功能 |
|------|------|
| `/pc shell <命令>` | 在 PC 執行命令 |
| `/pc screenshot` 或 `/ss` | PC 桌面截圖 |
| `/pc open <目標>` | 開啟程式或網址 |
| `/pc file_read <路徑>` | 讀取 PC 檔案 |
| `/pc file_list [路徑]` | 列出 PC 目錄 |
| `/pc file_write <路徑> <內容>` | 寫入 PC 檔案 |
| `/pc status` | PC 系統狀態 |
| `/pc click <x> <y>` | 滑鼠點擊 |
| `/pc type <文字>` | 鍵盤輸入 |
| `/pc hotkey ctrl+c` | 組合鍵 |
| `/pc excel_read <路徑>` | 讀取 Excel |
| `/pc clipboard` | PC 剪貼簿內容 |
| `/pc process` | 列出 PC 程序 |

---

### 📧 郵件

| 指令 | 功能 |
|------|------|
| `/email` | 查看郵件設定 |
| `/email config from\|to\|resend_api_key` | 設定寄件者 / 收件者 / Resend API Key |
| `/email to@example.com\|主旨\|內容` | 寄送郵件 |

---

### 🔄 韌體更新

| 方法 | 操作 |
|------|------|
| **Telegram OTA** | 直接傳 `.bin` 檔給 Bot，自動更新！ |
| **URL OTA** | `/ota https://example.com/firmware.bin` |
| **USB 重新燒入** | 用 `espflash` 或 Flash Download Tool |
| **PC Driver 更新** | `/update`（PC Driver 會自動編譯並燒入） |

---

### 🔧 系統管理

| 指令 | 功能 |
|------|------|
| `/start` 或 `/help` | 顯示指令列表 |
| `/status` | 系統狀態（版本、記憶體、WiFi、時間等） |
| `/diag` | 自我診斷報告 |
| `/tokens` | AI Token 使用統計 |
| `/driver` | PC Driver 連線說明 |
| `/briefing` | 每日簡報（天氣、新聞） |
| `/reset` | ⚠️ **恢復出廠設定**（清空所有 NVS 設定） |

---

## ❓ 常見問題 FAQ

<details>
<summary><b>Q：燒入後 Telegram Bot 沒反應？</b></summary>

按照順序檢查：

1. **WiFi 名稱/密碼對嗎？** → 確認大小寫、空格（用 PuTTY 看 Serial log 最快）
2. **TG_TOKEN 有填嗎？** → 確認 `user_config.txt` 裡有雙引號包住
3. **CHAT_ID 是數字嗎？** → 不要填 username，要填數字 ID
4. **你有對 Bot 說過 Hi 嗎？** → 第一次使用必須先在 Telegram 對 Bot 按 Start
5. **USB 線能傳資料嗎？** → 試另一條線，很多 USB 線只能充電

</details>

<details>
<summary><b>Q：如何更換 WiFi（不重新編譯）？</b></summary>

在 Telegram 對 Bot 發送：
```
/wifi set 新的WiFi名稱 新的WiFi密碼
```
板子會自動重啟並連上新的 WiFi。不需要重新編譯。

</details>

<details>
<summary><b>Q：忘記 WiFi 密碼，板子上不了線怎麼辦？</b></summary>

1. 修改 `user_config.txt` 的 WiFi 設定
2. 重新 `.\build.ps1` → `.\flash.ps1` 燒入

</details>

<details>
<summary><b>Q：燒入中斷（顯示失敗或某個百分比卡住）？</b></summary>

1. 按住板子上的 **BOOT** 鍵
2. 同時按一下 **RESET** 鍵
3. 放開 BOOT 鍵
4. 重新執行 `.\flash.ps1`（或 Flash Download Tool 按 START）

</details>

<details>
<summary><b>Q：build.ps1 報錯？</b></summary>

| 錯誤訊息 | 解法 |
|----------|------|
| `TG_TOKEN not set` | 檢查 `user_config.txt` 格式，每個值都要用 `"雙引號"` 包住 |
| `espup: command not found` | 重新安裝：`cargo install espup espflash` → `espup install` |
| 編譯超久 | 第一次正常，約 15-25 分鐘。之後增量編譯只需 1-3 分鐘 |
| `linker error` | 確認已執行 `. "$HOME\export-esp.ps1"` 載入工具鏈 |

</details>

<details>
<summary><b>Q：支援 Mac / Linux 嗎？</b></summary>

支援！只需把 PowerShell 命令替換為對應 shell 指令：

```bash
# Linux / Mac
source $HOME/export-esp.sh        # 載入 ESP 工具鏈
cargo build --release              # 編譯
espflash flash -p /dev/ttyUSB0 target/xtensa-esp32s3-espidf/release/esp32  # 燒入
```

</details>

<details>
<summary><b>Q：可以用手機燒入嗎？</b></summary>

不行。需要電腦（Windows / Mac / Linux）進行韌體燒入。燒入完成後就不需要電腦了，板子獨立運作。

</details>

---

## 🏛️ 架構

```
┌─────────────────────────────────────────────┐
│              Telegram Bot API               │
│         (commands, voice, photos)            │
└──────────────────┬──────────────────────────┘
                   │ HTTPS
┌──────────────────▼──────────────────────────┐
│            ESP32-S3 (Novaclaw Firmware)         │
│                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ Gemini   │ │ ESP-SR   │ │ Auto Tasks   │ │
│  │ AI Chat  │ │ WakeNet  │ │ Scheduler    │ │
│  └──────────┘ └──────────┘ └──────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ OV3660   │ │ ST7789   │ │ I2S Speaker  │ │
│  │ Camera   │ │ LCD Face │ │ TTS / Tone   │ │
│  └──────────┘ └──────────┘ └──────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ NVS      │ │ OTA      │ │ USB Serial   │ │
│  │ Memory   │ │ Update   │ │ PC Driver    │ │
│  └──────────┘ └──────────┘ └──────────────┘ │
└─────────────────────────────────────────────┘
         │ USB Serial (JSON Lines)
┌────────▼────────────────────────────────────┐
│  PC (Windows) — Shell / Screenshot / Email  │
└─────────────────────────────────────────────┘
```

---

## 🔑 安全

**已保護：**
- **零祕密在程式碼中** — 所有 API Key、密碼透過外部 secret 檔或 `user_config.txt` 注入，不會被 commit
- **repo 外 secret store** — `build.ps1` 預設優先讀 `%USERPROFILE%\.novaclaw\secrets\user_config.txt`
- **Chat ID 驗證** — 只回應授權的 Telegram 使用者
- **NVS 憑證儲存** — 敏感資料存於 ESP32 的 NVS 分區
- **PC 安全模式**（預設開啟）— AI 產生的危險 PC 指令（`shell`、`python`、`file_write`、`email`）預設封鎖，僅允許安全唯讀指令。切換：`/pc_unlock` / `/pc_lock`
- **OTA 僅限 HTTPS** — URL 韌體更新必須使用 HTTPS
- **MJPEG 串流 token** — 攝影串流需要隨機 session token 才能存取
- **聊天內容 log 遮蔽** — 用戶文字、語音轉錄、PTT 不會被記錄到 serial log
- **URL log 遮蔽** — HTTP 請求 log 中的 bot token 會被移除
- `user_config.txt` 已加入 `.gitignore`

**已知風險（請自行評估）：**
- 這是一個**高權限遠端控制系統** — PC 指令、OTA 更新、AI 自動化可以修改你的電腦
- 當 `/pc_unlock` 啟用時，AI 回覆可觸發 shell 命令、檔案寫入、程式執行
- 語音、照片、聊天歷史會送到 Google Gemini API 處理
- 自訂 TTS Proxy URL 可將文字送到第三方服務
- 透過 Telegram 檔案上傳的 OTA 不驗證韌體簽章
- MJPEG 攝影串流在區網使用 HTTP（非 HTTPS）

> ⚠️ **本專案設計為個人實驗用途。** 若你要連接真實 Telegram 帳號和 Windows 主機，請理解安全邊界。部署前請審閱原始碼。

---

## 🤝 Contributing

歡迎 PR！

1. Fork 本專案
2. 建立功能分支 (`git checkout -b feature/amazing`)
3. Commit (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing`)
5. 開 Pull Request

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) / [Apache-2.0](LICENSE-APACHE).

---

<p align="center">
  <b>⭐ 如果覺得有用，請給個 Star！</b><br/>
  <sub>Built with ❤️ by <a href="https://github.com/ry6612-pixel">ethan</a> — Powered by ESP32-S3 + Gemini</sub>
  <br/>
  <sub>Inspired by <a href="https://github.com/78/xiaozhi-esp32">xiaozhi-esp32</a> · Originally prototyped under the ZeroClaw name</sub>
</p>
