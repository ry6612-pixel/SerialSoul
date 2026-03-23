<h1 align="center">
  <br>
  <img src="docs/demo/01-boot-status.png" alt="Novaclaw Boot" width="280">
  <br>
  Novaclaw
  <br>
</h1>

<h3 align="center">A complete AI assistant running on a $10 microcontroller.<br>No server. No PC. No code. Just WiFi.</h3>

<p align="center">
  <a href="README.md"><b>English</b></a> ·
  <a href="README.zh-TW.md">繁體中文</a> ·
  <a href="README.ja.md">日本語</a>
</p>

<p align="center">
  <a href="https://github.com/ry6612-pixel/Novaclaw/stargazers"><img src="https://img.shields.io/github/stars/ry6612-pixel/Novaclaw?style=for-the-badge&color=gold" alt="Stars"></a>&nbsp;
  <a href="#quick-start--5-minutes"><img src="https://img.shields.io/badge/Ready_in_5_min-blue?style=for-the-badge" /></a>&nbsp;
  <a href="#hardware"><img src="https://img.shields.io/badge/~US$10-green?style=for-the-badge" /></a>&nbsp;
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-orange?style=for-the-badge" /></a>
</p>

---

## What is Novaclaw?

**Novaclaw** is standalone AI firmware that runs entirely on an ESP32-S3 chip. It connects to Telegram and Google Gemini to give you a fully autonomous AI assistant — camera vision, voice interaction, PC remote control, scheduled automation — all from a board that costs less than a coffee.

> **"Program it in plain language."** Instead of writing code, just *tell* Novaclaw what to do:
> - *"Take a photo every 5 minutes and alert me if anyone enters the room"*
> - *"Remind me to drink water every 30 minutes"*
> - *"If the temperature sensor reads above 35°C, send me an email alert"*
>
> Novaclaw parses your intent, creates the automation, and runs it — no syntax, no IDE, no deployment.

| Traditional AI Setup | Novaclaw |
|---|---|
| $500+ PC always running | **$10 ESP32 board, 0.5W idle** |
| Complex Docker/Python setup | **Flash once, done** |
| Cloud subscription fees | **Free Gemini API tier** |
| Write code to automate | **Type a sentence in Telegram** |

---

## Novaclaw vs OpenClaw vs ZeroClaw

Novaclaw belongs to a different category than OpenClaw and ZeroClaw. Those are server-side AI assistant platforms that you run on a desktop or cloud instance. Novaclaw is standalone firmware that runs directly on a $10 microcontroller with zero server dependency.

**The core difference: Novaclaw IS the device. OpenClaw and ZeroClaw USE devices as peripherals.**

| | Novaclaw | ZeroClaw | OpenClaw |
|---|---|---|---|
| Architecture | Standalone MCU firmware | CLI/Gateway on desktop/server | Node.js Gateway on desktop/server |
| Language | Rust (esp-idf) | Rust | TypeScript |
| Runs on | ESP32-S3 ($10 board) | Desktop, SBC, cloud | Desktop, server |
| Server required | None | Self-hosted gateway | Self-hosted gateway |
| RAM | 8 MB (on-chip PSRAM) | < 5 MB | > 1 GB |
| Binary | ~1.8 MB firmware | ~8.8 MB | ~28 MB (dist) |
| Channels | Telegram | 22+ channels | 22+ channels |
| Hardware | Built-in camera, LCD, speaker, mic | Peripheral trait (ESP32, RPi) | None |
| Skills | 16 on-device | 70+ tools + plugins | 5,400+ community |
| Setup | Flash once, provision via USB | install.sh + onboard wizard | Node.js + config |

**Choose Novaclaw when you want:**
- A self-contained AI device that works without any server or PC
- Physical sensing: camera vision, environmental monitoring, voice I/O
- Edge AI that costs $10 and draws 0.5W idle
- Automations created by typing sentences, not writing code

**Choose ZeroClaw or OpenClaw when you need:**
- 22+ messaging channels (WhatsApp, Slack, Discord, Signal, etc.)
- A plugin ecosystem with thousands of community skills
- Multi-agent orchestration or browser automation
- A server or desktop you already have to run a gateway

---

## Live Demo

> Real screenshots from Novaclaw running on a ~$10 ESP32-S3-N16R8 board.

