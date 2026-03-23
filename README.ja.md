<h1 align="center">
  <br>
  <img src="docs/demo/01-boot-status.png" alt="Novaclaw 起動" width="280">
  <br>
  Novaclaw
  <br>
</h1>

<h3 align="center">10ドルのマイコンで動く完全なAIアシスタント<br>サーバー不要・PC不要・コーディング不要 — WiFiだけ</h3>

<p align="center">
  <b>🌐</b>&nbsp;
  <a href="README.md">English</a> ·
  <a href="README.zh-TW.md">繁體中文</a> ·
  <a href="README.ja.md"><b>日本語</b></a>
</p>

<p align="center">
  <a href="https://github.com/ry6612-pixel/Novaclaw/stargazers"><img src="https://img.shields.io/github/stars/ry6612-pixel/Novaclaw?style=for-the-badge&color=gold" alt="Stars"></a>&nbsp;
  <a href="#-クイックスタート--5分"><img src="https://img.shields.io/badge/⚡_5分で開始-blue?style=for-the-badge" /></a>&nbsp;
  <a href="#-ハードウェア"><img src="https://img.shields.io/badge/💰_約¥1,500-green?style=for-the-badge" /></a>&nbsp;
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%2FApache--2.0-orange?style=for-the-badge" /></a>
</p>

---

## Novaclaw とは？

**Novaclaw** は ESP32-S3 チップ上で完全に動作するスタンドアロン AI ファームウェアです。Telegram と Google Gemini に接続し、カメラビジョン・音声操作・PC リモート制御・スケジュール自動化を備えた完全自律型 AI アシスタントを実現します — コーヒー1杯より安いボードで。

> **「自然言語でプログラミング」** — コードを書く代わりに、Novaclaw に *話しかける* だけ：
> - *「5分ごとに写真を撮って、誰か部屋に入ったら通知して」*
> - *「30分ごとに水を飲むよう教えて」*
> - *「温度が35°Cを超えたらメールで知らせて」*
>
> Novaclaw が意図を解析し、自動化タスクを作成し、自律実行します。

| 従来のAI構成 | Novaclaw |
|---|---|
| 5万円以上のPC常時稼働 | **¥1,500 の ESP32 ボード、待機 0.5W** |
| Docker/Python の複雑な設定 | **フラッシュ1回で完了** |
| クラウドのサブスク料金 | **Gemini 無料枠** |
| 自動化にはコードが必要 | **Telegram でテキスト入力するだけ** |

---

## 📸 デモ

> 実機スクリーンショット — 約¥1,500 の ESP32-S3-N16R8 ボードで稼働中。

<table>
<tr>
<td align="center" width="33%">
<img src="docs/demo/01-boot-status.png" width="240"><br>
<b>起動とステータス</b><br>
<sub>起動後 Telegram でフルステータスを報告 — FWバージョン、AIモデル、RAM、周辺機器、スキル一覧。</sub>
</td>
<td align="center" width="33%">
<img src="docs/demo/02-help-menu.png" width="240"><br>
<b>コマンドメニュー</b><br>
<sub>40以上のコマンドをカテゴリ別に整理：システム、メモリ、スケジュール、スキル、カメラ、オーディオ、PC制御、OTA。</sub>
</td>
<td align="center" width="33%">
<img src="docs/demo/03-ai-chat.png" width="240"><br>
<b>AI会話</b><br>
<sub>あらゆる言語で完全な対話AI。テキストを入力 — Gemini の知能で応答。</sub>
</td>
</tr>
<tr>
<td align="center">
<img src="docs/demo/05-camera-snap.png" width="240"><br>
<b>カメラ撮影</b><br>
<sub>「写真を撮って」— OV3660 が撮影・アップロードし、Telegram チャットに画像を送信。</sub>
</td>
<td align="center">
<img src="docs/demo/06-camera-vision.png" width="240"><br>
<b>AIビジョン分析</b><br>
<sub>Gemini Vision が写真を検査 — 物体検出、テキスト読取、異常検知、実用的なアドバイス。</sub>
</td>
<td align="center">
<img src="docs/demo/07-pc-control.png" width="240"><br>
<b>PCリモート制御</b><br>
<sub>スクリーンショット、シェルコマンド、ファイル操作、アプリ起動 — USB Serial 経由で Telegram から。</sub>
</td>
</tr>
<tr>
<td align="center">
<img src="docs/demo/04-skills-list.png" width="240"><br>
<b>16のAIスキル</b><br>
<sub>コード実行、画像生成、Excel処理、メール、電卓、翻訳など。</sub>
</td>
<td align="center">
<img src="docs/demo/08-auto-tasks.png" width="240"><br>
<b>AutoTask</b><br>
<sub>自然言語で繰り返しAIタスクを作成 —「10分ごとに写真を撮って異常をチェック」。</sub>
</td>
<td align="center">
<img src="docs/demo/09-scheduling.png" width="240"><br>
<b>スマートスケジューリング</b><br>
<sub>毎日リマインダー、インターバルタスク、トリガーベーススキル — すべて自然言語で作成。</sub>
</td>
</tr>
</table>

