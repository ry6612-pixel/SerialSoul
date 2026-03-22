<h1 align="center">🤖 Novaclaw — あなたの $10 AI アシスタント</h1>
<h3 align="center">ESP32-S3 で動作するスタンドアロン AI ファームウェア — PC不要、コーディング不要、WiFiだけ</h3>

<p align="center">
  <b>🌐 Language / 語言：</b>
  <a href="README.md">English</a> ·
  <a href="README.zh-TW.md">繁體中文</a> ·
  <a href="README.ja.md"><b>日本語</b></a>
</p>

<p align="center">
  <a href="#クイックスタート"><img src="https://img.shields.io/badge/⚡_5分で起動-blue?style=for-the-badge" /></a>
  <a href="#hardware"><img src="https://img.shields.io/badge/💰_ハードウェア_~¥1500-green?style=for-the-badge" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-orange?style=for-the-badge" /></a>
</p>

<p align="center">
  <b>従来のPLCや複雑なAIシステムに高額な費用をかける時代は終わりです。</b><br/>
  Novaclawにインスパイアされ、ESP32-S3向けに独自開発した Novaclaw = リアルタイム、ノーコードのエッジAI + オートメーション、約1500円のハードウェアで。
</p>

---

## 💡 これは何？

**Novaclaw**（**E**dge **T**echnology **H**ardware **A**I **N**ode）は ESP32-S3 で動作する **スタンドアロン AI ファームウェア** です。PCもクラウドサーバーも不要——約1500円の開発ボード + WiFi だけで：

- 🤖 **Telegram AI アシスタント**（多言語チャット、音声認識、画像分析）
- 📷 **エッジビジョン**（OV3660 + Gemini Vision、ローカル撮影 + クラウド分析）
- 🔌 **低コストオートメーション**（GPIO制御、スケジューラ、自然言語タスク作成）
- 🖥️ **リモートPC制御**（USB Serial経由 — スクリーンショット、アプリ起動、コマンド実行）
- 🔄 **OTAアップデート**（Telegram で .bin ファイルを送信するだけでファームウェア更新）

---

## ⭐ 主な特長

| | 特長 | 説明 |
|---|---|---|
| ⚡ | **ノーコード、即座にデプロイ** | WiFi + API Key を入力 → フラッシュ → 5分でエッジAI展開 |
| 👁️ | **AIビジョン** | 物体認識、テキスト/バーコード読取、安全検知 |
| 🛠️ | **格安PLCキラー** | ~¥1500のハードウェアで軽量PLCを置き換え |
| 🎙️ | **音声インタラクション** | 音声認識 + TTS + ウェイクワード検知 |
| 📡 | **WiFi自動復旧** | デュアルネットワーク自動切替、120秒ヘルスチェック |
| 🔒 | **ソースコードにシークレットゼロ** | 全ての認証情報は設定ファイルで注入 |

---

<a id="hardware"></a>
## 📦 ハードウェア

### コアボード（必須）

| 部品 | スペック | 価格 | 備考 |
|------|---------|------|------|
| **ESP32-S3-N16R8 開発ボード** | 16MB Flash / 8MB PSRAM | ~¥1,500 | GuoYun ブランド動作確認済み |

> **最小構成：ESP32-S3 + USBケーブル1本で、完全なTelegram AIアシスタントが動きます！**

### オプションモジュール（プラグ＆プレイ）

| モジュール | 型番 | 価格 | 機能 | なくても？ |
|-----------|------|------|------|-----------|
| 📷 カメラ | OV3660 | ~¥400 | 撮影、AI分析 | ビジョン無効、他は正常 |
| 🖥️ LCD | ST7789 240×320 | ~¥300 | 表情・状態表示 | 表示なし、機能に影響なし |
| 🔊 スピーカー | MAX98357 I2S | ~¥150 | TTS音声、通知音 | 音声出力なし |
| 🎤 マイク | INMP441 I2S | ~¥150 | 音声認識、ウェイクワード | 音声入力なし |

---

<a id="クイックスタート"></a>
## 🎯 事前準備 — 3つのキー

### 🔑 キー1：Telegram Bot Token + Chat ID

