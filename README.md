<h1 align="center">🌟 Novaclaw — Your $10 AI Assistant</h1>
<h3 align="center">Standalone AI firmware for ESP32-S3 — no computer, no coding, just WiFi</h3>

<p align="center">
  <b>🌐 Language：</b>
  <a href="README.md"><b>English</b></a> ·
  <a href="README.zh-TW.md">繁體中文</a> ·
  <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <a href="https://github.com/ry6612-pixel/Novaclaw/stargazers"><img src="https://img.shields.io/github/stars/ry6612-pixel/Novaclaw?style=for-the-badge&color=gold" alt="Stars"></a>
  <a href="#quick-start"><img src="https://img.shields.io/badge/⚡_Ready_in_5_min-blue?style=for-the-badge" /></a>
  <a href="#hardware"><img src="https://img.shields.io/badge/💰_Hardware_~US$10-green?style=for-the-badge" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-orange?style=for-the-badge" /></a>
</p>

<p align="center">
  <b>A complete AI assistant running on a $10 microcontroller.</b><br/>
  No server. No PC. No code. Just plug in, connect WiFi, and <b>program it in plain English</b>.
</p>

---

## What is Novaclaw?

**Novaclaw** is a standalone AI firmware that runs entirely on an ESP32-S3 chip. It connects to Telegram and Google Gemini to give you a fully autonomous AI assistant — camera vision, voice interaction, PC remote control, scheduled automation — all from a board that costs less than a coffee.

**No computer needed after initial flash.** The ESP32 connects directly to WiFi and runs 24/7 on its own.

> 💬 **"Program it in plain language."** Instead of writing code, you just *tell* Novaclaw what to do:
> - `"Take a photo every 5 minutes and alert me if anyone enters the room"`
> - `"Remind me to drink water every 30 minutes"`
> - `"If the temperature sensor reads above 35°C, open a browser and send an email"`
>
> Novaclaw parses your intent, creates the automation, and runs it — no syntax, no IDE, no deployment.

### Why does this exist?

| Traditional AI Setup | Novaclaw |
|---|---|
| $500+ PC/Server always running | **$10 ESP32 board, 0.5W idle** |
| Complex Docker/Python setup | **Flash once, done** |
| Cloud subscription fees | **Free Gemini API tier** |
| Write code to automate | **Type a sentence in Telegram** |
| Needs internet infrastructure | **Just WiFi** |

---

## 📸 Live Demo

> Novaclaw running on a ~$10 ESP32-S3-N16R8 board via Telegram.

### 🟢 System Boot & Status

Novaclaw boots and reports full system status via Telegram:

```
🤖 Novaclaw v5.2.0 Online!
├─ AI Model: Gemini 3 Flash
├─ Free RAM: 8168 KB
├─ Skills: 16 installed
├─ Voice: ESP-SR WakeNet ✅
├─ Camera: OV3660 ✅
├─ LCD: ST7789 ✅
└─ PC Driver: Connected ✅
```

All self-contained on one $10 microcontroller.

### 📷 AI Camera Vision — Detecting Real Problems

Say **"take a photo"** and Novaclaw:
1. Snaps a photo with the OV3660 camera ($3 module)
2. Uploads to Gemini Vision for analysis
3. Returns a **detailed inspection report** — identifying objects, spotting anomalies, and giving actionable recommendations

```
User: take a photo and check the AC unit
Novaclaw: 📷 Photo captured. Analyzing...

🔍 Inspection Report:
  ⚠️ Damaged pipe insulation detected
  ⚠️ Non-standard mounting bracket
  ⚠️ Potential condensation risk
  ✅ Recommended: Re-wrap insulation, secure bracket with M6 bolts
```

From a $3 camera on a $10 board.

### ⏱️ Autonomous Task Scheduling

Tell Novaclaw in **plain language** — it creates recurring AutoTasks that run autonomously:

```
User: Every 10 minutes, snap a photo and check if any equipment looks broken
Novaclaw: ✅ AutoTask created: Photo + AI inspection every 10 min

User: Alert me at 8am every morning with today's weather
Novaclaw: ✅ Daily reminder set: 08:00 → weather briefing

User: When I say 'office check', take a photo and describe what you see
Novaclaw: ✅ Trigger skill created: "office check" → camera snap + description
```

**No cron syntax. No Python. No YAML. Just type what you want.**

### 📋 Full Command Reference

40+ commands organized by category: System, Memory, Schedule, Skills, AutoTasks, PC Control, OTA — all accessible via Telegram.

---

## ⭐ Key Features

| | Feature | Description |
|---|---|---|
| 🧠 | **Gemini AI Chat** | Full conversational AI in any language with context memory |
| 📷 | **Camera Vision** | OV3660 snap + Gemini Vision analysis — object detection, OCR, anomaly reports |
| 🎙️ | **Voice I/O** | Speech-to-text, TTS playback, wake word detection (ESP-SR WakeNet) |
| 🖥️ | **PC Remote Control** | USB Serial → screenshots, shell commands, file operations, app launching |
| 📅 | **Smart Scheduling** | Natural language task creation, daily reminders, recurring AutoTasks |
| 🧰 | **16 AI Skills** | Code execution, image gen, Excel, email, calculator, translator, and more |
| 🖼️ | **LCD Display** | ST7789 screen showing emoji faces, status, and AI replies |
| 🔄 | **OTA Updates** | Send a .bin file via Telegram to upgrade firmware wirelessly |
| 📡 | **WiFi Auto-Recovery** | Dual-network failover, auto-reconnect, 120s health checks |
| ⏸️ | **Emergency Controls** | `/pause`, `/stop`, `/shutdown` — instant halt with rate limiting (max 5 AI calls/min) |
| � | **Natural Language Programming** | Create automations, schedules, and tasks by typing plain sentences — no code needed |
| 🔒 | **Security Controls** | PC Safe Mode (AI commands blocked by default), OTA HTTPS-only, stream token auth, chat content log masking |

---

## 🏗️ Real-World Use Cases

| Scenario | How | Example |
|----------|-----|---------|
| 🏭 **Factory Automation** | Replace PLCs with scheduler + GPIO | Defect screening, equipment monitoring |
| 🏗️ **Construction Safety** | Periodic photo capture + AI analysis | Hard-hat detection, restricted zone alerts |
| 🏠 **Smart Home** | Standalone intelligent home unit | Door recognition, elderly safety, package monitoring |
| 💻 **Remote Desktop** | USB-connected PC control via Telegram | Screenshots, launch apps, run scripts, send emails |
| 🏢 **Office Assistant** | Natural-language task management | Invoice scanning, auto-scheduling, meeting reminders |

---

<a id="hardware"></a>
## 📦 Hardware — What You Need

### Core (Required) — ~$10

| Component | Spec | Price | Note |
|-----------|------|-------|------|
| **ESP32-S3-N16R8** | 16MB Flash / 8MB PSRAM | ~$8 | Any ESP32-S3 N16R8 board works |

> **Minimum setup: Just one ESP32-S3 board + USB cable = full Telegram AI assistant!**

### Optional Modules (Plug & Play)

| Module | Model | Price | Enables | Without it? |
|--------|-------|-------|---------|-------------|
| 📷 Camera | OV3660 | ~$3 | `/camera snap`, AI Vision | Vision disabled, everything else works |
| 🖥️ LCD | ST7789 240×320 | ~$2 | Emoji face, status display | No display, all features still work |
| 🔊 Speaker | MAX98357 I2S | ~$1 | TTS voice output | Silent operation |
| 🎤 Microphone | INMP441 I2S | ~$1 | Voice input, wake word, PTT | Text-only, fully functional |

**Full setup with all modules: ~$15.** That's a complete AI assistant with camera, screen, voice, and speaker.