---

## ⭐ 主な機能

| | 機能 | 説明 |
|---|---|---|
| 🧠 | **Gemini AI チャット** | コンテキストメモリ付き対話AI、あらゆる言語対応 |
| 📷 | **カメラビジョン** | OV3660 撮影 + Gemini Vision — 物体検出、OCR、異常レポート |
| 🎙️ | **音声 I/O** | 音声認識、TTS再生、ウェイクワード検出 (ESP-SR) |
| 🖥️ | **PCリモート制御** | USB Serial → スクリーンショット、シェル、ファイル操作、アプリ起動 |
| 📅 | **スマートスケジューリング** | 自然言語タスク作成、毎日リマインダー、繰り返しAutoTask |
| 🧰 | **16のAIスキル** | コード実行、画像生成、Excel、メール、電卓、翻訳 |
| 🖼️ | **LCDディスプレイ** | ST7789 画面 — 絵文字、ステータス、AI応答表示 |
| 🔄 | **OTAアップデート** | Telegram で .bin ファイルを送信してファームウェア更新 |
| 📡 | **WiFi自動復旧** | デュアルネットワーク対応、自動再接続、120秒ヘルスチェック |
| 💬 | **自然言語プログラミング** | 文章入力でオートメーション作成 — コード不要 |
| 🔒 | **セキュリティ制御** | PCセーフモード、OTA HTTPS限定、ストリームトークン認証、チャットログマスク |

---

## 🏗️ ユースケース

| シナリオ | 方法 | 例 |
|----------|------|-------|
| 🏭 **工場** | スケジュール + GPIO + AIビジョン | 不良品検出、機器監視 |
| 🏗️ **建設現場** | 定期撮影 + AI分析 | ヘルメット検出、エリアアラート |
| 🏠 **スマートホーム** | スタンドアロン知能ユニット | 来客認識、高齢者見守り |
| 💻 **リモートデスクトップ** | USB接続PC制御 | スクリーンショット、スクリプト、Telegram経由メール |
| 🏢 **オフィス** | 自然言語タスク管理 | 請求書スキャン、自動スケジューリング |

---

<a id="-ハードウェア"></a>

## 📦 ハードウェア

### コア（必須）— 約¥1,500

| コンポーネント | スペック | 価格 |
|---------------|---------|------|
| **ESP32-S3-N16R8** | 16MB Flash / 8MB PSRAM | ~¥1,200 |

> **最小構成：ESP32-S3 ボード1枚 + USBケーブル = 完全な Telegram AI アシスタント。**

### オプションモジュール

| モジュール | 価格 | 有効機能 |
|-----------|------|---------|
| 📷 OV3660 カメラ | ~¥450 | カメラ撮影、AIビジョン |
| 🖥️ ST7789 LCD | ~¥300 | 絵文字、ステータス表示 |
| 🔊 MAX98357 スピーカー | ~¥150 | TTS音声出力 |
| 🎤 INMP441 マイク | ~¥150 | 音声入力、ウェイクワード |

**フル構成：~¥2,250** — カメラ、画面、音声、スピーカー搭載の完全AIアシスタント。

---

<a id="-クイックスタート--5分"></a>

## ⚡ クイックスタート — 5分

### 前提条件