<table>
<tr>
<td align="center" width="33%">
<img src="docs/demo/01-boot-status.png" width="240"><br>
<b>Boot & Status</b><br>
<sub>System boots and reports full status via Telegram — firmware version, AI model, RAM, connected peripherals, installed skills.</sub>
</td>
<td align="center" width="33%">
<img src="docs/demo/02-help-menu.png" width="240"><br>
<b>Command Menu</b><br>
<sub>40+ commands organized by category: System, Memory, Schedule, Skills, Camera, Audio, PC Control, OTA.</sub>
</td>
<td align="center" width="33%">
<img src="docs/demo/03-ai-chat.png" width="240"><br>
<b>AI Conversation</b><br>
<sub>Full conversational AI in any language. Just type — Novaclaw responds with Gemini intelligence.</sub>
</td>
</tr>
<tr>
<td align="center">
<img src="docs/demo/05-camera-snap.png" width="240"><br>
<b>Camera Snap</b><br>
<sub>Say "take a photo" — OV3660 captures, uploads, and sends the image back to your Telegram chat.</sub>
</td>
<td align="center">
<img src="docs/demo/06-camera-vision.png" width="240"><br>
<b>AI Vision Analysis</b><br>
<sub>Gemini Vision inspects photos — detects objects, reads text, spots anomalies, gives actionable advice.</sub>
</td>
<td align="center">
<img src="docs/demo/07-pc-control.png" width="240"><br>
<b>PC Remote Control</b><br>
<sub>Screenshots, shell commands, file operations, app launching — all through Telegram via USB Serial.</sub>
</td>
</tr>
<tr>
<td align="center">
<img src="docs/demo/04-skills-list.png" width="240"><br>
<b>16 AI Skills</b><br>
<sub>Code execution, image generation, Excel processing, email, calculator, translator, and more.</sub>
</td>
<td align="center">
<img src="docs/demo/08-auto-tasks.png" width="240"><br>
<b>AutoTasks</b><br>
<sub>Create recurring AI tasks in plain language — "every 10 min, snap a photo and check for anomalies."</sub>
</td>
<td align="center">
<img src="docs/demo/09-scheduling.png" width="240"><br>
<b>Smart Scheduling</b><br>
<sub>Daily reminders, interval tasks, trigger-based skills — all created with natural language.</sub>
</td>
</tr>
</table>

---

## Key Features

| | Feature | Description |
|---|---|---|
| | **Gemini AI Chat** | Full conversational AI with context memory, any language |
| | **Camera Vision** | OV3660 snap + Gemini Vision -- object detection, OCR, anomaly reports |
| | **Voice I/O** | Speech-to-text, TTS playback, wake word detection (ESP-SR) |
| | **PC Remote Control** | USB Serial -- screenshots, shell, file ops, app launching |
| | **Smart Scheduling** | Natural language task creation, daily reminders, recurring AutoTasks |
| | **16 AI Skills** | Code execution, image gen, Excel, email, calculator, translator |
| | **LCD Display** | ST7789 screen -- status, AI replies |
| | **OTA Updates** | Send .bin via Telegram to upgrade firmware wirelessly |
| | **WiFi Auto-Recovery** | Dual-network failover, auto-reconnect, 120s health checks |
| | **Natural Language Programming** | Create automations by typing plain sentences -- no code |
| | **Security Controls** | PC Safe Mode, OTA HTTPS-only, stream token auth, chat log masking |

---

## Use Cases

| Scenario | How | Example |
|----------|-----|---------|
| **Factory** | Scheduler + GPIO + AI vision | Defect screening, equipment monitoring |
| **Construction** | Periodic photo + AI analysis | Hard-hat detection, zone alerts |
| **Smart Home** | Standalone intelligent unit | Door recognition, elderly safety |
| **Remote Desktop** | USB-connected PC control | Screenshots, scripts, emails via Telegram |
| **Office** | Natural-language task management | Invoice scanning, auto-scheduling |

---

<a id="hardware"></a>

## Hardware

### Core (Required) — ~$10

| Component | Spec | Price |
|-----------|------|-------|
| **ESP32-S3-N16R8** | 16MB Flash / 8MB PSRAM | ~$8 |

> **Minimum: Just one ESP32-S3 board + USB cable = full Telegram AI assistant.**

### Optional Modules

| Module | Price | Enables |
|--------|-------|---------|
| OV3660 Camera | ~$3 | Camera snap, AI vision |
| ST7789 LCD | ~$2 | Status display |
| MAX98357 Speaker | ~$1 | TTS voice output |
| INMP441 Microphone | ~$1 | Voice input, wake word |