---

<a id="quick-start"></a>
## ⚡ Quick Start — 5 Minutes to Your AI Assistant

### Prerequisites (3 Keys)

You need three things before flashing:

| Key | Where to Get | Time |
|-----|-------------|------|
| 🤖 **Telegram Bot Token** | [@BotFather](https://t.me/BotFather) → `/newbot` | 1 min |
| 🧠 **Gemini API Key** | [Google AI Studio](https://aistudio.google.com/apikey) | 1 min |
| 📶 **WiFi Name + Password** | Your home/office WiFi | 0 min |

### Method 1: Pre-built Binary (No Coding)

1. Download the latest `.bin` from [Releases](https://github.com/ry6612-pixel/Novaclaw/releases)
2. Create `user_config.txt` on the ESP32's flash (or use the setup tool):
   ```
   WIFI_SSID=YourWiFiName
   WIFI_PASS=YourWiFiPassword
   TG_TOKEN=123456:ABC-YourBotToken
   GEMINI_KEY=AIzaSyYourGeminiKey
   CHAT_ID=YourTelegramChatID
   ```
3. Flash:
   ```bash
   espflash flash -p COM3 novaclaw-v5.2.0.bin
   ```
4. Open Telegram → talk to your bot. Done! 🎉

### Method 2: Build from Source

```bash
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw

# Recommended: move secrets outside the repo (Windows)
.\setup-secure-config.ps1
notepad $HOME/.novaclaw/secrets/user_config.txt

# Legacy fallback (less safe, still supported)
cp user_config.example.txt user_config.txt
notepad user_config.txt

# Build & flash
cargo build --release
espflash flash -p COM3 --partition-table partitions.csv target/xtensa-esp32s3-espidf/release/esp32
```

`build.ps1` will look for secrets in this order:
1. `NOVACLAW_CONFIG`
2. `NOVACLAW_CONFIG_DIR/user_config.txt`
3. `$HOME/.novaclaw/secrets/user_config.txt`
4. repo-local `user_config.txt` (legacy fallback)

<details>
<summary><b>🔧 Build Prerequisites</b></summary>

```bash
# Install Rust + ESP32 toolchain
rustup install nightly
cargo install espup espflash
espup install

# Set environment
. ~/export-esp.sh  # Linux/Mac
# or run build.ps1 on Windows
```

</details>

---

## 🎮 Command Reference

### System

| Command | Description |
|---------|-------------|
| `/help` | Full command list |
| `/status` | System info (RAM, uptime, model, tokens) |
| `/model` | Current AI model info |
| `/model set flash\|pro\|3.1-pro` | Switch Gemini model |
| `/tokens` | Token usage stats |
| `/diag` | Self-diagnostics |
| `/wifi` | WiFi status |
| `/wifi set SSID password` | Configure WiFi |

### AI & Memory

| Command | Description |
|---------|-------------|
| *(any text)* | AI conversation (auto-detects language) |
| `/memories` | View saved memories |
| `/remember key value` | Save a memory |
| `/forget key` | Delete a memory |

### Camera & Vision

| Command | Description |
|---------|-------------|
| `/camera snap` | Take photo → send to Telegram |
| `/camera vision` | Take photo → Gemini AI analysis |
| `/camera vision <prompt>` | Take photo → custom AI analysis |

### Scheduling

| Command | Description |
|---------|-------------|
| `/remind 5m message` | Remind every 5 minutes |
| `/remind 8:00 message` | Daily reminder at 8:00 |
| `/tasks` | List background AutoTasks |
| `/tasks del ID` | Stop a specific task |
| `/tasks clear` | Clear all tasks |

### Emergency Controls

| Command | Description |
|---------|-------------|
| `/pause` | **Halt all AI calls immediately** |
| `/resume` | Resume AI processing |
| `/stop` | Emergency stop all tasks + pause |
| `/shutdown` | Full shutdown — stop AI, disable reminders, clear tasks |

### PC Control (USB Connected)

| Command | Description |
|---------|-------------|
| `/pc shell <command>` | Run shell command on PC |
| `/screenshot` | Capture PC desktop |
| `/pc type <text>` | Type text on PC |
| `/pc hotkey ctrl+c` | Send keyboard shortcut |
| `/pc open <file/url>` | Open file or URL |

### Voice & Audio

| Command | Description |
|---------|-------------|
| `/voice off\|normal\|brief` | Voice reply mode |
| `/audio wake on\|off` | Toggle wake word detection |
| `/audio wake set <word>` | Set custom wake word |

### OTA & Maintenance

| Command | Description |
|---------|-------------|
| `/ota <url>` | OTA update from URL |
| Send `.bin` file | OTA update via Telegram |
| `/reset` | Factory reset |

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────┐
│              Telegram Bot API               │
│         (commands, voice, photos)            │
└──────────────────┬──────────────────────────┘
                   │ HTTPS
┌──────────────────▼──────────────────────────┐
│            ESP32-S3 (Novaclaw)                │
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

## Wiring Diagrams

<details>
<summary><b>📷 OV3660 Camera (DVP parallel)</b></summary>

| OV3660 Pin | ESP32-S3 GPIO | Function |
|------------|---------------|----------|
| XCLK | GPIO 15 | Clock |
| SDA | GPIO 4 | I2C Data |
| SCL | GPIO 5 | I2C Clock |
| D0–D7 | GPIO 11,9,8,10,12,18,17,16 | Data bus |
| VSYNC | GPIO 6 | Vertical sync |
| HREF | GPIO 7 | Horizontal ref |
| PCLK | GPIO 13 | Pixel clock |

> Custom pins? Send `/camera pins d0=11 d1=9 ...` via Telegram — saved to NVS permanently.

</details>

<details>
<summary><b>🖥️ ST7789 LCD (SPI)</b></summary>

| ST7789 Pin | ESP32-S3 GPIO | Function |
|------------|---------------|----------|
| RST | GPIO 21 | Reset |
| DC | GPIO 47 | Data/Command |
| BL | GPIO 38 | Backlight |
| SCLK | GPIO 19 | SPI Clock |
| SDA (MOSI) | GPIO 20 | SPI Data |
| CS | GPIO 45 | Chip Select |

> Adjust via: `/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45 w=240 h=320`

</details>

<details>
<summary><b>🔊 MAX98357 Speaker + 🎤 INMP441 Mic (I2S)</b></summary>

**Speaker (MAX98357):**
| Pin | ESP32-S3 GPIO |
|-----|---------------|
| BCLK | GPIO 46 |
| LRC | GPIO 14 |
| DIN | GPIO 48 |

**Microphone (INMP441):**
| Pin | ESP32-S3 GPIO |
|-----|---------------|
| SCK | GPIO 2 |
| WS | GPIO 1 |
| SD | GPIO 41 |

</details>

---

## 🔑 Security

**What's protected:**
- **Zero secrets in source code** — credentials are loaded from an external secret file or `user_config.txt` (gitignored)
- **Repo-external secret store** — `build.ps1` prefers `$HOME/.novaclaw/secrets/user_config.txt` or `NOVACLAW_CONFIG`
- **Chat ID verification** — only responds to authorized Telegram users
- **NVS credential storage** — sensitive data stored in ESP32's NVS partition
- **PC Safe Mode** (default ON) — AI-generated PC commands (`shell`, `python`, `file_write`, `email`) are blocked; only safe read-only commands pass. Toggle: `/pc_unlock` / `/pc_lock`
- **OTA HTTPS-only** — firmware updates via URL must use HTTPS
- **MJPEG stream token** — camera stream requires random session token in URL
- **Chat content log masking** — user text, voice transcripts, and PTT are not logged to serial
- **Rate limiting** — max 5 Gemini API calls per 60 seconds to prevent runaway costs
- **Emergency pause** — `/pause` instantly halts all AI processing
- **URL log masking** — bot tokens are stripped from HTTP request logs

**Known risks (use at your own discretion):**
- This is a **high-privilege remote control system** — PC commands, OTA updates, and AI automation can modify your computer
- When `/pc_unlock` is active, AI responses can trigger shell commands, file writes, and program execution on your PC
- Voice, photos, and chat history are sent to Google Gemini API for processing
- Custom TTS proxy URLs can route text to third-party services
- OTA via Telegram file upload does not verify firmware signatures
- MJPEG camera stream uses HTTP (not HTTPS) on local network

> ⚠️ **This project is designed for personal experimentation.** If you connect it to your real Telegram account and Windows PC, understand the security boundary. Review the source code before deployment.

---

## FAQ

<details>
<summary><b>How much does Gemini API cost?</b></summary>

Google offers a generous free tier for Gemini API. For personal use (a few hundred requests/day), it costs **$0**. Novaclaw also has built-in rate limiting (5 calls/min) to prevent accidental overuse.

</details>

<details>
<summary><b>Can I use this without a camera/LCD/speaker?</b></summary>

Yes! The minimum setup is just an ESP32-S3 board. Camera, LCD, speaker, and microphone are all optional — Novaclaw auto-detects what's connected.

</details>

<details>
<summary><b>Does it work in English / Chinese / Japanese / ...?</b></summary>

Yes. Novaclaw uses Gemini's multilingual capabilities. It responds in whatever language you write to it.

</details>

<details>
<summary><b>Can I flash from my phone?</b></summary>

Initial flashing requires a computer (Windows/Mac/Linux). After that, the board runs independently — no PC needed. Future OTA updates can be done via Telegram.

</details>

<details>
<summary><b>What about xiaozhi-esp32?</b></summary>

xiaozhi-esp32 is a great project focused on voice chat. Novaclaw focuses on **autonomous edge AI** — camera vision, scheduled automation, PC control, and **natural-language programming** (create automations by typing plain sentences). Different goals, complementary approaches.

</details>

---

## 🤝 Contributing

PRs welcome!

1. Fork this repo
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Commit (`git commit -m 'Add amazing feature'`)
4. Push (`git push origin feature/amazing`)
5. Open a Pull Request

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) / [Apache-2.0](LICENSE-APACHE).

---

## 🙏 Acknowledgments

- **[ZeroClaw Labs](https://github.com/zeroclaw-labs/zeroclaw)** — The architectural patterns, Rust-based AI agent design, and the vision of running AI on minimal hardware were directly inspired by the ZeroClaw project. Some early development referenced their codebase structure. ETHAN is an independent implementation focused specifically on embedded ESP32 firmware, but we gratefully acknowledge ZeroClaw's influence on this project.
- **[Espressif](https://www.espressif.com/)** — For the incredible ESP32-S3 chip and ESP-IDF framework
- **[Google Gemini](https://ai.google.dev/)** — For the multimodal AI API that makes edge intelligence possible
- **[xiaozhi-esp32](https://github.com/78/xiaozhi-esp32)** — A pioneering ESP32 AI project that proved the concept
- **[ethan-esp32](https://github.com/ry6612-pixel/ethan-esp32)** — Earlier version of this project, now evolved into Novaclaw

---

## ⭐ Star History

<p align="center">
  <a href="https://www.star-history.com/#ry6612-pixel/Novaclaw&Date">
    <img src="https://api.star-history.com/svg?repos=ry6612-pixel/Novaclaw&type=Date" alt="Star History Chart" width="600" />
  </a>
</p>

---

<p align="center">
  <b>⭐ If Novaclaw saves you time or money, give it a Star!</b><br/>
  <sub>Built with ❤️ in Taiwan — Powered by ESP32-S3 + Gemini AI</sub>
</p>