| キー | 取得方法 |
|------|---------|
| 🤖 **Telegram Bot Token** | [@BotFather](https://t.me/BotFather) → `/newbot` |
| 🧠 **Gemini API Key** | [Google AI Studio](https://aistudio.google.com/apikey) |
| 📶 **WiFi SSID + パスワード** | 自宅/オフィスの WiFi |

### ソースからビルド

```bash
git clone https://github.com/ry6612-pixel/Novaclaw.git
cd Novaclaw

# 1. シークレット設定ファイルを作成（リポジトリ外に保存）
.\setup-secure-config.ps1
notepad $HOME/.novaclaw/secrets/user_config.txt

# 2. ビルド（バイナリにシークレットは含まれません）
.\build.ps1

# 3. フラッシュ
.\flash.ps1 -Port COM3

# 4. USB Serial経由でデバイスNVSにシークレットを書き込み
.\provision.ps1 -Port COM3
```

> **セキュリティ設計：** シークレットはファームウェアバイナリに**一切**コンパイルされません。
> 初回起動時にUSB Serial経由でデバイスに送信され、チップ内蔵NVSフラッシュに保存されます。
> `.bin` ファイルは安全に配布できます。

<details>
<summary><b>ビルド前提条件</b></summary>

```bash
rustup install nightly
cargo install espup espflash
espup install
. ~/export-esp.sh   # Linux/Mac
# Windows: build.ps1 が環境を自動設定
```

</details>

<details>
<summary><b>シークレット設定の優先順位</b></summary>

`provision.ps1` は以下の順序でシークレットを検索:
1. `-Config` パラメータ
2. `NOVACLAW_CONFIG` 環境変数
3. `NOVACLAW_CONFIG_DIR/user_config.txt`
4. `$HOME/.novaclaw/secrets/user_config.txt`
5. リポジトリローカル `user_config.txt`（レガシーフォールバック）

</details>

---

## 🎮 コマンドリファレンス

### コア

| コマンド | 説明 |
|---------|------|
| *（任意のテキスト）* | AI会話（言語自動検出） |
| `/help` | 全コマンド一覧 |
| `/status` | システム情報 |
| `/model set flash\|pro` | Gemini モデル切替 |
| `/tokens` | トークン使用統計 |
| `/diag` | 自己診断 |

### カメラ＆ビジョン

| コマンド | 説明 |
|---------|------|
| `/camera snap` | 撮影 → Telegram 送信 |
| `/camera vision` | 撮影 → Gemini AI 分析 |
| `/camera vision <プロンプト>` | 撮影 → カスタムAI質問 |
| `/camera stream` | MJPEG ストリーム開始 |

### スケジュール

| コマンド | 説明 |
|---------|------|
| `/remind 5m メッセージ` | 繰り返しリマインダー |
| `/remind 8:00 メッセージ` | 毎日8:00 |
| `/tasks` | AutoTask一覧 |
| `/stop` | 全タスク緊急停止 |
| `/pause` / `/resume` | AI処理の一時停止/再開 |

### メモリ＆スキル

| コマンド | 説明 |
|---------|------|
| `/memories` | 保存されたメモリ表示 |
| `/remember key value` | メモリ保存 |
| `/skills` | AIスキル一覧 |
| `/skill add name\|desc\|trigger` | カスタムスキル作成 |

### PC制御 (USB接続)

| コマンド | 説明 |
|---------|------|
| `/pc shell <cmd>` | シェルコマンド実行 |
| `/screenshot` | PCデスクトップキャプチャ |
| `/pc type <text>` | キーボード入力 |
| `/pc hotkey ctrl+c` | ショートカット送信 |
| `/pc open <file/url>` | ファイルまたはURL開く |

### 音声＆オーディオ

| コマンド | 説明 |
|---------|------|
| `/voice off\|normal\|brief` | 音声応答モード |
| `/audio wake on\|off` | ウェイクワード検出 |
| `/audio transcribe` | 録音 → 音声認識 |

### WiFi＆メンテナンス

| コマンド | 説明 |
|---------|------|
| `/wifi set SSID password` | WiFi設定 |
| `/ota <https://url>` | OTAファームウェア更新 |
| `.bin` ファイル送信 | Telegram経由OTA |
| `/reset` | 工場出荷状態にリセット |

---

## 🏛️ アーキテクチャ

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

## 配線図

<details>
<summary><b>📷 OV3660 カメラ (DVP)</b></summary>

| ピン | GPIO | | ピン | GPIO |
|------|------|-|------|------|
| XCLK | 15 | | D4 | 12 |
| SDA | 4 | | D5 | 18 |
| SCL | 5 | | D6 | 17 |
| D0 | 11 | | D7 | 16 |
| D1 | 9 | | VSYNC | 6 |
| D2 | 8 | | HREF | 7 |
| D3 | 10 | | PCLK | 13 |

> カスタムピン？Telegram で `/camera pins d0=11 d1=9 ...` — NVS に保存されます。

</details>

<details>
<summary><b>🖥️ ST7789 LCD (SPI)</b></summary>

RST=21, DC=47, BL=38, SCLK=19, MOSI=20, CS=45

> `/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45 w=240 h=320`

</details>

<details>
<summary><b>🔊 スピーカー + 🎤 マイク (I2S)</b></summary>

**MAX98357:** BCLK=46, LRC=14, DIN=48
**INMP441:** SCK=2, WS=1, SD=41

</details>

---

## 🔑 セキュリティ

**保護済み：**
- **バイナリにシークレットなし** — 認証情報はUSB Serial経由でNVSに書き込み、ファームウェアにはコンパイルされない
- ソースコードにシークレットなし — 認証情報は外部設定ファイル (`~/.novaclaw/secrets/`)
- Chat ID 検証 — 許可されたユーザーのみ応答
- PCセーフモード（デフォルトON）— AIコマンドをブロック；`/pc_unlock` / `/pc_lock` で切替
- OTA は HTTPS のみ
- MJPEG ストリームにセッショントークン必須
- チャット内容を Serial にログ出力しない
- レート制限 — 最大5回/分 AI呼び出し

**既知のリスク：**
- 高権限リモート制御 — PCコマンド、OTA、AI自動化がコンピュータを変更可能
- 音声・写真・チャットが Google Gemini API に送信される
- Telegram 経由OTAはファームウェア署名を検証しない
- MJPEG ストリームはローカルネットワークで HTTP を使用

> ⚠️ **個人実験用に設計されています。** 展開前にソースコードをレビューしてください。

---

## 🚀 リリースチェックリスト

変更を公開する前に：

```powershell
.\pre-publish-scan.ps1      # ALL CHECKS PASSED が必須
.\build.ps1                  # コンパイル成功が必須
.\flash.ps1                  # デバイスにフラッシュ
.\provision.ps1              # シークレットをNVSに書き込み
git status --short           # クリーンが必須
git push
```

---

## FAQ

<details><summary><b>Gemini API は有料ですか？</b></summary>
無料枠で1日数百リクエスト処理可能。内蔵レート制限（5回/分）で過度な使用を防止。
</details>

<details><summary><b>カメラ/LCD/スピーカーなしでも動きますか？</b></summary>
はい。最小要件は ESP32-S3 のみ。すべての周辺機器はオプションで、自動検出されます。
</details>

<details><summary><b>日本語/英語/中国語/... 対応？</b></summary>
対応。入力した言語で応答します。
</details>

<details><summary><b>xiaozhi-esp32 との違いは？</b></summary>
xiaozhi-esp32 は音声チャット特化。Novaclaw は自律エッジAI特化 — カメラビジョン、スケジューリング、PC制御、自然言語プログラミング。補完的なプロジェクトです。
</details>

---

## 🤝 コントリビューション

PRお待ちしています！Fork → ブランチ作成 → コミット → PR。

---

## 📄 ライセンス

デュアルライセンス [MIT](LICENSE-MIT) / [Apache-2.0](LICENSE-APACHE)

## 🙏 謝辞

- **[Espressif](https://www.espressif.com/)** — ESP32-S3 チップと ESP-IDF フレームワーク
- **[Google Gemini](https://ai.google.dev/)** — マルチモーダル AI API
- **[xiaozhi-esp32](https://github.com/78/xiaozhi-esp32)** — 先駆的な ESP32 AI プロジェクト