**Full setup: ~$15** — complete AI assistant with camera, screen, voice, and speaker.

---

<a id="quick-start--5-minutes"></a>

## Quick Start — 5 Minutes

### Prerequisites

| Key | Where to Get |
|-----|-------------|
| **Telegram Bot Token** | [@BotFather](https://t.me/BotFather) -> `/newbot` |
| **Gemini API Key** | [Google AI Studio](https://aistudio.google.com/apikey) |
| **WiFi SSID + Password** | Your home/office WiFi |

### Build from Source

```bash
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw

# 1. Set up your secret config (stored outside the repo)
.\setup-secure-config.ps1
notepad $HOME/.novaclaw/secrets/user_config.txt

# 2. Build (zero secrets in binary)
.\build.ps1

# 3. Flash
.\flash.ps1 -Port COM3

# 4. Provision secrets into device NVS via USB serial
.\provision.ps1 -Port COM3
```

> **Security:** Secrets are **never** compiled into the firmware binary.
> They are sent to the device over USB serial at first boot and stored in
> on-chip NVS flash. The `.bin` file is safe to distribute.

<details>
<summary><b>Build Prerequisites</b></summary>

```bash
rustup install nightly
cargo install espup espflash
espup install
. ~/export-esp.sh   # Linux/Mac
# Windows: build.ps1 handles environment
```

</details>

<details>
<summary><b>Secret Configuration Priority</b></summary>

`provision.ps1` looks for secrets in this order:
1. `-Config` parameter
2. `NOVACLAW_CONFIG` environment variable
3. `NOVACLAW_CONFIG_DIR/user_config.txt`
4. `$HOME/.novaclaw/secrets/user_config.txt`
5. Repo-local `user_config.txt` (legacy fallback)

</details>

---

## Commands

### Core

| Command | Description |
|---------|-------------|
| *(any text)* | AI conversation (auto-detects language) |
| `/help` | Full command list |
| `/status` | System info |
| `/model set flash\|pro` | Switch Gemini model |
| `/tokens` | Token usage stats |
| `/diag` | Self-diagnostics |

### Camera & Vision

| Command | Description |
|---------|-------------|
| `/camera snap` | Take photo → Telegram |
| `/camera vision` | Photo → Gemini AI analysis |
| `/camera vision <prompt>` | Photo → custom AI query |
| `/camera stream` | Start MJPEG stream |

### Scheduling

| Command | Description |
|---------|-------------|
| `/remind 5m message` | Recurring reminder |
| `/remind 8:00 message` | Daily at 8:00 |
| `/tasks` | List AutoTasks |
| `/stop` | Emergency stop all |
| `/pause` / `/resume` | Halt/resume AI processing |

### Memory & Skills

| Command | Description |
|---------|-------------|
| `/memories` | View saved memories |
| `/remember key value` | Save a memory |
| `/skills` | List AI skills |
| `/skill add name\|desc\|trigger` | Create custom skill |

### PC Control (USB Connected)

| Command | Description |
|---------|-------------|
| `/pc shell <cmd>` | Run shell command |
| `/screenshot` | Capture PC desktop |
| `/pc type <text>` | Keyboard input |
| `/pc hotkey ctrl+c` | Send shortcut |
| `/pc open <file/url>` | Open file or URL |

### Voice & Audio

| Command | Description |
|---------|-------------|
| `/voice off\|normal\|brief` | Voice reply mode |
| `/audio wake on\|off` | Wake word detection |
| `/audio transcribe` | Record → speech-to-text |

### WiFi & Maintenance

| Command | Description |
|---------|-------------|
| `/wifi set SSID password` | Configure WiFi |
| `/ota <https://url>` | OTA firmware update |
| Send `.bin` file | OTA via Telegram |
| `/reset` | Factory reset |

---

## Architecture

```
┌─────────────────────────────────────────────┐
│              Telegram Bot API               │
└──────────────────┬──────────────────────────┘
                   │ HTTPS
┌──────────────────▼──────────────────────────┐
│            ESP32-S3 (Novaclaw)              │
│                                             │
│  ┌──────────┐ ┌──────────┐ ┌─────────────┐ │
│  │ Gemini   │ │ ESP-SR   │ │ AutoTask    │ │
│  │ AI Chat  │ │ WakeNet  │ │ Scheduler   │ │
│  └──────────┘ └──────────┘ └─────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌─────────────┐ │
│  │ OV3660   │ │ ST7789   │ │ I2S Audio   │ │
│  │ Camera   │ │ LCD      │ │ Spk + Mic   │ │
│  └──────────┘ └──────────┘ └─────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌─────────────┐ │
│  │ NVS      │ │ OTA      │ │ USB Serial  │ │
│  │ Memory   │ │ Update   │ │ PC Driver   │ │
│  └──────────┘ └──────────┘ └─────────────┘ │
└─────────────────────────────────────────────┘
         │ USB Serial (JSON Lines)
┌────────▼────────────────────────────────────┐
│  PC — Shell / Screenshot / Email / Files    │
└─────────────────────────────────────────────┘
```

---

## Wiring

<details>
<summary><b>OV3660 Camera (DVP)</b></summary>

| Pin | GPIO | | Pin | GPIO |
|-----|------|-|-----|------|
| XCLK | 15 | | D4 | 12 |
| SDA | 4 | | D5 | 18 |
| SCL | 5 | | D6 | 17 |
| D0 | 11 | | D7 | 16 |
| D1 | 9 | | VSYNC | 6 |
| D2 | 8 | | HREF | 7 |
| D3 | 10 | | PCLK | 13 |

> Custom pins? `/camera pins d0=11 d1=9 ...` via Telegram — saved to NVS.

</details>

<details>
<summary><b>ST7789 LCD (SPI)</b></summary>

RST=21, DC=47, BL=38, SCLK=19, MOSI=20, CS=45

> `/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45 w=240 h=320`

</details>

<details>
<summary><b>Speaker + Mic (I2S)</b></summary>

**MAX98357:** BCLK=46, LRC=14, DIN=48
**INMP441:** SCK=2, WS=1, SD=41

</details>

---

## Security

**Protected:**
- **Zero secrets in binary** — credentials provisioned via USB serial into NVS, never compiled into firmware
- Zero secrets in source — credentials via external config file (`~/.novaclaw/secrets/`)
- Chat ID verification — only responds to authorized users
- PC Safe Mode (default ON) — AI commands blocked; toggle `/pc_unlock` / `/pc_lock`
- OTA HTTPS-only
- MJPEG stream requires session token
- Chat content not logged to serial
- Rate limiting — max 5 AI calls/min

**Known risks:**
- High-privilege remote control — PC commands, OTA, AI automation can modify your computer
- Voice, photos, chat sent to Google Gemini API
- OTA via Telegram doesn't verify firmware signatures
- MJPEG stream uses HTTP on local network

> ⚠️ **Designed for personal experimentation.** Review source code before deployment.

---

## Release Checklist

Before making changes public:

```powershell
.\pre-publish-scan.ps1      # Must show ALL CHECKS PASSED
.\build.ps1                  # Must compile successfully
.\flash.ps1                  # Flash to device
.\provision.ps1              # Send secrets to NVS
git status --short           # Must be clean
git push
```

---

## FAQ

<details><summary><b>How much does Gemini API cost?</b></summary>
Free tier handles hundreds of requests/day. Built-in rate limiting (5 calls/min) prevents overuse.
</details>

<details><summary><b>Works without camera/LCD/speaker?</b></summary>
Yes. Minimum is just ESP32-S3. All peripherals are optional and auto-detected.
</details>

<details><summary><b>Supports English / Chinese / Japanese / ...?</b></summary>
Yes. Responds in whatever language you write.
</details>

<details><summary><b>What about xiaozhi-esp32?</b></summary>
xiaozhi-esp32 focuses on voice chat. Novaclaw focuses on autonomous edge AI -- camera vision, scheduling, PC control, and natural-language programming. Complementary projects.
</details>

<details><summary><b>How is Novaclaw different from OpenClaw or ZeroClaw?</b></summary>
OpenClaw and ZeroClaw are server-side AI platforms that run on a desktop or cloud instance and connect to 22+ messaging channels. Novaclaw is standalone firmware that runs directly on a $10 ESP32-S3 -- no server, no PC, no Docker. See the comparison table above for details.
</details>

---

## Contributing

PRs welcome! Fork -> branch -> commit -> PR.

---

## License

Dual-licensed under [MIT](LICENSE-MIT) / [Apache-2.0](LICENSE-APACHE).

## Acknowledgments

- **[Espressif](https://www.espressif.com/)** — ESP32-S3 chip and ESP-IDF framework
- **[Google Gemini](https://ai.google.dev/)** — Multimodal AI API
- **[xiaozhi-esp32](https://github.com/78/xiaozhi-esp32)** — Pioneering ESP32 AI project