1. Telegram で **`@BotFather`** を検索 → `/newbot` → Token をコピー
2. **`@userinfobot`** を検索 → 数字の Chat ID を取得
3. 自分のBotに「Hi」を送信

### 🔑 キー2：Google Gemini API Key

**[aistudio.google.com/apikey](https://aistudio.google.com/apikey)** → 「API キーを作成」→ コピー（無料）

### 🔑 キー3：WiFi の SSID とパスワード

---

## ⚡ 方法1：.bin を直接フラッシュ（コンパイル不要、最も簡単！）

1. [Flash Download Tool](https://www.espressif.com/en/support/download/other-tools) をダウンロード
2. [GitHub Releases](https://github.com/ry6612-pixel/Novaclaw/releases) から最新の .bin をダウンロード
3. ESP32-S3 をUSBで接続
4. Flash Download Tool で ESP32-S3 を選択 → .bin ファイル → アドレス `0x0` → START

```bash
# CLI（代替案）
espflash flash -p COM3 Novaclaw-v5.2.0.bin
```

---

## 🔧 方法2：ソースからビルド

```powershell
# 1. Rust + ESP ツールチェーンをインストール
winget install Rustlang.Rust.MSVC
cargo install espup espflash
espup install

# 2. プロジェクトをクローン
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw

# 3. 設定ファイルを作成
Copy-Item user_config.example.txt user_config.txt
notepad user_config.txt   # WiFi / Token / API Key を入力

# 4. ビルド＆フラッシュ
.\build.ps1
.\flash.ps1 -Port COM3
```

---

## 📖 コマンド一覧

### 📶 WiFi

| コマンド | 機能 |
|---------|------|
| `/wifi` | 現在のWiFi状態を表示 |
| `/wifi set SSID パスワード` | プライマリWiFiを設定（自動再起動） |
| `/wifi set2 SSID パスワード` | バックアップWiFiを設定 |
| `/wifi swap` | プライマリ ↔ バックアップ入替 |
| `/wifi clear` | 全WiFi設定をクリア |

### 📷 カメラ

| コマンド | 機能 |
|---------|------|
| `/camera snap` | 撮影してTelegramに送信 |
| `/camera vision` | 撮影 + AI自動分析 |
| `/camera vision これは何？` | 撮影 + 質問付きAI分析 |

### 🔧 システム

| コマンド | 機能 |
|---------|------|
| `/help` | 全コマンドを表示 |
| `/status` | システム状態 |
| `/model set pro` | AIモデル切替 |
| `/voice off/normal/brief` | 音声モード |
| `/memories` | メモリ一覧 |
| `/remember キー 値` | メモリ保存 |
| `/remind 5m 水を飲む` | リマインダー |
| `/reset` | ⚠️ 工場出荷リセット |

### 💻 リモートPC

| コマンド | 機能 |
|---------|------|
| `/pc shell <コマンド>` | PC上でコマンド実行 |
| `/screenshot` | PCスクリーンショット |
| `/pc open <対象>` | プログラム起動 |

---

## ❓ よくある質問

| 質問 | 回答 |
|------|------|
| Botが反応しない | WiFi/Token/ChatIDを確認。USBデータケーブルを使用 |
| WiFiを変更したい | `/wifi set 新SSID 新パスワード` を送信（再コンパイル不要） |
| フラッシュが中断 | BOOTボタンを押しながらRESETを押す → 再フラッシュ |

---

## 🔑 セキュリティ

- ソースコードにシークレットなし — `user_config.txt` で注入
- Chat ID 認証 — 許可されたユーザーのみ応答
- `user_config.txt` は `.gitignore` に含まれています

---

## 📄 ライセンス

[MIT](LICENSE-MIT) / [Apache-2.0](LICENSE-APACHE) デュアルライセンス

---

<p align="center">
  <b>⭐ 気に入ったら Star をお願いします！</b><br/>
  <sub>Built with ❤️ by <a href="https://github.com/ry6612-pixel">ethan</a> — Powered by ESP32-S3 + Gemini</sub>
  <br/>
  <sub>Inspired by <a href="https://github.com/78/xiaozhi-esp32">xiaozhi-esp32</a></sub>
</p>
