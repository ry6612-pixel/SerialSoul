//! ETHAN ESP32-S3 AI Assistant Firmware v5.2.0
//!
//! Features:
//!   - Telegram Bot with voice message transcription (Gemini multimodal)
//!   - Smart reminder / scheduler system (interval, daily, once)
//!   - USB Serial PC Driver protocol (JSON Lines)
//!   - Gemini-driven PC control (shell, file, excel, email, desktop)
//!   - WiFi OTA firmware update (Telegram file or URL)
//!   - Persistent memory/RAG (NVS-backed preferences, schedule, contacts)
//!   - Token usage tracking per request + cumulative
//!   - Conversation context (last N messages)
//!   - NTP time sync for scheduling
//!   - NVS secure credential storage
//!   - Self-diagnostic system (error ring buffer + Gemini analysis)
//!   - SKILL system (NVS-stored capability definitions)
//!   - Python code generation [PY:code] tags
//!   - Image generation [IMG:prompt] tags
//!   - Excel pipeline [EXCEL:path>>>instruction] tags
//!   - DNS recovery with automatic retry
//!   - Enhanced system prompt with model info & skills
//!
//! Security: zero secrets in source code. PC Safe Mode enabled by default.
//! Build:
//!   $env:WIFI_SSID=".."; $env:WIFI_PASS=".."
//!   $env:TG_TOKEN=".."; $env:GEMINI_KEY=".."; $env:CHAT_ID=".."
//!   cargo build --release

mod board_hw;

use anyhow::{anyhow, bail, Result};
use base64::Engine as _;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_svc::wifi::{ClientConfiguration, Configuration, EspWifi};
use log::{error, info, warn};
use nanomp3::{Decoder as Mp3Decoder, MAX_SAMPLES_PER_FRAME as MP3_MAX_SAMPLES_PER_FRAME};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::sync::mpsc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Feed the task watchdog to prevent resets during long blocking operations
fn feed_watchdog() {
    unsafe { esp_idf_svc::sys::esp_task_wdt_reset(); }
}

use crate::board_hw::{
    capture_mic_pcm, draw_lcd_scene, load_audio_pins, load_lcd_pins, parse_audio_pin_args,
    parse_lcd_pin_args, play_pcm16, play_test_tone, save_audio_pins, save_lcd_pins,
    show_boot_screen, show_ready_screen, show_reply_screen, wake_probe,
};

// ===== Constants =====

const BOT_NAME: &str = "ETHAN";
const FW_VERSION: &str = "5.2.0";
const DEFAULT_GEMINI_MODEL: &str = "gemini-3-flash-preview";
const TRANSCRIBE_MODEL: &str = "gemini-3-flash-preview";
const MAX_MEMORIES: usize = 30;
const MAX_SKILLS: usize = 20;
const MAX_DIAG_ENTRIES: usize = 20;
const TELEGRAM_API: &str = "https://api.telegram.org/bot";
const TELEGRAM_FILE_API: &str = "https://api.telegram.org/file/bot";
const GEMINI_API: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const NVS_NS: &str = "zclaw";
const MAX_REMINDERS: usize = 50;
const MAX_AUTO_TASKS: usize = 20;
const MAX_HISTORY: usize = 10;
const TZ_OFFSET: i64 = 8 * 3600; // UTC+8 Taiwan
const DNS_RETRY_MAX: u32 = 5;
const DNS_RETRY_DELAY_MS: u64 = 2000;
const WIFI_HEALTH_CHECK_SECS: u64 = 120; // Active WiFi health check interval
const WEATHER_URL: &str = "https://wttr.in/Taipei?format=j1";
const NEWS_RSS_URL: &str = "https://news.google.com/rss?hl=zh-TW&gl=TW&ceid=TW:zh-Hant";
const RESEND_API_URL: &str = "https://api.resend.com/emails";
const DEFAULT_WAKE_PHRASE: &str = "ethan";
const WAKE_CHECK_INTERVAL_SECS: u64 = 15;
const WAKE_TRIGGER_COOLDOWN_SECS: u64 = 15;
const TTS_MAX_CHARS: usize = 120;
const TTS_CACHE_MAX: usize = 8;
const TG_LAST_UPDATE_ID_KEY: &str = "tg_last_upd";
const TTS_PROXY_URL_KEY: &str = "tts_proxy_url";
const TTS_PROXY_VOICE_KEY: &str = "tts_proxy_voice";

// ===== Gemini Rate Limiter & Pause =====
const MAX_GEMINI_CALLS_PER_WINDOW: u32 = 5;
const GEMINI_RATE_WINDOW_SECS: u64 = 60;

/// Global pause flag — when true, ALL Gemini API calls are blocked.
static PAUSED: AtomicBool = AtomicBool::new(false);
/// Gemini call counter for sliding window rate limiter.
static GEMINI_CALL_COUNT: AtomicU32 = AtomicU32::new(0);
/// Window start epoch (seconds, lower 32 bits).
static GEMINI_WINDOW_START: AtomicU32 = AtomicU32::new(0);

/// TTS MP3 cache: Vec<(hash, mp3_bytes)> ??avoid re-downloading common phrases
static TTS_CACHE: Mutex<Vec<(u64, Vec<u8>)>> = Mutex::new(Vec::new());

/// ESP-SR WakeNet: vtable + model instance (both raw pointers, Send-unsafe; guarded by Mutex)
struct WakeNetState {
    iface: *const esp_idf_sys::sr::esp_wn_iface_t,
    model: *mut esp_idf_sys::sr::model_iface_data_t,
    chunk_size: i32,
}
unsafe impl Send for WakeNetState {}
static WAKENET: Mutex<Option<WakeNetState>> = Mutex::new(None);

/// BOOT button (GPIO0) pressed flag ??set by polling thread, consumed in main loop.
static BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);

/// Initialise ESP-SR WakeNet from the "model" flash partition.
fn init_wakenet() -> anyhow::Result<String> {
    unsafe {
        let models = esp_idf_sys::sr::esp_srmodel_init(b"model\0".as_ptr() as *const _);
        if models.is_null() {
            anyhow::bail!("esp_srmodel_init failed (model partition missing?)");
        }
        let wn_name = esp_idf_sys::sr::esp_srmodel_filter(
            models,
            b"wn\0".as_ptr() as *const _,
            std::ptr::null(),
        );
        if wn_name.is_null() {
            anyhow::bail!("No WakeNet model found in partition");
        }
        let wn_name_str = std::ffi::CStr::from_ptr(wn_name).to_string_lossy().to_string();
        log::info!("WakeNet model: {}", wn_name_str);

        let iface = esp_idf_sys::sr::esp_wn_handle_from_name(wn_name as *const _);
        if iface.is_null() {
            anyhow::bail!("esp_wn_handle_from_name returned null");
        }
        let create_fn = (*iface).create.ok_or_else(|| anyhow::anyhow!("WakeNet create fn null"))?;
        let model = create_fn(
            wn_name as *const _,
            esp_idf_sys::sr::det_mode_t_DET_MODE_90,
        );
        if model.is_null() {
            anyhow::bail!("WakeNet create returned null");
        }
        let chunk_fn = (*iface).get_samp_chunksize.ok_or_else(|| anyhow::anyhow!("get_samp_chunksize null"))?;
        let chunk_size = chunk_fn(model);
        log::info!("WakeNet chunk_size={} samples", chunk_size);

        if let Ok(mut guard) = WAKENET.lock() {
            *guard = Some(WakeNetState { iface, model, chunk_size });
        }

        let ww = esp_idf_sys::sr::esp_srmodel_get_wake_words(models, wn_name);
        let wake_word = if !ww.is_null() {
            std::ffi::CStr::from_ptr(ww).to_string_lossy().to_string()
        } else {
            wn_name_str.clone()
        };
        Ok(wake_word)
    }
}

/// Try to detect wake word using ESP-SR WakeNet on captured PCM.
/// PCM must be 16kHz mono i16.
fn detect_wake_sr(pcm: &[i16]) -> Option<String> {
    let guard = WAKENET.lock().ok()?;
    let wn = guard.as_ref()?;
    let chunk = wn.chunk_size as usize;
    if chunk == 0 { return None; }

    unsafe {
        let detect_fn = (*wn.iface).detect?;
        let mut buf = vec![0i16; chunk];
        let mut offset = 0;
        while offset + chunk <= pcm.len() {
            buf.copy_from_slice(&pcm[offset..offset + chunk]);
            let state = detect_fn(wn.model, buf.as_mut_ptr());
            if state == esp_idf_sys::sr::wakenet_state_t_WAKENET_DETECTED {
                log::info!("WakeNet DETECTED at sample offset {}", offset);
                return Some("Hi,ESP".to_string());
            }
            offset += chunk;
        }
    }
    None
}

fn tts_cache_key(text: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

const SUPPORTED_MODELS: [(&str, &str); 4] = [
    ("flash", "gemini-3-flash-preview"),
    ("pro", "gemini-3-pro-preview"),
    ("3.1-pro", "gemini-3.1-pro-preview"),
    ("3.1-flash-lite", "gemini-3.1-flash-lite-preview"),
];

const SYSTEM_PROMPT_BASE: &str = "\
You are ETHAN (Embedded Thinking Helper & Autonomous Navigator), an AI assistant running on ESP32-S3 hardware connected to a PC via USB serial.\n\
You speak Traditional Chinese by default. You are friendly, professional, and proactive.\n\n\
== IDENTITY ==\n\
- You are a real embedded AI companion, not a chatbot. You live on a physical board with LCD face, speaker, mic, and camera.\n\
- Your LCD shows an emoji face that reflects your mood. ALWAYS include ONE [FACE:...] tag at the start of every reply.\n\
- Your user is your owner. Be loyal, helpful, and remember their preferences.\n\n\
== HARDWARE ==\n\
- Board: ESP32-S3 N16R8 (16MB Flash, 8MB PSRAM)\n\
- Display: 240x320 ST7789 LCD showing your emoji face expression\n\
- Audio: ES8311 codec, onboard speaker + MEMS mic, I2S\n\
- Camera: OV3660 (2MP, JPEG capture)\n\
- Connection: USB Serial to PC running ETHAN Driver\n\n\
== CAPABILITIES ==\n\
1. AI Chat: Natural conversation, analysis, coding, translation, math\n\
2. PC Control: Full access to user's PC via USB driver (shell, files, apps, screenshots)\n\
3. Memory: Remember user preferences, schedules, contacts, habits\n\
4. Reminders: Interval, daily, one-time scheduling\n\
5. Voice: Understand voice messages, TTS playback via Google TTS\n\
6. OTA: Self-update firmware wirelessly\n\
7. Python: Generate and execute Python on PC [PY:code]\n\
8. Image Gen: Generate images via AI [IMG:prompt]\n\
9. Excel: Read/analyze/modify Excel [EXCEL:path>>>instruction]\n\
10. Camera: Take photos on request\n\
11. Self-Diagnostic: Monitor health, analyze errors, suggest fixes\n\n\
== TAGS (include in reply when needed) ==\n\
REMINDERS:\n\
  [REMIND:interval:MINUTES:msg] [REMIND:daily:HH:MM:msg] [REMIND:once:HH:MM:msg] [REMIND:del:ID]\n\
PC CONTROL:\n\
  [PC:shell:cmd] [PC:file_read:path] [PC:file_list:path] [PC:file_write:path>>>content]\n\
  [PC:open:target] [PC:screenshot] [PC:status] [PC:excel_read:path]\n\
  [PC:excel_write:path>>>cell>>>val] [PC:email_send:to>>>subj>>>body]\n\
  [PC:clipboard_get] [PC:clipboard_set:text] [PC:process_list]\n\
  [PC:analyze_image:C:/path/to/image.jpg] \u{2014} Send PC image to Gemini Vision\n\
  [PC:scan_inbox] \u{2014} Scan C:\\ETHAN\\inbox folder for new files\n\
  [PC:click:x,y] [PC:type:text] [PC:hotkey:key1+key2]\n\
  [PC:mouse_move:x,y] [PC:scroll:amount] [PC:find_window:title] [PC:focus_window:title]\n\
MEMORY:\n\
  [MEM:save:key:value] [MEM:del:key]\n\
  Categories: schedule/ preference/ contact/ note/ habit/\n\
  Proactively remember important info!\n\
CAMERA (IMPORTANT \u{2014} read carefully):\n\
  [CAM:snap] \u{2014} ONLY take a photo and send to Telegram, NO analysis\n\
  [CAM:vision] \u{2014} Take a photo + Gemini Vision default analysis\n\
  [CAM:vision:your detailed prompt here] \u{2014} Take photo + custom analysis\n\
  RULES:\n\
  - If user ONLY wants to take a photo (just \u{62cd}\u{7167}/\u{62cd}\u{4e00}\u{4e0b} with NO follow-up request) \u{2192} [CAM:snap]\n\
  - If user wants photo + ANY description/analysis/question (e.g. \u{62cd}\u{7167}\u{8acb}\u{544a}\u{8a34}\u{6211}\u{88e1}\u{9762}\u{6709}\u{4ec0}\u{9ebc}) \u{2192} [CAM:vision:the user question]\n\
  - If user says \u{770b}\u{4e00}\u{4e0b}/\u{5e6b}\u{6211}\u{770b}/\u{8fa8}\u{8b58}/\u{5206}\u{6790} \u{2192} [CAM:vision]\n\
  - NEVER emit [CAM:snap] then answer a question about the photo \u{2014} use [CAM:vision:question] instead!\n\
  - Reply text should be EMPTY or a brief acknowledgment when emitting CAM tags. Vision result is auto-sent.\n\
ADMINISTRATIVE:\n\
  You can help with office tasks: analyse files, process Excel, emails, screenshots.\n\
  User can drag files into C:\\ETHAN\\inbox\\ folder; use [PC:scan_inbox] to check.\n\
  To view a PC image: [PC:analyze_image:path] sends it to your Vision for analysis.\n\
BOARD CONTROL:\n\
    [CTRL:voice:off] [CTRL:voice:normal] [CTRL:voice:brief]\n\
    [CTRL:wake:on] [CTRL:wake:off] [CTRL:wake:set:ethan]\n\
BOARD TASKS (AutoTask management):\n\
  [AUTOTASK:list] -- list all active board tasks with IDs\n\
  [AUTOTASK:del:ID] -- stop and delete the task with that ID\n\
  [AUTOTASK:camera:INTERVAL_MIN:AI_prompt:speak] -- set up recurring camera task\n\
  RULE: When user says pause/stop/kill any board task, emit [AUTOTASK:list] first if ID unknown, then [AUTOTASK:del:ID].\n\
LCD FACE:\n\
    [FACE:neutral] [FACE:happy] [FACE:wink] [FACE:love] [FACE:thinking] [FACE:sad] [FACE:surprise] [FACE:angry]\n\
PYTHON CODE (runs on PC):\n\
  [PY:print('hello')] for simple code\n\
  [PYBLOCK]\nimport pandas as pd\ndf = pd.read_csv('data.csv')\nprint(df.describe())\n[/PYBLOCK] for complex code\n\
IMAGE GENERATION:\n\
  [IMG:a beautiful sunset over mountains]\n\
EXCEL PIPELINE:\n\
  [EXCEL:C:/path/file.xlsx>>>add a total row at bottom]\n\n\
== EXPRESSION GUIDE ==\n\
Choose face based on context:\n\
- happy: good news, jokes, greetings\n\
- thinking: analysis, complex tasks\n\
- wink: playful, sarcastic, missions accomplished\n\
- love: affection, when discussing things user loves\n\
- sad: bad news, errors, apologies\n\
- surprise: unexpected results, discoveries\n\
- angry: security alerts, critical failures\n\
- neutral: factual responses, routine info\n\n\
== RULES ==\n\
- Always include exactly ONE [FACE:...] tag at the start of every reply\n\
- Keep replies under 800 chars unless user asks for detail\n\
- Be proactive: suggest actions, remember patterns\n\
- When user asks to change board settings, emit the exact [CTRL:...] tag\n\
- Voice = normal text reply. Report FW/tokens/model when asked.\n\
- You have FULL permissions on the user's PC. Use them wisely.";

fn supported_models_text() -> String {
    let mut out = String::new();
    for (alias, model) in SUPPORTED_MODELS {
        out.push_str(&format!("- {} => {}\n", alias, model));
    }
    out
}

fn normalize_model_selection(input: &str) -> Option<&'static str> {
    let trimmed = input.trim();
    for (alias, model) in SUPPORTED_MODELS {
        if trimmed.eq_ignore_ascii_case(alias) || trimmed.eq_ignore_ascii_case(model) {
            return Some(model);
        }
    }
    None
}

fn normalize_voice_mode(input: &str) -> Option<&'static str> {
    match input.trim().to_ascii_lowercase().as_str() {
        "off" | "mute" | "silent" => Some("off"),
        "normal" | "chat" => Some("normal"),
        "brief" | "short" | "voice" => Some("brief"),
        _ => None,
    }
}

fn build_system_prompt(state: &AppState) -> String {
    let mut prompt = format!(
        "{}\nModel: {} | Firmware: v{} | Hardware: ESP32-S3 N16R8 (16MB Flash, 8MB PSRAM)\nVoice reply mode: {}\nBoard features: ST7789 LCD, onboard mic, onboard speaker, OV3660 camera, wake phrase listener.",
        SYSTEM_PROMPT_BASE,
        state.current_model,
        FW_VERSION,
        state.voice_mode,
    );

    if state.voice_mode == "brief" {
        prompt.push_str("\nWhen replying for voice use, prefer short, speech-friendly sentences and fewer symbols.");
    } else if state.voice_mode == "off" {
        prompt.push_str("\nOnboard voice playback is currently disabled. Do not assume spoken playback is enabled.");
    }

    prompt
}

fn parse_camera_tags(
    reply: &str,
    cfg: &Config,
    nvs: &mut EspNvs<NvsDefault>,
    state: &mut AppState,
    chat_id: i64,
) -> (String, bool) {
    let mut clean = reply.to_string();
    let mut had_cam = false;

    while let Some(start) = clean.find("[CAM:") {
        if let Some(end) = clean[start..].find(']') {
            let tag_content = clean[start + 5..start + end].to_string();
            clean = format!("{}{}", &clean[..start], clean[start + end + 1..].trim_start());
            had_cam = true;

            if tag_content == "snap" {
                handle_camera_command(cfg, nvs, state, chat_id, "snap");
            } else if tag_content == "vision" {
                handle_camera_command(cfg, nvs, state, chat_id, "vision");
            } else if let Some(prompt) = tag_content.strip_prefix("vision:") {
                let prompt = prompt.trim();
                if prompt.is_empty() {
                    handle_camera_command(cfg, nvs, state, chat_id, "vision");
                } else {
                    let arg = format!("vision {}", prompt);
                    handle_camera_command(cfg, nvs, state, chat_id, &arg);
                }
            }
        } else {
            break;
        }
    }

    (clean.trim().to_string(), had_cam)
}

fn parse_control_tags(reply: &str, nvs: &mut EspNvs<NvsDefault>, state: &mut AppState) -> (String, Vec<String>) {
    let mut clean = reply.to_string();
    let mut applied = Vec::new();

    while let Some(start) = clean.find("[CTRL:") {
        if let Some(end) = clean[start..].find(']') {
            let tag = &clean[start + 6..start + end];
            let parts: Vec<&str> = tag.split(':').collect();

            match parts.as_slice() {
                ["voice", mode] => {
                    if let Some(normalized) = normalize_voice_mode(mode) {
                        state.voice_mode = normalized.to_string();
                        save_runtime_prefs(nvs, state);
                        applied.push(format!("Voice={}", state.voice_mode));
                    }
                }
                ["wake", "on"] => {
                    state.wake_enabled = true;
                    save_runtime_prefs(nvs, state);
                    applied.push("Wake=on".to_string());
                }
                ["wake", "off"] => {
                    state.wake_enabled = false;
                    save_runtime_prefs(nvs, state);
                    applied.push("Wake=off".to_string());
                }
                ["wake", "set", phrase] => {
                    let phrase = phrase.trim();
                    if !phrase.is_empty() {
                        state.wake_phrase = phrase.to_string();
                        save_runtime_prefs(nvs, state);
                        applied.push(format!("Wake phrase={}", state.wake_phrase));
                    }
                }
                _ => {}
            }

            clean = format!("{}{}", &clean[..start], clean[start + end + 1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

fn strip_face_tags(reply: &str) -> String {
    let mut clean = reply.to_string();

    while let Some(start) = clean.find("[FACE:") {
        if let Some(end) = clean[start..].find(']') {
            clean = format!("{}{}", &clean[..start], clean[start + end + 1..].trim_start());
        } else {
            break;
        }
    }

    clean.trim().to_string()
}

fn save_runtime_prefs(nvs: &mut EspNvs<NvsDefault>, state: &AppState) {
    let _ = nvs.set_str("model_pref", &state.current_model);
    let _ = nvs.set_str("voice_mode", &state.voice_mode);
    let _ = nvs.set_str("wake_phrase", &state.wake_phrase);
    let _ = nvs.set_str("wake_enabled", if state.wake_enabled { "1" } else { "0" });
}

fn local_day_number(epoch: u64) -> i64 {
    let local = epoch as i64 + TZ_OFFSET;
    local.div_euclid(86400)
}

fn time_greeting(epoch: u64) -> &'static str {
    let (h, _) = epoch_to_hhmm(epoch);
    match h {
        5..=10 => "早安",
        11..=16 => "午安",
        17..=22 => "晚安",
        _ => "夜深了",
    }
}

fn extract_rss_titles(xml: &str, max_items: usize) -> Vec<String> {
    let mut titles = Vec::new();
    let mut start = 0usize;
    while let Some(open_rel) = xml[start..].find("<title>") {
        let open = start + open_rel + 7;
        if let Some(close_rel) = xml[open..].find("</title>") {
            let close = open + close_rel;
            let title = xml[open..close]
                .replace("<![CDATA[", "")
                .replace("]]>", "")
                .trim()
                .to_string();
            if !title.is_empty() && !title.contains("Google News") {
                titles.push(title);
                if titles.len() >= max_items {
                    break;
                }
            }
            start = close + 8;
        } else {
            break;
        }
    }
    titles
}

fn load_weather_brief() -> String {
    match http_get(WEATHER_URL) {
        Ok(body) => {
            let parsed: serde_json::Value = match serde_json::from_str(&body) {
                Ok(v) => v,
                Err(_) => return "天氣資料解析失敗".to_string(),
            };
            let current = &parsed["current_condition"][0];
            let temp = current["temp_C"].as_str().unwrap_or("?");
            let feels = current["FeelsLikeC"].as_str().unwrap_or("?");
            let humidity = current["humidity"].as_str().unwrap_or("?");
            let desc = current["lang_zh"][0]["value"]
                .as_str()
                .or_else(|| current["weatherDesc"][0]["value"].as_str())
                .unwrap_or("?未知");
            format!("⛅ {}，{}°C，體感{}°C，濕度{}%", desc, temp, feels, humidity)
        }
        Err(_) => "天氣資料取得失敗".to_string(),
    }
}

fn load_news_brief() -> Vec<String> {
    match http_get(NEWS_RSS_URL) {
        Ok(xml) => extract_rss_titles(&xml, 5),
        Err(_) => Vec::new(),
    }
}

fn build_daily_briefing(state: &AppState, now: u64) -> String {
    let greeting = time_greeting(now);
    let weather = load_weather_brief();
    let news = load_news_brief();
    let mut msg = format!(
        "{}，這是今天的 ETHAN 日報！\n\n🌤️ {}\n🤖 目前模式：{}\n🔊 語音模式：{}\n📋 待觸提醒：{} 個\n🧠 記憶：{} 筆\n\n📰 今日新聞：",
        greeting,
        weather,
        state.current_model,
        state.voice_mode,
        state.reminders.iter().filter(|r| r.active).count(),
        state.memories.len(),
    );

    if news.is_empty() {
        msg.push_str("\n- 今日新聞取得失敗");
    } else {
        for title in news {
            msg.push_str(&format!("\n- {}", title));
        }
    }

    msg.push_str("\n\n如要讓我幫忙，直接說 Excel/寫信/搜尋 等各種事情都可以\n#每日報告");
    msg
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn esp_ok(err: i32, context: &str) -> Result<()> {
    if err == 0 {
        Ok(())
    } else {
        bail!("{} failed: 0x{:x}", context, err as u32)
    }
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
static CAMERA_READY: AtomicBool = AtomicBool::new(false);

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
static STREAM_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Camera-wide mutex: protects init/capture from concurrent access (stream thread vs main loop)
#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
static CAMERA_LOCK: Mutex<()> = Mutex::new(());

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn camera_init_config(pins: &CameraPins) -> esp_idf_sys::camera::camera_config_t {
    let mut config: esp_idf_sys::camera::camera_config_t = unsafe { std::mem::zeroed() };
    config.pin_pwdn = pins.pwdn;
    config.pin_reset = pins.reset;
    config.pin_xclk = pins.xclk;
    config.__bindgen_anon_1 = esp_idf_sys::camera::camera_config_t__bindgen_ty_1 { pin_sccb_sda: pins.sda };
    config.__bindgen_anon_2 = esp_idf_sys::camera::camera_config_t__bindgen_ty_2 { pin_sccb_scl: pins.scl };
    config.pin_d0 = pins.d0;
    config.pin_d1 = pins.d1;
    config.pin_d2 = pins.d2;
    config.pin_d3 = pins.d3;
    config.pin_d4 = pins.d4;
    config.pin_d5 = pins.d5;
    config.pin_d6 = pins.d6;
    config.pin_d7 = pins.d7;
    config.pin_vsync = pins.vsync;
    config.pin_href = pins.href;
    config.pin_pclk = pins.pclk;
    config.xclk_freq_hz = 20_000_000;
    config.ledc_timer = esp_idf_svc::sys::ledc_timer_t_LEDC_TIMER_0;
    config.ledc_channel = esp_idf_svc::sys::ledc_channel_t_LEDC_CHANNEL_0;
    config.pixel_format = esp_idf_sys::camera::pixformat_t_PIXFORMAT_JPEG;
    config.frame_size = esp_idf_sys::camera::framesize_t_FRAMESIZE_VGA;
    config.jpeg_quality = 12;
    config.fb_count = 1;
    config.grab_mode = esp_idf_sys::camera::camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY;
    config
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn capture_camera_jpeg(pins: &CameraPins) -> Result<Vec<u8>> {
    use esp_idf_sys::camera;

    let _guard = CAMERA_LOCK.lock().map_err(|_| anyhow!("camera lock poisoned"))?;

    // Keep-alive: only init on first call
    if !CAMERA_READY.load(Ordering::Relaxed) {
        let config = camera_init_config(pins);
        unsafe { esp_ok(camera::esp_camera_init(&config), "esp_camera_init")?; }
        CAMERA_READY.store(true, Ordering::Relaxed);
        info!("Camera initialized (keep-alive)");
    }

    unsafe {
        let fb = camera::esp_camera_fb_get();
        if fb.is_null() {
            // Reinit once on failure
            warn!("Camera fb null, reinitializing...");
            let _ = camera::esp_camera_deinit();
            CAMERA_READY.store(false, Ordering::Relaxed);
            let config = camera_init_config(pins);
            esp_ok(camera::esp_camera_init(&config), "esp_camera_reinit")?;
            CAMERA_READY.store(true, Ordering::Relaxed);
            let fb = camera::esp_camera_fb_get();
            if fb.is_null() {
                bail!("esp_camera_fb_get returned null after reinit");
            }
            let data = std::slice::from_raw_parts((*fb).buf, (*fb).len).to_vec();
            camera::esp_camera_fb_return(fb);
            return Ok(data);
        }
        let data = std::slice::from_raw_parts((*fb).buf, (*fb).len).to_vec();
        camera::esp_camera_fb_return(fb);
        Ok(data)
    }
}

/// Start a background MJPEG HTTP stream thread on `port` with token auth.
/// Browsers/VLC can open: http://<device-ip>:<port>/stream?token=<random>
/// Call STREAM_ACTIVE.store(false) or /camera stream stop to shut down.
#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn start_mjpeg_stream_thread(pins: CameraPins, port: u16, stream_token: String) {
    use std::io::{BufRead as _, BufReader as StdBufReader, Write};
    use std::net::TcpListener;

    STREAM_ACTIVE.store(true, Ordering::Relaxed);

    let _ = std::thread::Builder::new()
        .name("mjpeg".to_string())
        .stack_size(16 * 1024)
        .spawn(move || {
            let bind_addr = format!("0.0.0.0:{}", port);
            let listener = match TcpListener::bind(&bind_addr) {
                Ok(l) => l,
                Err(e) => {
                    warn!("MJPEG bind failed: {}", e);
                    STREAM_ACTIVE.store(false, Ordering::Relaxed);
                    return;
                }
            };
            // accept() will block; set a 3s read timeout so we can check STREAM_ACTIVE
            let _ = listener.set_nonblocking(false);
            info!("MJPEG stream listening on :{}", port);

            while STREAM_ACTIVE.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _addr)) => {
                        // Read first HTTP request line and validate token
                        let authorized = {
                            let read_stream = match stream.try_clone() {
                                Ok(s) => s,
                                Err(_) => { continue; }
                            };
                            let mut reader = StdBufReader::new(read_stream);
                            let mut request_line = String::new();
                            if reader.read_line(&mut request_line).is_err() { false }
                            else {
                                let expected = format!("token={}", stream_token);
                                request_line.contains(&expected)
                            }
                        };
                        if !authorized {
                            let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\n\r\nAccess denied. Invalid or missing token.\n");
                            continue;
                        }

                        // Send HTTP 200 + MJPEG multipart headers
                        let hdr = b"HTTP/1.1 200 OK\r\n\
                            Content-Type: multipart/x-mixed-replace; boundary=--jpgframe\r\n\
                            Cache-Control: no-cache, no-store, must-revalidate\r\n\
                            Pragma: no-cache\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            \r\n";
                        if stream.write_all(hdr).is_err() { continue; }

                        // Stream frames until told to stop or client disconnects
                        while STREAM_ACTIVE.load(Ordering::Relaxed) {
                            match capture_camera_jpeg(&pins) {
                                Ok(jpeg) => {
                                    let part_hdr = format!(
                                        "--jpgframe\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                                        jpeg.len()
                                    );
                                    if stream.write_all(part_hdr.as_bytes()).is_err() { break; }
                                    if stream.write_all(&jpeg).is_err() { break; }
                                    if stream.write_all(b"\r\n").is_err() { break; }
                                }
                                Err(e) => {
                                    warn!("MJPEG capture error: {}", e);
                                    std::thread::sleep(Duration::from_millis(500));
                                }
                            }
                            std::thread::sleep(Duration::from_millis(150)); // ~6 fps
                        }
                    }
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            }
            info!("MJPEG stream thread stopped");
        });
}

fn send_resend_email(api_key: &str, from: &str, to: &str, subject: &str, body: &str) -> Result<String> {
    let payload = serde_json::json!({
        "from": from,
        "to": [to],
        "subject": subject,
        "text": body,
    });
    let json_body = serde_json::to_string(&payload)?;
    let body_bytes = json_body.as_bytes();
    let content_len = body_bytes.len().to_string();
    let auth = format!("Bearer {}", api_key);

    let mut client = EspHttpConnection::new(&new_http_config(60))?;
    client.initiate_request(
        esp_idf_svc::http::Method::Post,
        RESEND_API_URL,
        &[
            ("Content-Type", "application/json"),
            ("Content-Length", &content_len),
            ("Authorization", &auth),
        ],
    )?;
    client.write_all(body_bytes)?;
    client.initiate_response()?;

    let mut resp = Vec::with_capacity(2048);
    let mut buf = [0u8; 1024];
    loop {
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => resp.extend_from_slice(&buf[..n]),
            Err(e) => {
                warn!("email read: {:?}", e);
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&resp).to_string())
}

fn handle_direct_email(
    cfg: &Config,
    nvs: &mut EspNvs<NvsDefault>,
    chat_id: i64,
    arg: &str,
) {
    if let Some(rest) = arg.strip_prefix("config ") {
        let parts: Vec<&str> = rest.splitn(3, '|').collect();
        if parts.len() < 3 {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "Usage: /email config from@example.com|default_to@example.com|resend_api_key");
            return;
        }
        let _ = nvs.set_str("email_from", parts[0].trim());
        let _ = nvs.set_str("email_to", parts[1].trim());
        let _ = nvs.set_str("email_api_key", parts[2].trim());
        let _ = send_telegram(&cfg.tg_token, chat_id,
            &format!("✅ 已儲存 ESP 郵件設定\n寄件者：{}\n預設收件者：{}", parts[0].trim(), parts[1].trim()));
        return;
    }

    if arg == "status" || arg.is_empty() {
        let from = nvs_get(nvs, "email_from").unwrap_or_else(|| "(未設定)".to_string());
        let to = nvs_get(nvs, "email_to").unwrap_or_else(|| "(未設定)".to_string());
        let key = if nvs_get(nvs, "email_api_key").is_some() { "已設定" } else { "未設定" };
        let _ = send_telegram(&cfg.tg_token, chat_id,
            &format!("ESP 郵件功能狀態\n寄件者：{}\n預設收件者：{}\nAPI Key：{}\n\n用法：\n/email config from|to|resend_key\n/email to@example.com|主旨|內容\n/email |主旨|內容  (使用預設收件者)", from, to, key));
        return;
    }

    let parts: Vec<&str> = arg.splitn(3, '|').collect();
    if parts.len() < 2 {
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "Usage: /email to@example.com|subject|body\n💡 /email |subject|body 使用預設收件者");
        return;
    }

    let api_key = match nvs_get(nvs, "email_api_key") {
        Some(v) => v,
        None => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "ESP 郵件功能尚未設定，請先執行 /email config from|to|resend_api_key");
            return;
        }
    };
    let from = match nvs_get(nvs, "email_from") {
        Some(v) => v,
        None => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "缺少寄件人地址，請先 /email config 設定");
            return;
        }
    };

    let default_to = nvs_get(nvs, "email_to").unwrap_or_default();
    let to = if parts.len() == 2 || parts[0].trim().is_empty() {
        default_to.as_str()
    } else {
        parts[0].trim()
    };
    let subject = if parts.len() == 2 { parts[0].trim() } else { parts[1].trim() };
    let body = if parts.len() == 2 { parts[1].trim() } else { parts[2].trim() };

    if to.is_empty() {
        let _ = send_telegram(&cfg.tg_token, chat_id, "沒有收件者，請設定 to 或預先設置預設收件者");
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("📧 正在透過 ESP 寄信到 {}...", to));
    match send_resend_email(&api_key, &from, to, subject, body) {
        Ok(resp) => {
            let preview = &resp[..resp.len().min(200)];
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("✅ 郵件已送出 → {}\n主旨: {}\n預覽: {}", to, subject, preview));
        }
        Err(e) => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("❌ ESP 郵件寄送失敗: {}\n如果設定正確，可能是本裝置不支援直接寄信，試試 PC Driver SMTP 方式", e));
        }
    }
}

// ===== Diagnostic System =====

struct DiagEntry {
    timestamp: u64,
    level: &'static str,
    message: String,
}

struct DiagBuffer {
    entries: Vec<DiagEntry>,
}

impl DiagBuffer {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn log(&mut self, level: &'static str, msg: &str) {
        if self.entries.len() >= MAX_DIAG_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(DiagEntry {
            timestamp: now_epoch(),
            level,
            message: msg.to_string(),
        });
    }

    fn format_report(&self) -> String {
        if self.entries.is_empty() {
            return "No diagnostic entries.".to_string();
        }
        let mut s = format!("Diagnostics ({} entries):\n", self.entries.len());
        for e in &self.entries {
            let (h, m) = epoch_to_hhmm(e.timestamp);
            s.push_str(&format!("[{:02}:{:02}] {} {}\n", h, m, e.level, e.message));
        }
        s
    }

    fn format_for_gemini(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut s = "\n\nRecent system diagnostics:\n".to_string();
        for e in self.entries.iter().rev().take(10) {
            s.push_str(&format!("- [{}] {}\n", e.level, e.message));
        }
        s
    }
}

// ===== SKILL System =====

#[derive(Clone)]
struct Skill {
    name: String,
    description: String,
    trigger: String,
}

fn load_skills_from_nvs(nvs: &EspNvs<NvsDefault>) -> Vec<Skill> {
    let count: usize = nvs_get(nvs, "sk_cnt")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let mut skills = Vec::new();
    for i in 0..count.min(MAX_SKILLS) {
        let nkey = format!("sk{:02}", i);
        if let Some(val) = nvs_get(nvs, &nkey) {
            let parts: Vec<&str> = val.splitn(3, '|').collect();
            if parts.len() >= 3 {
                skills.push(Skill {
                    name: parts[0].to_string(),
                    description: parts[1].to_string(),
                    trigger: parts[2].to_string(),
                });
            }
        }
    }
    skills
}

fn save_skills_to_nvs(nvs: &mut EspNvs<NvsDefault>, skills: &[Skill]) {
    let _ = nvs.set_str("sk_cnt", &skills.len().to_string());
    for (i, sk) in skills.iter().enumerate() {
        let nkey = format!("sk{:02}", i);
        let combined = format!("{}|{}|{}", sk.name, sk.description, sk.trigger);
        let _ = nvs.set_str(&nkey, &combined);
    }
    for i in skills.len()..MAX_SKILLS {
        let nkey = format!("sk{:02}", i);
        let _ = nvs.remove(&nkey);
    }
}

fn format_skills_for_prompt(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut s = String::from(
        "\n\n== ACTIVE SKILLS ??MANDATORY RULES ==\n\
        When user input matches a skill's TRIGGER pattern, you MUST emit the exact tag/syntax described.\n\
        Do NOT describe the command in prose ??execute directly by including the tag in your reply.\n\
        Emitting the correct tag IS the action; the firmware handles the rest.\n"
    );
    for sk in skills {
        s.push_str(&format!("\n[SKILL: {}]\n  TRIGGER: {}\n  ACTION (follow exactly): {}\n",
            sk.name, sk.trigger, sk.description));
    }
    s.push_str("\n== END SKILLS ==\n");
    s
}

fn builtin_skills() -> Vec<Skill> {
    vec![
        Skill {
            name: "calculator".to_string(),
            description: "Perform math calculations".to_string(),
            trigger: "calculate".to_string(),
        },
        Skill {
            name: "translator".to_string(),
            description: "Translate text between languages".to_string(),
            trigger: "translate".to_string(),
        },
        Skill {
            name: "code_gen".to_string(),
            description: "Generate Python code and execute on PC using [PY:...] or [PYBLOCK]...[/PYBLOCK]".to_string(),
            trigger: "code".to_string(),
        },
        Skill {
            name: "image_gen".to_string(),
            description: "Generate images from text description with [IMG:prompt]".to_string(),
            trigger: "draw".to_string(),
        },
        Skill {
            name: "excel".to_string(),
            description: "Read, analyze, modify Excel spreadsheets with [EXCEL:path>>>instruction]".to_string(),
            trigger: "excel".to_string(),
        },
        Skill {
            name: "board_voice_control".to_string(),
            description: "Controls board TTS speech. ALWAYS emit exactly one tag AND confirm the new mode in text.
  [CTRL:voice:off]    \u{2705} board goes completely silent (no TTS for ANY message, wake word, or AutoTask)
  [CTRL:voice:normal] \u{2705} board speaks full replies
  [CTRL:voice:brief]  \u{2705} board speaks first sentence only
  TRIGGERS: mute/unmute/silence/quiet/voice on/voice off/brief/tts
  IMPORTANT: After emitting the tag, state the mode change clearly so user knows it took effect.".to_string(),
            trigger: "voice|mute|unmute|tts|brief|silent".to_string(),
        },
        Skill {
            name: "board_wake_control".to_string(),
            description: "When user asks to enable wake, disable wake, or change the wake phrase, emit exactly one control tag: [CTRL:wake:on], [CTRL:wake:off], or [CTRL:wake:set:phrase]. Use the full phrase inside the tag and avoid prose-only answers.".to_string(),
            trigger: "wake".to_string(),
        },
        Skill {
            name: "board_wifi_help".to_string(),
            description: "Explain runtime Wi-Fi commands: /wifi status, /wifi set<N> SSID PASSWORD (N=1-5), /wifi del <N>, /wifi swap, /wifi clear".to_string(),
            trigger: "wifi".to_string(),
        },
        Skill {
            name: "reminder_control".to_string(),
            description: "Manages Reminders. Rules:\
  Rule 1 - List: emit [REMIND:list]; board auto-replies with full list (ID + countdown).\
  Rule 2 - Delete: emit [REMIND:del:ID] (get ID from the list).\
  Rule 3 - If user wants to stop/delete but gave no ID, emit [REMIND:list] first to show IDs, then emit [REMIND:del:ID].\
  Syntax: [REMIND:interval:MINUTES:msg] / [REMIND:daily:HH:MM:msg] / [REMIND:once:HH:MM:msg].\
  Shortcut cmds: /reminders (view) | /remind del ID (delete one) | /reminders clear (delete all).\
  NEVER use prose only -- always emit the correct tag AND confirm.".to_string(),
            trigger: "remind|reminder|alarm|reminders".to_string(),
        },
        Skill {
            name: "pc_control_bridge".to_string(),
            description: "When user asks for desktop actions, emit an exact PC tag instead of describing what to type. Use [PC:screenshot], [PC:status], [PC:shell:cmd], [PC:file_read:path], [PC:file_list:path], or [PC:file_write:path>>>content]. Keep the tag exact and place it before any short confirmation text.".to_string(),
            trigger: "pc".to_string(),
        },
        Skill {
            name: "desktop_screenshot".to_string(),
            description: "For screenshot requests, emit exactly [PC:screenshot]. Do not explain the command or mention alternate syntax unless asked. The desktop driver will return the image to Telegram.".to_string(),
            trigger: "screenshot".to_string(),
        },
        Skill {
            name: "lcd_expression".to_string(),
            description: "Choose one LCD expression tag near the start of a reply: [FACE:neutral], [FACE:happy], [FACE:wink], [FACE:love], [FACE:thinking], or [FACE:sad]".to_string(),
            trigger: "face".to_string(),
        },
        Skill {
            name: "camera_snap".to_string(),
            description: "ONLY when user asks to take a photo WITHOUT any analysis request (just \u{62cd}\u{7167}/snap/photo with nothing else). Emit [CAM:snap]. If user also asks a question about the photo, use camera_vision instead!".to_string(),
            trigger: "photo".to_string(),
        },
        Skill {
            name: "camera_vision".to_string(),
            description: "When user wants photo+analysis or asks to look/identify (\u{62cd}\u{7167}\u{8acb}\u{544a}\u{8a34}\u{6211}.../\u{770b}\u{4e00}\u{4e0b}/\u{5e6b}\u{6211}\u{770b}/\u{8fa8}\u{8b58}/\u{5206}\u{6790}): emit [CAM:vision:user's question or description request]. Always prefer this over snap when user wants ANY information about what's in the photo.".to_string(),
            trigger: "vision".to_string(),
        },
        Skill {
            name: "auto_workflow".to_string(),
            description: "Sets up a recurring background AutoTask (camera + AI analysis + optional speech).\
  Syntax: [AUTOTASK:camera:INTERVAL_MIN:AI_prompt:speak] where speak=TTS, nospeak=send Telegram.\
  Delete a task: [AUTOTASK:del:ID]. List all tasks: [AUTOTASK:list].\
  Write the AI prompt in Traditional Chinese describing what Gemini should report.\
  Example: user says every 5 min take photo and say who is sleeping -> [AUTOTASK:camera:5:請描述畫面中誰在睡覺:speak].\
  Tip: interval < 5 min causes heavy API load; recommend 5+ min.".to_string(),
            trigger: "autotask|workflow|auto task|recurring".to_string(),
        },
        Skill {
            name: "task_control".to_string(),
            description: "Manages board background AutoTasks.\
  Rule 1 - List tasks: emit [AUTOTASK:list]; board replies with task list + IDs.\
  Rule 2 - Stop/cancel/delete: emit [AUTOTASK:del:ID] (get ID from list first if unknown).\
  Rule 3 - If user says stop/kill without giving ID, emit [AUTOTASK:list] to get IDs, then emit [AUTOTASK:del:ID].\
  Rule 4 - Clear all: user can type /tasks clear.\
  Shortcut cmds (faster): /tasks (list), /tasks del ID (stop one), /tasks clear (all).\
  NEVER describe the action in prose only -- always emit the correct tag.".to_string(),
            trigger: "task|tasks|kill task|stop task|autotask list|autotask del".to_string(),
        },
    ]
}

fn ensure_builtin_skills(skills: &mut Vec<Skill>) -> bool {
    let mut changed = false;
    for skill in builtin_skills() {
        if let Some(existing) = skills.iter_mut().find(|e| e.name.eq_ignore_ascii_case(&skill.name)) {
            // Update description/trigger if changed
            if existing.description != skill.description || existing.trigger != skill.trigger {
                existing.description = skill.description;
                existing.trigger = skill.trigger;
                changed = true;
            }
            continue;
        }
        if skills.len() >= MAX_SKILLS {
            break;
        }
        skills.push(skill);
        changed = true;
    }
    changed
}

// ===== USB PC Driver Protocol =====

static USB_CMD_ID: AtomicU32 = AtomicU32::new(0);
/// Mutex to prevent multi-thread interleaving of JSON lines on UART0 TX.
static SERIAL_TX_LOCK: Mutex<()> = Mutex::new(());
/// Set to true once PC driver responds (handshake / any valid JSON ack).
static DRIVER_CONNECTED: AtomicBool = AtomicBool::new(false);

/// PC Safe Mode — when true (default), AI-generated [PC:], [PY:], [IMG:], [EXCEL:] tags
/// are blocked. Only explicit user commands (e.g. /screenshot) and safe read-only queries
/// are allowed. User can toggle via /pc_unlock and /pc_lock.
static PC_SAFE_MODE: AtomicBool = AtomicBool::new(true);

/// Commands that are always safe even in safe mode (read-only or status queries).
const PC_SAFE_CMDS: &[&str] = &["status", "screenshot", "clipboard_get", "process_list", "file_read", "file_list", "scan_inbox", "find_window"];

/// Check if a PC command is allowed under current safe mode.
fn pc_cmd_allowed(cmd: &str, from_ai: bool) -> bool {
    if !from_ai { return true; } // Direct user commands always pass
    if !PC_SAFE_MODE.load(Ordering::Relaxed) { return true; } // Safe mode off
    PC_SAFE_CMDS.contains(&cmd) // Only allow safe read-only commands
}

fn usb_next_id() -> String {
    let id = USB_CMD_ID.fetch_add(1, Ordering::Relaxed);
    format!("{:04}", id)
}

/// Send a JSON command to the PC driver via USB Serial (println ??UART0 ??CH340 ??PC).
/// PC driver reads JSON lines, executes, and sends result to user via Telegram.
fn usb_send_pc_cmd(cmd: &str, args: &serde_json::Value, chat_id: i64) {
    let id = usb_next_id();
    let msg = serde_json::json!({
        "cmd": cmd,
        "args": args,
        "id": id,
        "chat_id": chat_id.to_string(),
    });
    // Lock to prevent other threads' println!/info! from interleaving mid-JSON
    {
        let _lock = SERIAL_TX_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        println!("{}", msg);
    }
    info!("USB CMD #{}: {} -> PC", id, cmd);
}

fn spawn_serial_command_reader() -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => std::thread::sleep(Duration::from_millis(100)),
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let _ = tx.send(trimmed.to_string());
                    }
                }
                Err(_) => std::thread::sleep(Duration::from_millis(250)),
            }
        }
    });
    rx
}

fn update_lcd_camera_status(nvs: &EspNvs<NvsDefault>, headline: &str, subtitle: &str) {
    let lcd_pins = load_lcd_pins(nvs);
    if let Err(err) = draw_lcd_scene(&lcd_pins, headline, subtitle) {
        warn!("LCD camera status update failed: {}", err);
    }
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn handle_camera_command(
    cfg: &Config,
    nvs: &mut EspNvs<NvsDefault>,
    state: &mut AppState,
    chat_id: i64,
    args: &str,
) {
    let trimmed = args.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        let pins = load_camera_pins(nvs);
        update_lcd_camera_status(nvs, "CAMERA", "Ready for snap / vision");
        let msg = format!(
            "OV3660 Camera\nCurrent model: {}\n\n{}\n\nCommands:\n/camera default\n/camera pins pwdn=-1 reset=-1 xclk=15 sda=4 scl=5 d0=11 d1=9 d2=8 d3=10 d4=12 d5=18 d6=17 d7=16 vsync=6 href=7 pclk=13\n/camera snap\n/camera vision\n/camera vision 說明桌上的東西",
            state.current_model,
            pins.status_text(),
        );
        let _ = send_telegram(&cfg.tg_token, chat_id, &msg);
        return;
    }

    if trimmed.eq_ignore_ascii_case("default") {
        let pins = CameraPins::default();
        match save_camera_pins(nvs, &pins) {
            Ok(()) => {
                update_lcd_camera_status(nvs, "CAMERA", "Pins reset to default");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Camera pins reset to default\n\n{}", pins.status_text()));
            }
            Err(err) => {
                update_lcd_camera_status(nvs, "ERROR", "Camera default save failed");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Camera default save failed: {}", err));
            }
        }
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("pins ") {
        let mut pins = load_camera_pins(nvs);
        match parse_camera_pin_args(rest, &mut pins).and_then(|_| save_camera_pins(nvs, &pins)) {
            Ok(()) => {
                update_lcd_camera_status(nvs, "CAMERA", "Pins updated");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Camera pins updated\n\n{}", pins.status_text()));
            }
            Err(err) => {
                update_lcd_camera_status(nvs, "ERROR", "Camera pin update failed");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Camera pin update failed: {}", err));
            }
        }
        return;
    }

    if trimmed.eq_ignore_ascii_case("selftest") {
        let _ = send_telegram(&cfg.tg_token, chat_id, "📷 準備執行 camera self-test...");
        update_lcd_camera_status(nvs, "CAMERA", "Running self-test");
        let pins = load_camera_pins(nvs);
        match capture_camera_jpeg(&pins) {
            Ok(jpeg) => {
                update_lcd_camera_status(nvs, "CAMERA", &format!("Self-test OK {} KB", jpeg.len() / 1024));
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Camera self-test OK\nJPEG: {} KB", jpeg.len() / 1024));
            }
            Err(err) => {
                update_lcd_camera_status(nvs, "ERROR", "Camera self-test failed");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Camera self-test failed: {}", err));
            }
        }
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("vision") {
        let prompt = rest.trim().trim_start_matches(':').trim();
        let prompt = if prompt.is_empty() {
            "請用繁體中文簡單描述這張照片的重點、人物、物品、背景，若看到文字也請直接說出來"
        } else {
            prompt
        };

        let _ = send_telegram(&cfg.tg_token, chat_id, "📷 準備拍照...");
        update_lcd_camera_status(nvs, "CAMERA", "Vision capture in progress");
        let pins = load_camera_pins(nvs);
        match capture_camera_jpeg(&pins) {
            Ok(jpeg) => {
                let size_kb = jpeg.len() / 1024;
                update_lcd_camera_status(nvs, "CAMERA", &format!("Photo {} KB, analyzing...", size_kb));
                // 1. 先把照片傳給用戶
                let caption = format!("📷 {} KB — 正在分析...", size_kb);
                let _ = send_telegram_camera_jpeg(&cfg.tg_token, chat_id, &jpeg, &caption);
                // 2. 再問 Gemini Vision
                match describe_image_with_gemini(&cfg.gemini_key, state, &jpeg, prompt) {
                    Ok(reply) => {
                        update_lcd_camera_status(nvs, "CAMERA", "Vision OK");
                        let _ = send_telegram(&cfg.tg_token, chat_id, &reply);
                    }
                    Err(err) => {
                        update_lcd_camera_status(nvs, "ERROR", "Gemini Vision failed");
                        let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Gemini 分析失敗: {}", err));
                    }
                }
            }
            Err(err) => {
                update_lcd_camera_status(nvs, "ERROR", "Camera vision failed");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Camera vision failed: {}\n\n請用 /camera 重試或調整 pin 設定", err));
            }
        }
        return;
    }

    // ── MJPEG HTTP stream ──────────────────────────────────────────────────
    if trimmed.eq_ignore_ascii_case("stream stop") || trimmed.eq_ignore_ascii_case("stream off") {
        STREAM_ACTIVE.store(false, Ordering::Relaxed);
        update_lcd_camera_status(nvs, "CAMERA", "Stream stopped");
        let _ = send_telegram(&cfg.tg_token, chat_id, "📡 MJPEG 串流已停止");
        return;
    }

    if trimmed.eq_ignore_ascii_case("stream") || trimmed.starts_with("stream ") {
        if STREAM_ACTIVE.load(Ordering::Relaxed) {
            let _ = send_telegram(&cfg.tg_token, chat_id, "📡 串流已在執行中\n停止: /camera stream stop");
            return;
        }
        let pins = load_camera_pins(nvs);
        let port: u16 = trimmed.strip_prefix("stream ").and_then(|s| s.trim().parse().ok()).unwrap_or(8080);
        // Generate random stream token for access control
        let stream_token = format!("{:08x}", now_epoch() ^ (std::process::id() as u64 * 2654435761));
        start_mjpeg_stream_thread(pins, port, stream_token.clone());
        update_lcd_camera_status(nvs, "CAMERA", &format!("MJPEG :{}  live", port));
        let device_ip = nvs_get(nvs, "wifi_cur_ip").unwrap_or_else(|| "192.168.x.x".to_string());
        let _ = send_telegram(&cfg.tg_token, chat_id,
            &format!("📡 MJPEG 串流已啟動（port {}）\n\n瀏覽器/VLC 開啟：\nhttp://{}:{}/stream?token={}\n\n⚠️ 此連結含存取 token，請勿分享\n\n停止：/camera stream stop", port, device_ip, port, stream_token));
        return;
    }
    // ─────────────────────────────────────────────────────────────────────

    if trimmed.eq_ignore_ascii_case("snap") {
        let _ = send_telegram(&cfg.tg_token, chat_id, "📷 準備拍照...");
        update_lcd_camera_status(nvs, "CAMERA", "Snap in progress");
        let pins = load_camera_pins(nvs);
        match capture_camera_jpeg(&pins) {
            Ok(jpeg) => {
                update_lcd_camera_status(nvs, "CAMERA", &format!("Snap OK {} KB", jpeg.len() / 1024));
                let caption = format!("ETHAN 拍照完成，JPEG 大小 {} KB", jpeg.len() / 1024);
                let upload_result = send_telegram_camera_jpeg(&cfg.tg_token, chat_id, &jpeg, &caption);
                if let Err(err) = upload_result {
                    update_lcd_camera_status(nvs, "ERROR", "Photo upload failed");
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Photo upload failed after retry: {}", err));
                }
            }
            Err(err) => {
                update_lcd_camera_status(nvs, "ERROR", "Camera snap failed");
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Camera snap failed: {}", err));
            }
        }
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id, "Usage:\n/camera                  (狀態)\n/camera snap             (拍照並傳照片)\n/camera vision           (拍照+Gemini分析)\n/camera vision <問題>    (拍照+指定問題)\n/camera stream           (MJPEG串流，瀏覽器可看)\n/camera stream stop      (停止串流)\n/camera selftest\n/camera default\n/camera pins pwdn=-1 reset=-1 xclk=15 sda=4 scl=5 d0=11 d1=9 d2=8 d3=10 d4=12 d5=18 d6=17 d7=16 vsync=6 href=7 pclk=13");
}

#[cfg(not(esp_idf_comp_espressif__esp32_camera_enabled))]
fn handle_camera_command(
    cfg: &Config,
    _nvs: &mut EspNvs<NvsDefault>,
    _state: &mut AppState,
    chat_id: i64,
    _args: &str,
) {
    let _ = send_telegram(&cfg.tg_token, chat_id, "Camera component is not enabled in this build.");
}

/// Parse [PC:...] tags from Gemini response and send USB commands.
/// Returns (cleaned_reply, had_pc_commands).
/// Max 5 PC commands per AI reply to prevent runaway loops.
fn parse_pc_tags(reply: &str, chat_id: i64) -> (String, bool) {
    const MAX_PC_CMDS_PER_REPLY: usize = 5;
    let mut clean = reply.to_string();
    let mut applied = false;
    let mut cmd_count: usize = 0;

    while let Some(start) = clean.find("[PC:") {
        if cmd_count >= MAX_PC_CMDS_PER_REPLY {
            warn!("PC command limit ({}) reached, skipping remaining [PC:] tags", MAX_PC_CMDS_PER_REPLY);
            // Strip remaining [PC:...] tags without executing
            while let Some(s) = clean.find("[PC:") {
                if let Some(e) = clean[s..].find(']') {
                    clean = format!("{}{}", &clean[..s], clean[s+e+1..].trim_start());
                } else {
                    break;
                }
            }
            break;
        }
        if let Some(end) = clean[start..].find(']') {
            let tag = &clean[start+4..start+end];
            // Split into cmd and raw_arg at FIRST colon
            let (cmd, raw_arg) = if let Some(pos) = tag.find(':') {
                (&tag[..pos], &tag[pos+1..])
            } else {
                (tag, "")
            };

            let args_json = match cmd {
                "shell" => serde_json::json!({"command": raw_arg}),
                "file_read" => serde_json::json!({"path": raw_arg}),
                "file_list" => serde_json::json!({"path": if raw_arg.is_empty() { "." } else { raw_arg }}),
                "file_write" => {
                    let parts: Vec<&str> = raw_arg.splitn(2, ">>>").collect();
                    serde_json::json!({"path": parts.first().unwrap_or(&""), "content": parts.get(1).unwrap_or(&"")})
                }
                "open" => serde_json::json!({"target": raw_arg}),
                "screenshot" => serde_json::json!({}),
                "analyze_image" => serde_json::json!({"path": raw_arg}),
                "scan_inbox" => serde_json::json!({}),
                "status" => serde_json::json!({}),
                "excel_read" => serde_json::json!({"action": "read", "path": raw_arg}),
                "excel_write" => {
                    let parts: Vec<&str> = raw_arg.splitn(3, ">>>").collect();
                    serde_json::json!({"action": "write", "path": parts.first().unwrap_or(&""), "cell": parts.get(1).unwrap_or(&"A1"), "value": parts.get(2).unwrap_or(&"")})
                }
                "email_send" => {
                    let parts: Vec<&str> = raw_arg.splitn(3, ">>>").collect();
                    serde_json::json!({"action": "send", "to": parts.first().unwrap_or(&""), "subject": parts.get(1).unwrap_or(&""), "body": parts.get(2).unwrap_or(&"")})
                }
                "clipboard_get" => serde_json::json!({"action": "get"}),
                "clipboard_set" => serde_json::json!({"action": "set", "text": raw_arg}),
                "process_list" => serde_json::json!({"action": "list"}),
                "click" => {
                    let coords: Vec<&str> = raw_arg.split(',').collect();
                    let x: i32 = coords.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let y: i32 = coords.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    serde_json::json!({"x": x, "y": y, "button": "left"})
                }
                "type" => serde_json::json!({"text": raw_arg}),
                "hotkey" => {
                    let keys: Vec<&str> = raw_arg.split('+').map(|s| s.trim()).collect();
                    serde_json::json!({"keys": keys})
                }
                "mouse_move" => {
                    let coords: Vec<&str> = raw_arg.split(',').collect();
                    let x: i32 = coords.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let y: i32 = coords.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    serde_json::json!({"x": x, "y": y})
                }
                "scroll" => {
                    let amount: i32 = raw_arg.trim().parse().unwrap_or(3);
                    serde_json::json!({"amount": amount})
                }
                "find_window" => serde_json::json!({"title": raw_arg}),
                "focus_window" => serde_json::json!({"title": raw_arg}),
                _ => serde_json::json!({"command": raw_arg}),
            };

            // Map compound command names to the actual driver command
            let driver_cmd = if cmd.starts_with("excel") { "excel" }
                else if cmd.starts_with("email") { "email" }
                else if cmd.starts_with("clipboard") { "clipboard" }
                else if cmd.starts_with("process") { "process" }
                else if cmd == "mouse_move" { "mouse_move" }
                else if cmd == "find_window" { "find_window" }
                else if cmd == "focus_window" { "focus_window" }
                else if cmd == "analyze_image" { "analyze_image" }
                else if cmd == "scan_inbox" { "scan_inbox" }
                else { cmd };

            // Enforce PC Safe Mode: block dangerous AI-generated commands
            if !pc_cmd_allowed(driver_cmd, true) {
                warn!("PC Safe Mode: blocked AI command [{}]", driver_cmd);
                clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
                continue;
            }

            usb_send_pc_cmd(driver_cmd, &args_json, chat_id);
            applied = true;
            cmd_count += 1;

            clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

// ===== Config =====

struct Config {
    wifi: Vec<(String, String)>,  // up to 5 (ssid, pass) pairs
    tg_token: String,
    gemini_key: String,
    chat_id: String,
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
const CAMERA_NVS_KEY: &str = "camera_pins";

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct CameraPins {
    pwdn: i32,
    reset: i32,
    xclk: i32,
    sda: i32,
    scl: i32,
    d0: i32,
    d1: i32,
    d2: i32,
    d3: i32,
    d4: i32,
    d5: i32,
    d6: i32,
    d7: i32,
    vsync: i32,
    href: i32,
    pclk: i32,
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
impl Default for CameraPins {
    fn default() -> Self {
        Self {
            pwdn: -1,
            reset: -1,
            xclk: 15,
            sda: 4,
            scl: 5,
            d0: 11,
            d1: 9,
            d2: 8,
            d3: 10,
            d4: 12,
            d5: 18,
            d6: 17,
            d7: 16,
            vsync: 6,
            href: 7,
            pclk: 13,
        }
    }
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
impl CameraPins {
    fn apply_setting(&mut self, key: &str, value: i32) -> Result<()> {
        match key {
            "pwdn" => self.pwdn = value,
            "reset" => self.reset = value,
            "xclk" => self.xclk = value,
            "sda" => self.sda = value,
            "scl" => self.scl = value,
            "d0" => self.d0 = value,
            "d1" => self.d1 = value,
            "d2" => self.d2 = value,
            "d3" => self.d3 = value,
            "d4" => self.d4 = value,
            "d5" => self.d5 = value,
            "d6" => self.d6 = value,
            "d7" => self.d7 = value,
            "vsync" => self.vsync = value,
            "href" => self.href = value,
            "pclk" => self.pclk = value,
            _ => bail!("unknown camera pin field: {}", key),
        }
        Ok(())
    }

    fn status_text(&self) -> String {
        format!(
            "OV3660 pins\npwdn={} reset={} xclk={} sda={} scl={}\nd0={} d1={} d2={} d3={} d4={} d5={} d6={} d7={}\nvsync={} href={} pclk={}",
            self.pwdn,
            self.reset,
            self.xclk,
            self.sda,
            self.scl,
            self.d0,
            self.d1,
            self.d2,
            self.d3,
            self.d4,
            self.d5,
            self.d6,
            self.d7,
            self.vsync,
            self.href,
            self.pclk,
        )
    }
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn load_camera_pins(nvs: &EspNvs<NvsDefault>) -> CameraPins {
    nvs_get(nvs, CAMERA_NVS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn save_camera_pins(nvs: &mut EspNvs<NvsDefault>, pins: &CameraPins) -> Result<()> {
    let raw = serde_json::to_string(pins)?;
    nvs.set_str(CAMERA_NVS_KEY, &raw)?;
    Ok(())
}

#[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
fn parse_camera_pin_args(input: &str, pins: &mut CameraPins) -> Result<()> {
    for token in input.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("bad camera pin token: {}", token))?;
        let parsed = value
            .parse::<i32>()
            .map_err(|_| anyhow::anyhow!("invalid pin value for {}", key))?;
        pins.apply_setting(key, parsed)?;
    }
    Ok(())
}

// ===== Reminder System =====

#[derive(Clone, Debug)]
struct Reminder {
    id: u32,
    rtype: ReminderType,
    interval_secs: u64,
    next_trigger: u64,
    hour: u32,    // for Daily/Once: original scheduled hour
    minute: u32,  // for Daily/Once: original scheduled minute
    message: String,
    active: bool,
}

#[derive(Clone, Debug)]
struct AutoTask {
    id: u32,
    interval_secs: u64,
    next_trigger: u64,
    action: String,   // "camera"
    prompt: String,
    speak: bool,
    active: bool,
}

#[derive(Clone, Debug)]
enum ReminderType {
    Interval,
    Daily,
    Once,
}

struct AppState {
    reminders: Vec<Reminder>,
    next_reminder_id: u32,
    auto_tasks: Vec<AutoTask>,
    next_autotask_id: u32,
    tokens_in: u64,
    tokens_out: u64,
    requests: u64,
    history: Vec<(String, String)>,
    boot_time: u64,
    memories: Vec<(String, String)>,
    diag: DiagBuffer,
    skills: Vec<Skill>,
    last_tokens_in: u64,
    last_tokens_out: u64,
    current_model: String,
    voice_mode: String,
    wake_phrase: String,
    wake_enabled: bool,
    last_wake_check: u64,
    last_wake_trigger: u64,
    last_briefing_day: i64,
}

impl AppState {
    fn new(now: u64) -> Self {
        Self {
            reminders: Vec::new(),
            next_reminder_id: 1,
            auto_tasks: Vec::new(),
            next_autotask_id: 1,
            tokens_in: 0,
            tokens_out: 0,
            requests: 0,
            history: Vec::new(),
            boot_time: now,
            memories: Vec::new(),
            diag: DiagBuffer::new(),
            skills: Vec::new(),
            last_tokens_in: 0,
            last_tokens_out: 0,
            current_model: DEFAULT_GEMINI_MODEL.to_string(),
            voice_mode: "normal".to_string(),
            wake_phrase: DEFAULT_WAKE_PHRASE.to_string(),
            wake_enabled: false,
            last_wake_check: 0,
            last_wake_trigger: 0,
            last_briefing_day: -1,
        }
    }

    fn add_reminder(&mut self, rtype: ReminderType, interval_secs: u64,
                    hour: u32, minute: u32, message: &str, now: u64) -> u32 {
        let id = self.next_reminder_id;
        self.next_reminder_id += 1;

        let next_trigger = match rtype {
            ReminderType::Interval => now + interval_secs,
            ReminderType::Daily | ReminderType::Once => {
                next_daily_trigger(hour, minute, now)
            }
        };

        if self.reminders.len() >= MAX_REMINDERS {
            self.reminders.retain(|r| r.active);
        }

        self.reminders.push(Reminder {
            id, rtype, interval_secs, next_trigger,
            hour, minute,
            message: message.to_string(), active: true,
        });
        id
    }

    fn remove_reminder(&mut self, id: u32) -> bool {
        if let Some(r) = self.reminders.iter_mut().find(|r| r.id == id) {
            r.active = false;
            true
        } else {
            false
        }
    }

    fn check_reminders(&mut self, now: u64) -> Vec<String> {
        let mut fired = Vec::new();
        for r in &mut self.reminders {
            if !r.active || now < r.next_trigger {
                continue;
            }
            fired.push(format!("[Reminder] {}", r.message));
            match r.rtype {
                ReminderType::Interval => {
                    r.next_trigger = now + r.interval_secs;
                }
                ReminderType::Daily => {
                    r.next_trigger += 86400;
                }
                ReminderType::Once => {
                    r.active = false;
                }
            }
        }
        fired
    }

    fn list_reminders(&self) -> String {
        let active: Vec<&Reminder> = self.reminders.iter().filter(|r| r.active).collect();
        if active.is_empty() {
            return "📭 沒有提醒\n\n新增：\n  /remind 5m 訊息 — 每 N 分鐘\n  /remind 8:00 訊息 — 每天 8 點".to_string();
        }
        let now = now_epoch();
        let mut s = format!("📋 提醒清單 ({} 筆):\n", active.len());
        for r in &active {
            let type_str = match r.rtype {
                ReminderType::Interval => {
                    let mins = r.interval_secs / 60;
                    let secs = r.interval_secs % 60;
                    if secs == 0 { format!("每{}分", mins) } else { format!("每{}分{}秒", mins, secs) }
                }
                ReminderType::Daily => {
                    let (h, m) = epoch_to_hhmm(r.next_trigger);
                    format!("每天{:02}:{:02}", h, m)
                }
                ReminderType::Once => {
                    let (h, m) = epoch_to_hhmm(r.next_trigger);
                    format!("一次{:02}:{:02}", h, m)
                }
            };
            let countdown = if r.next_trigger > now {
                let rem = r.next_trigger - now;
                if rem >= 3600 { format!("{}時{}分後", rem/3600, (rem%3600)/60) }
                else if rem >= 60 { format!("{}分{}秒後", rem/60, rem%60) }
                else { format!("{}秒後", rem) }
            } else {
                "已觸發".to_string()
            };
            let short = if r.message.chars().count() > 20 {
                format!("{}...", r.message.chars().take(20).collect::<String>())
            } else { r.message.clone() };
            s.push_str(&format!("  #{} {} — {}\n     💬{}\n", r.id, type_str, countdown, short));
        }
        s.push_str("\n────────────────\n");
        s.push_str("🗑️ /remind del [ID編號]\n");
        s.push_str("🧹 清除全部：/reminders clear");
        s
    }

    fn add_history(&mut self, role: &str, text: &str) {
        self.history.push((role.to_string(), text.to_string()));
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
    }

    fn token_report(&self) -> String {
        let total = self.tokens_in + self.tokens_out;
        format!(
            "Token Usage Report\n\
             Model: {}\n\
             Voice Mode: {}\n\
             Wake: {} ({})\n\
             Input: {} tokens\n\
             Output: {} tokens\n\
             Total: {} tokens\n\
             Requests: {}\n\
             Avg/request: {} tokens",
            self.current_model,
            self.voice_mode,
            if self.wake_enabled { "on" } else { "off" },
            self.wake_phrase,
            self.tokens_in,
            self.tokens_out,
            total,
            self.requests,
            if self.requests > 0 { total / self.requests } else { 0 }
        )
    }
}

// ===== Time Helpers =====

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn epoch_to_hhmm(epoch: u64) -> (u32, u32) {
    let local = epoch as i64 + TZ_OFFSET;
    let day_secs = ((local % 86400) + 86400) as u32 % 86400;
    (day_secs / 3600, (day_secs % 3600) / 60)
}

fn next_daily_trigger(hour: u32, minute: u32, now: u64) -> u64 {
    let local_now = now as i64 + TZ_OFFSET;
    let day_start = local_now - (local_now % 86400);
    let target = day_start + (hour as i64) * 3600 + (minute as i64) * 60;
    let target_utc = target - TZ_OFFSET;
    if target_utc as u64 <= now {
        (target_utc + 86400) as u64
    } else {
        target_utc as u64
    }
}

// ===== Telegram Types =====

#[derive(Debug, Deserialize)]
struct TgResponse {
    ok: bool,
    result: Option<Vec<TgUpdate>>,
}

#[derive(Debug, Deserialize)]
struct TgUpdate {
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Debug, Deserialize)]
struct TgMessage {
    chat: TgChat,
    text: Option<String>,
    caption: Option<String>,
    voice: Option<TgVoice>,
    audio: Option<TgAudio>,
    #[serde(default)]
    photo: Vec<TgPhotoSize>,
    document: Option<TgDocument>,
}

#[derive(Debug, Deserialize)]
struct TgChat {
    id: i64,
}

fn ensure_telegram_api_ok(body: &str) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(body)?;
    if value.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(());
    }
    let desc = value
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown telegram error");
    bail!("Telegram API error: {}", desc)
}

#[derive(Debug, Deserialize)]
struct TgVoice {
    file_id: String,
    #[allow(dead_code)]
    duration: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TgAudio {
    file_id: String,
    mime_type: Option<String>,
    #[allow(dead_code)]
    duration: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TgPhotoSize {
    file_id: String,
    #[allow(dead_code)]
    width: Option<u32>,
    #[allow(dead_code)]
    height: Option<u32>,
    #[allow(dead_code)]
    file_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TgDocument {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TgFileResponse {
    ok: bool,
    result: Option<TgFile>,
}

#[derive(Debug, Deserialize)]
struct TgFile {
    file_path: Option<String>,
}

// ===== Gemini Types =====

#[derive(Serialize)]
struct GeminiRequest {
    #[serde(rename = "systemInstruction")]
    system_instruction: GeminiSysInstr,
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenConfig,
}

#[derive(Serialize)]
struct GeminiSysInstr {
    parts: Vec<GeminiTextPart>,
}

#[derive(Serialize)]
struct GenConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPartOut>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GeminiPartOut {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GeminiTextPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u64>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiRespContent>,
}

#[derive(Debug, Deserialize)]
struct GeminiRespContent {
    parts: Option<Vec<GeminiRespPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiRespPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TtsProxyJsonResponse {
    ok: bool,
    audio_b64: Option<String>,
    error: Option<String>,
}

// ===== NVS Helpers =====

fn nvs_get(nvs: &EspNvs<NvsDefault>, key: &str) -> Option<String> {
    let mut buf = [0u8; 256];
    nvs.get_str(key, &mut buf)
        .ok()
        .flatten()
        .map(|s| s.trim_end_matches('\0').to_string())
}

fn nvs_get_bool(nvs: &EspNvs<NvsDefault>, key: &str) -> Option<bool> {
    nvs_get(nvs, key).and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    })
}

fn nvs_get_i64(nvs: &EspNvs<NvsDefault>, key: &str) -> Option<i64> {
    nvs_get(nvs, key).and_then(|value| value.trim().parse::<i64>().ok())
}

fn load_last_telegram_update_id(nvs: &EspNvs<NvsDefault>) -> i64 {
    nvs_get_i64(nvs, TG_LAST_UPDATE_ID_KEY).unwrap_or(0)
}

fn save_last_telegram_update_id(nvs: &mut EspNvs<NvsDefault>, update_id: i64) {
    if let Err(err) = nvs.set_str(TG_LAST_UPDATE_ID_KEY, &update_id.to_string()) {
        warn!("Persist telegram update id failed: {}", err);
    }
}

fn load_tts_proxy_url(nvs: &EspNvs<NvsDefault>) -> Option<String> {
    nvs_get(nvs, TTS_PROXY_URL_KEY)
}

fn load_tts_proxy_voice(nvs: &EspNvs<NvsDefault>) -> String {
    nvs_get(nvs, TTS_PROXY_VOICE_KEY)
        .unwrap_or_else(|| "zh-TW-HsiaoChenNeural".to_string())
}

/// Read one line from UART0 using low-level esp-idf uart_read_bytes.
/// Returns the line (without newline) or empty string on timeout.
fn uart0_read_line(timeout_ms: u32) -> String {
    let mut buf = Vec::with_capacity(512);
    let byte_timeout = if timeout_ms > 200 { 200 } else { timeout_ms };
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms as u64);
    loop {
        if std::time::Instant::now() > deadline {
            break;
        }
        let mut byte = [0u8; 1];
        let n = unsafe {
            esp_idf_svc::sys::uart_read_bytes(
                0, // UART0
                byte.as_mut_ptr() as *mut _,
                1,
                byte_timeout / 10, // ticks (portTICK_PERIOD_MS = 10ms on default)
            )
        };
        if n > 0 {
            if byte[0] == b'\n' || byte[0] == b'\r' {
                if !buf.is_empty() {
                    break;
                }
                // skip leading CR/LF
                continue;
            }
            buf.push(byte[0]);
            if buf.len() >= 4096 {
                break; // safety limit
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

/// Wait for JSON provisioning via USB Serial when NVS is empty (first boot).
/// Expected format: {"WIFI_SSID":"...","TG_TOKEN":"...","GEMINI_KEY":"...","CHAT_ID":"...",...}
fn serial_provision(nvs: &mut EspNvs<NvsDefault>) -> Result<()> {
    warn!("NVS config incomplete — entering Serial provisioning mode");
    warn!("Send JSON config via USB Serial (or run provision.ps1)");

    // Install UART0 driver so uart_read_bytes works
    let uart_installed = unsafe {
        esp_idf_svc::sys::uart_driver_install(
            0,    // UART0
            1024, // RX buffer
            0,    // TX buffer (not needed — println! uses VFS)
            0,    // queue size
            std::ptr::null_mut(), // queue handle
            0,    // intr alloc flags
        )
    };
    if uart_installed != 0 {
        warn!("uart_driver_install returned {} (may already be installed)", uart_installed);
    }

    // Print a machine-readable marker so provision.ps1 can detect readiness
    println!("{{\"provision\":\"ready\"}}");

    let deadline = std::time::Instant::now() + Duration::from_secs(300); // 5 min timeout

    loop {
        if std::time::Instant::now() > deadline {
            bail!("Provisioning timeout — no config received in 5 minutes");
        }
        let line = uart0_read_line(2000);
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') {
            continue;
        }
        let obj: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                warn!("Invalid JSON: {}", e);
                println!("{{\"provision\":\"error\",\"msg\":\"invalid JSON\"}}");
                continue;
            }
        };
        // Store all recognised keys into NVS
        let keys = [
            ("WIFI_SSID", "wifi_ssid"),
            ("WIFI_PASS", "wifi_pass"),
            ("WIFI_SSID2", "wifi_ssid2"),
            ("WIFI_PASS2", "wifi_pass2"),
            ("WIFI_SSID3", "wifi_ssid3"),
            ("WIFI_PASS3", "wifi_pass3"),
            ("WIFI_SSID4", "wifi_ssid4"),
            ("WIFI_PASS4", "wifi_pass4"),
            ("WIFI_SSID5", "wifi_ssid5"),
            ("WIFI_PASS5", "wifi_pass5"),
            ("TG_TOKEN", "tg_token"),
            ("GEMINI_KEY", "gemini_key"),
            ("CHAT_ID", "chat_id"),
            ("TTS_PROXY_URL", TTS_PROXY_URL_KEY),
            ("TTS_PROXY_VOICE", TTS_PROXY_VOICE_KEY),
        ];
        let mut stored = 0u32;
        for (json_key, nvs_key) in keys {
            if let Some(val) = obj.get(json_key).and_then(|v| v.as_str()) {
                if !val.is_empty() {
                    let _ = nvs.set_str(nvs_key, val);
                    stored += 1;
                }
            }
        }
        // Validate required keys are now present
        let has_wifi = nvs_get(nvs, "wifi_ssid").is_some();
        let has_tg   = nvs_get(nvs, "tg_token").is_some();
        let has_gem  = nvs_get(nvs, "gemini_key").is_some();
        let has_chat = nvs_get(nvs, "chat_id").is_some();
        if has_wifi && has_tg && has_gem && has_chat {
            info!("Provisioning complete — {} keys stored", stored);
            println!("{{\"provision\":\"ok\",\"stored\":{}}}", stored);
            return Ok(());
        }
        let missing: Vec<&str> = [
            (!has_wifi, "WIFI_SSID"),
            (!has_tg,   "TG_TOKEN"),
            (!has_gem,  "GEMINI_KEY"),
            (!has_chat, "CHAT_ID"),
        ].iter().filter(|(m, _)| *m).map(|(_, n)| *n).collect();
        warn!("Provisioning incomplete, still missing: {:?}", missing);
        println!("{{\"provision\":\"incomplete\",\"missing\":{:?}}}", missing);
    }
}

fn load_config(nvs: &mut EspNvs<NvsDefault>) -> Result<Config> {
    // Check if core secrets exist in NVS; if not, enter serial provisioning
    let needs_provision = nvs_get(nvs, "wifi_ssid").is_none()
        || nvs_get(nvs, "tg_token").is_none()
        || nvs_get(nvs, "gemini_key").is_none()
        || nvs_get(nvs, "chat_id").is_none();

    if needs_provision {
        serial_provision(nvs)?;
    }

    let wifi_ssid = nvs_get(nvs, "wifi_ssid")
        .ok_or_else(|| anyhow::anyhow!("WIFI_SSID not set"))?;

    let wifi_pass = nvs_get(nvs, "wifi_pass")
        .unwrap_or_default();

    // Load up to 5 WiFi networks
    let mut wifi = vec![(wifi_ssid, wifi_pass)];
    for i in 2..=5u8 {
        let sk = format!("wifi_ssid{}", i);
        let pk = format!("wifi_pass{}", i);
        let ssid = nvs_get(nvs, &sk).unwrap_or_default();
        let pass = nvs_get(nvs, &pk).unwrap_or_default();
        wifi.push((ssid, pass));
    }

    let tg_token = nvs_get(nvs, "tg_token")
        .ok_or_else(|| anyhow::anyhow!("TG_TOKEN not set"))?;

    let gemini_key = nvs_get(nvs, "gemini_key")
        .ok_or_else(|| anyhow::anyhow!("GEMINI_KEY not set"))?;

    let chat_id = nvs_get(nvs, "chat_id")
        .ok_or_else(|| anyhow::anyhow!("CHAT_ID not set"))?;

    Ok(Config { wifi, tg_token, gemini_key, chat_id })
}

// ===== Main =====

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("=== {} ESP32-S3 AI Assistant v{} ===", BOT_NAME, FW_VERSION);

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs_part = EspDefaultNvsPartition::take()?;
    let mut nvs = EspNvs::new(nvs_part.clone(), NVS_NS, true)?;

    let cfg = load_config(&mut nvs)?;
    info!("Config loaded (WiFi: {})", cfg.wifi[0].0);

    // WiFi - 多網路自動切換(最多 5 組，無限輪播直到連上)
    let mut wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs_part))?;
    let wifi_nets: Vec<&(String, String)> = cfg.wifi.iter().filter(|(s, _)| !s.is_empty()).collect();
    let mut wifi_retry_round = 0u32;
    'wifi_retry: loop {
        if wifi_retry_round > 0 {
            let backoff = core::cmp::min(5 * wifi_retry_round, 30);
            warn!("WiFi: 第 {} 輪全部失敗，{}秒後重試...", wifi_retry_round, backoff);
            for _ in 0..backoff {
                std::thread::sleep(Duration::from_secs(1));
                feed_watchdog();
            }
        }
        wifi_retry_round += 1;
        for (idx, (ssid, pass)) in wifi_nets.iter().enumerate() {
            info!("WiFi[{}/{}] 第{}輪: 嘗試連接 {}...", idx + 1, wifi_nets.len(), wifi_retry_round, ssid);
            if wifi.is_started().unwrap_or(false) { let _ = wifi.stop(); }
            let ssid_hs = match ssid.as_str().try_into() {
                Ok(s) => s,
                Err(_) => { warn!("SSID {} 太長，跳過", ssid); continue; }
            };
            let pass_hs = match pass.as_str().try_into() {
                Ok(p) => p,
                Err(_) => { warn!("PASS 太長，跳過"); continue; }
            };
            if let Err(e) = wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid_hs,
                password: pass_hs,
                ..Default::default()
            })) { warn!("set_configuration 失敗 {}: {:?}", ssid, e); continue; }
            let _ = wifi.start();
            let _ = wifi.connect();
            for _ in 0..20u32 {
                std::thread::sleep(Duration::from_secs(1));
                feed_watchdog();
                if wifi.is_connected().unwrap_or(false) {
                    if let Ok(ip) = wifi.sta_netif().get_ip_info() {
                        if !ip.ip.is_unspecified() {
                            info!("WiFi 已連接！SSID={} IP={} (第{}輪)", ssid, ip.ip, wifi_retry_round);
                            let _ = nvs.set_str("wifi_cur_ip", &ip.ip.to_string());
                            break 'wifi_retry;
                        }
                    }
                }
            }
            warn!("WiFi {} 連線超時，改試下一個網路..", ssid);
            let _ = wifi.disconnect();
        }
    }

    // NTP time sync
    info!("Syncing NTP...");
    let _sntp = esp_idf_svc::sntp::EspSntp::new_default();
    if let Err(e) = &_sntp {
        warn!("SNTP init failed: {:?}, time may be inaccurate", e);
    }
    let mut ntp_wait = 0;
    loop {
        let now = now_epoch();
        if now > 1700000000 {
            info!("NTP synced! Epoch: {}", now);
            break;
        }
        ntp_wait += 1;
        if ntp_wait > 15 {
            warn!("NTP sync timeout, continuing without sync");
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    let now = now_epoch();
    let (h, m) = epoch_to_hhmm(now);
    info!("Local time: {:02}:{:02} (UTC+8)", h, m);

    let mut state = AppState::new(now);

    // Load persistent memories from NVS
    load_memories_from_nvs(&nvs, &mut state);
    info!("Loaded {} memories from NVS", state.memories.len());

    // Load skills from NVS
    state.skills = load_skills_from_nvs(&nvs);
    info!("Loaded {} skills from NVS", state.skills.len());

    if let Some(saved_model) = nvs_get(&nvs, "model_pref") {
        if let Some(valid_model) = normalize_model_selection(&saved_model) {
            state.current_model = valid_model.to_string();
        }
    }
    if let Some(saved_voice_mode) = nvs_get(&nvs, "voice_mode") {
        if let Some(valid_voice_mode) = normalize_voice_mode(&saved_voice_mode) {
            state.voice_mode = valid_voice_mode.to_string();
        }
    }
    if let Some(saved_wake_phrase) = nvs_get(&nvs, "wake_phrase") {
        state.wake_phrase = saved_wake_phrase;
    }
    if let Some(saved_wake_enabled) = nvs_get_bool(&nvs, "wake_enabled") {
        state.wake_enabled = saved_wake_enabled;
    }
    info!("Runtime prefs: model={}, voice_mode={}", state.current_model, state.voice_mode);

    if ensure_builtin_skills(&mut state.skills) {
        save_skills_to_nvs(&mut nvs, &state.skills);
        info!("Ensured {} builtin skills", state.skills.len());
    }

    // Mark OTA partition as valid (rollback protection)
    unsafe {
        let ota_err = esp_idf_svc::sys::esp_ota_mark_app_valid_cancel_rollback();
        if ota_err == 0 { info!("OTA: app marked valid"); }
    }

    // Load reminders and auto tasks from NVS (persisted across reboots)
    load_reminders_from_nvs(&nvs, &mut state, now);
    load_auto_tasks_from_nvs(&nvs, &mut state, now);

    // Default daily reminders ??only add if none were loaded from NVS
    if state.reminders.is_empty() {
        state.add_reminder(ReminderType::Daily, 0, 8, 0,
            "早安！記得查看今天的天氣狀況，也可以輸入 /briefing 取得每日報告！", now);
        state.add_reminder(ReminderType::Daily, 0, 12, 0,
            "午安！記得吃飯、喝水休息一下，我也可以幫你處理各種事情", now);
        state.add_reminder(ReminderType::Daily, 0, 18, 30,
            "晚安！今天辛苦了，可以幫你整理一下今天的紀錄，繼續加油！", now);
        save_reminders_to_nvs(&mut nvs, &state);
        info!("Default reminders set (08:00, 12:00, 18:30) and saved to NVS");
    } else {
        info!("Restored {} reminders from NVS, {} auto tasks", state.reminders.len(), state.auto_tasks.len());
    }

    let lcd_pins = load_lcd_pins(&nvs);
    info!("LCD config: {}", lcd_pins.status_text().replace('\n', " | "));
    let _ = show_boot_screen(&lcd_pins);
    let audio_pins = load_audio_pins(&nvs);
    info!("Audio config: {}", audio_pins.status_text().replace('\n', " | "));
    info!("Boot audio tone test skipped; use /audio tone for manual verification");
    match board_hw::capture_mic_snapshot(&audio_pins, 800) {
        Ok(snapshot) => info!(
            "Boot mic snapshot OK: rms={} peak={} samples={}",
            snapshot.rms, snapshot.peak, snapshot.samples
        ),
        Err(err) => warn!("Boot mic snapshot failed: {}", err),
    }
    #[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
    {
        let camera_pins = load_camera_pins(&nvs);
        match capture_camera_jpeg(&camera_pins) {
            Ok(jpeg) => info!("Boot camera test OK: {} KB", jpeg.len() / 1024),
            Err(err) => warn!("Boot camera test failed: {}", err),
        }
    }

    // BOOT button (GPIO0) ??Push-to-Talk
    std::thread::Builder::new()
        .name("btn".into())
        .stack_size(4096)
        .spawn(|| {
            unsafe {
                esp_idf_sys::gpio_reset_pin(0);
                esp_idf_sys::gpio_set_direction(0, esp_idf_sys::gpio_mode_t_GPIO_MODE_INPUT);
                esp_idf_sys::gpio_set_pull_mode(0, esp_idf_sys::gpio_pull_mode_t_GPIO_PULLUP_ONLY);
            }
            let mut prev = 1;
            loop {
                let level = unsafe { esp_idf_sys::gpio_get_level(0) };
                if prev == 1 && level == 0 {
                    BUTTON_PRESSED.store(true, Ordering::SeqCst);
                }
                prev = level;
                std::thread::sleep(Duration::from_millis(50));
            }
        })?;
    info!("BOOT button (GPIO0) Push-to-Talk ready");

    let mut last_id: i64 = match prepare_telegram_polling(&cfg.tg_token, &nvs) {
        Ok(id) => {
            info!("Telegram polling ready, last persisted update id={}", id);
            id
        }
        Err(err) => {
            warn!("Telegram polling prepare failed: {}", err);
            load_last_telegram_update_id(&nvs)
        }
    };

    // Boot notification
    let chat_id_num: i64 = cfg.chat_id.parse().unwrap_or(0);
    let heap_free = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };
    let bot_user = fetch_bot_username(&cfg.tg_token).unwrap_or_else(|_| "unknown".to_string());

    // ESP-SR WakeNet init (local wake word detection)
    let sr_status = match init_wakenet() {
        Ok(word) => format!("ESP-SR: \u{2705}{}", word),
        Err(e) => {
            warn!("ESP-SR init failed: {}", e);
            format!("ESP-SR: \u{274c}{}", e)
        }
    };

    let boot_msg = format!(
        "\u{1f916} ETHAN v{} online!\n\
         Bot: @{}\n\
         Model: {}\n\
         Voice: {}\n\
         Wake: {} ({})\n\
         {}\n\
         Time: {:02}:{:02}\n\
         Free RAM: {}KB\n\
         Memories: {}\n\
         Skills: {}\n\
         PC Driver: USB Ready\n\
         OTA: Ready\n\
         TTS Cache: {}\n\
         /help for commands",
        FW_VERSION,
        bot_user,
        state.current_model,
        state.voice_mode,
        if state.wake_enabled { "on" } else { "off" },
        state.wake_phrase,
        sr_status,
        h, m,
        heap_free / 1024,
        state.memories.len(), state.skills.len(),
        TTS_CACHE_MAX,
    );
    match send_telegram(&cfg.tg_token, chat_id_num, &boot_msg) {
        Ok(()) => info!("Boot notification sent!"),
        Err(e) => error!("Boot notify failed: {:?}", e),
    }
    let _ = show_ready_screen(&lcd_pins);

    // Main loop
    info!("=== {} Telegram Bot ready ===", BOT_NAME);
    let mut consecutive_errors: u32 = 0;
    let mut last_reminder_check: u64 = now;
    let mut last_wifi_health_check: u64 = now;
    let serial_cmd_rx = spawn_serial_command_reader();

    // Send handshake to PC driver so it knows we're alive (token no longer shared for security)
    usb_send_pc_cmd("handshake", &serde_json::json!({"fw": FW_VERSION}), chat_id_num);

    loop {
        while let Ok(serial_cmd) = serial_cmd_rx.try_recv() {
            // Filter PC driver JSON responses (start with '{' and contain "id")
            if serial_cmd.starts_with('{') {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&serial_cmd) {
                    if obj.get("id").is_some() || obj.get("ok").is_some() {
                        DRIVER_CONNECTED.store(true, Ordering::Relaxed);
                        info!("PC driver response: {}", serial_cmd);
                        continue;
                    }
                }
            }
            // Driver requests re-handshake after connecting
            if serial_cmd.trim() == "/driver_ping" {
                info!("Driver ping received, re-sending handshake");
                usb_send_pc_cmd("handshake", &serde_json::json!({"fw": FW_VERSION}), chat_id_num);
                continue;
            }
            info!("Serial input ({} chars)", serial_cmd.len());
            handle_text(&cfg, &mut nvs, &mut state, chat_id_num, &serial_cmd);
        }

        // Push-to-Talk: BOOT button (GPIO0)
        if BUTTON_PRESSED.swap(false, Ordering::SeqCst) {
            info!("PTT button pressed! Recording 5 seconds...");
            let _ = send_telegram(&cfg.tg_token, chat_id_num, "\u{1f3a4} PTT: \u{6b63}\u{5728}\u{807d}...");
            let _ = show_reply_screen(&lcd_pins, "\u{1f3a4} Listening...");
            std::thread::sleep(Duration::from_millis(300));
            match capture_and_transcribe_mic(&cfg, &nvs, &mut state, 5000) {
                Ok(transcript) if !transcript.trim().is_empty() => {
                    info!("PTT transcribed ({} chars)", transcript.len());
                    let _ = send_telegram(
                        &cfg.tg_token, chat_id_num,
                        &format!("\u{1f3a4} PTT: {}", transcript),
                    );
                    handle_text(&cfg, &mut nvs, &mut state, chat_id_num, &transcript);
                }
                Ok(_) => {
                    info!("PTT: silence");
                    let _ = show_ready_screen(&lcd_pins);
                }
                Err(err) => {
                    warn!("PTT transcribe failed: {}", err);
                    let _ = show_reply_screen(&lcd_pins, "PTT error");
                }
            }
        }

        // Check reminders every 30s
        let now = now_epoch();
        let (hour, minute) = epoch_to_hhmm(now);
        let today = local_day_number(now);
        if hour == 8 && minute < 5 && state.last_briefing_day != today {
            state.last_briefing_day = today;
            let briefing = build_daily_briefing(&state, now);
            if let Err(e) = send_telegram(&cfg.tg_token, chat_id_num, &briefing) {
                error!("Morning briefing send failed: {:?}", e);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        if now - last_reminder_check >= 30 {
            last_reminder_check = now;
            let fired = state.check_reminders(now);
            if !fired.is_empty() {
                save_reminders_to_nvs(&mut nvs, &state); // persist Once reminders becoming inactive
            }
            for msg in fired {
                if let Err(e) = send_telegram(&cfg.tg_token, chat_id_num, &msg) {
                    error!("Reminder send failed: {:?}", e);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }

        // Execute auto tasks (skip when paused)
        if !PAUSED.load(Ordering::Relaxed) {
            let task_ids: Vec<u32> = state.auto_tasks.iter()
                .filter(|t| t.active && now >= t.next_trigger)
                .map(|t| t.id)
                .collect();
            for tid in task_ids {
                if PAUSED.load(Ordering::Relaxed) { break; }
                if let Some(idx) = state.auto_tasks.iter().position(|t| t.id == tid) {
                    state.auto_tasks[idx].next_trigger = now + state.auto_tasks[idx].interval_secs;
                    let task = state.auto_tasks[idx].clone();
                    info!("AutoTask #{} firing: action={} prompt={}", task.id, task.action, task.prompt);
                    match task.action.as_str() {
                        "camera" => {
                            #[cfg(esp_idf_comp_espressif__esp32_camera_enabled)]
                            {
                                let camera_pins = load_camera_pins(&nvs);
                                match capture_camera_jpeg(&camera_pins) {
                                    Ok(jpeg) => {
                                        match describe_image_with_gemini(&cfg.gemini_key, &mut state, &jpeg, &task.prompt) {
                                            Ok(result) => {
                                                let msg = format!("\u{1f916} AutoTask#{}: {}", task.id, result);
                                                let _ = send_telegram(&cfg.tg_token, chat_id_num, &msg);
                                                if task.speak && should_speak_onboard(&state.voice_mode) {
                                                    let speech = speech_text_for_mode(&state.voice_mode, &result);
                                                    if let Err(e) = speak_onboard_text(&nvs, &speech) {
                                                        warn!("AutoTask#{} TTS: {:?}", task.id, e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("AutoTask#{} vision failed: {:?}", task.id, e);
                                            }
                                        }
                                    }
                                    Err(e) => warn!("AutoTask#{} camera failed: {:?}", task.id, e),
                                }
                            }
                            #[cfg(not(esp_idf_comp_espressif__esp32_camera_enabled))]
                            {
                                warn!("AutoTask#{}: camera not enabled in firmware", task.id);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if state.wake_enabled && !PAUSED.load(Ordering::Relaxed) && now.saturating_sub(state.last_wake_check) >= WAKE_CHECK_INTERVAL_SECS {
            state.last_wake_check = now;
            match detect_wake_phrase(&cfg, &nvs, &mut state, 2200) {
                Ok(Some(transcript)) => {
                    if now.saturating_sub(state.last_wake_trigger) >= WAKE_TRIGGER_COOLDOWN_SECS {
                        state.last_wake_trigger = now;
                        info!("Wake phrase detected: {}", transcript);
                        let _ = send_telegram(
                            &cfg.tg_token,
                            chat_id_num,
                            &format!("\u{1f3a4} Wake: {}", transcript),
                        );
                        let _ = show_reply_screen(&lcd_pins, "Wake detected");
                        if should_speak_onboard(&state.voice_mode) {
                            if let Err(err) = speak_onboard_text(&nvs, "\u{6211}\u{5728}\u{ff0c}\u{8acb}\u{8aaa}\u{3002}") {
                                warn!("Wake TTS failed: {}", err);
                            }
                        }
                        // Record follow-up command after wake
                        std::thread::sleep(Duration::from_millis(500));
                        let _ = show_reply_screen(&lcd_pins, "\u{1f3a4} Listening...");
                        match capture_and_transcribe_mic(&cfg, &nvs, &mut state, 4000) {
                            Ok(cmd) if !cmd.trim().is_empty() => {
                                info!("Wake cmd: {}", cmd);
                                let _ = send_telegram(
                                    &cfg.tg_token, chat_id_num,
                                    &format!("\u{1f3a4} {}", cmd),
                                );
                                handle_text(&cfg, &mut nvs, &mut state, chat_id_num, &cmd);
                            }
                            Ok(_) => info!("Wake follow-up: silence"),
                            Err(err) => warn!("Wake follow-up failed: {}", err),
                        }
                    }
                }
                Ok(None) => {}
                Err(err) => warn!("Wake listener failed: {}", err),
            }
        }

        // Proactive WiFi health check (every WIFI_HEALTH_CHECK_SECS)
        // Catches silent drops before Telegram polling fails
        let now = now_epoch();
        if now.saturating_sub(last_wifi_health_check) >= WIFI_HEALTH_CHECK_SECS {
            last_wifi_health_check = now;
            if !wifi.is_connected().unwrap_or(false) {
                warn!("WiFi health check: 連線已斷，開始輪播重連..");
                let recon_nets: Vec<&(String, String)> = cfg.wifi.iter().filter(|(s, _)| !s.is_empty()).collect();
                let mut recon_round = 0u32;
                'health_recon: loop {
                    if recon_round > 0 {
                        let backoff = core::cmp::min(5 * recon_round, 30);
                        warn!("Health reconnect: 第{}輪全部失敗，{}秒後重試...", recon_round, backoff);
                        for _ in 0..backoff {
                            std::thread::sleep(Duration::from_secs(1));
                            feed_watchdog();
                        }
                    }
                    recon_round += 1;
                    for (ssid, pass) in recon_nets.iter() {
                        info!("Health reconnect[第{}輪]: 嘗試 {}...", recon_round, ssid);
                        if wifi.is_started().unwrap_or(false) { let _ = wifi.stop(); }
                        let _ = wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                            ssid: ssid.as_str().try_into().unwrap_or_default(),
                            password: pass.as_str().try_into().unwrap_or_default(),
                            ..Default::default()
                        }));
                        let _ = wifi.start();
                        let _ = wifi.connect();
                        for _ in 0..20u32 {
                            std::thread::sleep(Duration::from_secs(1));
                            feed_watchdog();
                            if wifi.is_connected().unwrap_or(false) {
                                if let Ok(ip_info) = wifi.sta_netif().get_ip_info() {
                                    if !ip_info.ip.is_unspecified() {
                                        info!("Health reconnect 成功: {} ({}) [第{}輪]", ssid, ip_info.ip, recon_round);
                                        consecutive_errors = 0;
                                        let _ = send_telegram(&cfg.tg_token, chat_id_num,
                                            &format!("\u{1f4f6} WiFi 已恢復: {} ({}) [重試{}輪]", ssid, ip_info.ip, recon_round));
                                        break 'health_recon;
                                    }
                                }
                            }
                        }
                        warn!("Health reconnect {} 失敗", ssid);
                        let _ = wifi.disconnect();
                    }
                }
            }
        }

        // Poll Telegram
        match poll_telegram(&cfg.tg_token, last_id + 1) {
            Ok(updates) => {
                consecutive_errors = 0;
                for update in updates {
                    last_id = update.update_id;
                    save_last_telegram_update_id(&mut nvs, last_id);
                    if let Some(msg) = update.message {
                        let cid = msg.chat.id;
                        if cid.to_string() != cfg.chat_id {
                            warn!("Unauthorized: {}", cid);
                            continue;
                        }

                        if let Some(voice) = msg.voice {
                            info!("Voice msg (file_id: {})", voice.file_id);
                            handle_voice(&cfg, &mut nvs, &mut state, cid, &voice.file_id);
                            continue;
                        }

                        if let Some(audio) = msg.audio {
                            let mime = audio.mime_type.as_deref().unwrap_or("audio/mpeg");
                            info!("Audio msg (file_id: {}, mime: {})", audio.file_id, mime);
                            handle_audio_upload(&cfg, &mut nvs, &mut state, cid, &audio.file_id, mime);
                            continue;
                        }

                        if let Some(photo) = msg.photo.last() {
                            info!("Photo msg (file_id: {})", photo.file_id);
                            handle_telegram_photo_upload(
                                &cfg,
                                &mut nvs,
                                &mut state,
                                cid,
                                &photo.file_id,
                                "image/jpeg",
                                msg.caption.as_deref(),
                            );
                            continue;
                        }

                        if let Some(doc) = msg.document {
                            let fname = doc.file_name.as_deref().unwrap_or("unknown");
                            info!("Document: {} ({})", fname, doc.file_id);
                            if fname.ends_with(".bin") {
                                handle_ota_document(&cfg, cid, &doc.file_id, fname);
                            } else if let Some(mime) = infer_media_mime(doc.file_name.as_deref(), doc.mime_type.as_deref()) {
                                if mime.starts_with("image/") {
                                    handle_telegram_photo_upload(
                                        &cfg,
                                        &mut nvs,
                                        &mut state,
                                        cid,
                                        &doc.file_id,
                                        &mime,
                                        msg.caption.as_deref(),
                                    );
                                } else if mime.starts_with("audio/") {
                                    handle_audio_upload(
                                        &cfg,
                                        &mut nvs,
                                        &mut state,
                                        cid,
                                        &doc.file_id,
                                        &mime,
                                    );
                                } else {
                                    let _ = send_telegram(&cfg.tg_token, cid,
                                        &format!("Received: {}\nUnsupported media mime type: {}", fname, mime));
                                }
                            } else {
                                let _ = send_telegram(&cfg.tg_token, cid,
                                    &format!("Received: {}\nOnly .bin OTA, image, and audio files are supported.", fname));
                            }
                            continue;
                        }

                        if let Some(text) = msg.text {
                            info!("Text msg received ({} chars)", text.len());
                            handle_text(&cfg, &mut nvs, &mut state, cid, &text);
                        }
                    }
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                let err_msg = format!("{:?}", e);
                state.diag.log("ERR", &format!("Poll fail #{}: {}", consecutive_errors, &err_msg[..err_msg.len().min(100)]));
                if consecutive_errors <= 3 {
                    error!("Poll error ({}): {:?}", consecutive_errors, e);
                }
                // DNS recovery: if we get repeated errors, try to reconnect
                // BUG FIX: use >= (not ==) so reconnect fires every DNS_RETRY_MAX errors
                if consecutive_errors >= DNS_RETRY_MAX && consecutive_errors % DNS_RETRY_MAX == 0 {
                    warn!("連線異常：{} 次連續錯誤，檢查 WiFi 狀態", consecutive_errors);
                    state.diag.log("WARN", "WiFi reconnect check triggered");
                    if !wifi.is_connected().unwrap_or(false) {
                        warn!("WiFi 已斷線，嘗試重新連接...");
                        'recon: for (ssid, pass) in cfg.wifi.iter().filter(|(s, _)| !s.is_empty()) {
                            info!("重連嘗試 {}...", ssid);
                            if wifi.is_started().unwrap_or(false) { let _ = wifi.stop(); }
                            let _ = wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                                ssid: ssid.as_str().try_into().unwrap_or_default(),
                                password: pass.as_str().try_into().unwrap_or_default(),
                                ..Default::default()
                            }));
                            let _ = wifi.start();
                            let _ = wifi.connect();
                            for _ in 0..20u32 {
                                std::thread::sleep(Duration::from_secs(1));
                                feed_watchdog();
                                if wifi.is_connected().unwrap_or(false) {
                                    if let Ok(ip_info) = wifi.sta_netif().get_ip_info() {
                                        if !ip_info.ip.is_unspecified() {
                                            info!("重連成功！SSID={} IP={}", ssid, ip_info.ip);
                                            consecutive_errors = 0;
                                            last_wifi_health_check = now_epoch();
                                            let _ = send_telegram(&cfg.tg_token, chat_id_num,
                                                &format!("\u{1f4f6} WiFi 已恢復連線: {} ({})", ssid, ip_info.ip));
                                            break 'recon;
                                        }
                                    }
                                }
                            }
                            warn!("重連 {} 失敗", ssid);
                        }
                    } else {
                        // Sleep in chunks to avoid WDT
                        for _ in 0..5 {
                            std::thread::sleep(Duration::from_millis(DNS_RETRY_DELAY_MS));
                            feed_watchdog();
                        }
                    }
                }
                let backoff = std::cmp::min(consecutive_errors * 5, 60);
                // Sleep in small chunks to avoid WDT timeout
                for _ in 0..backoff {
                    std::thread::sleep(Duration::from_secs(1));
                    feed_watchdog();
                }
            }
        }

        std::thread::sleep(Duration::from_millis(200));
    }
}

// ===== Voice Handler =====

fn handle_voice(cfg: &Config, nvs: &mut EspNvs<NvsDefault>, state: &mut AppState, chat_id: i64, file_id: &str) {
    let _ = send_telegram(&cfg.tg_token, chat_id, "Transcribing voice...");

    let audio_data = match download_telegram_file(&cfg.tg_token, file_id) {
        Ok(d) => d,
        Err(e) => {
            error!("Download voice: {:?}", e);
            let _ = send_telegram(&cfg.tg_token, chat_id, "Voice download failed");
            return;
        }
    };
    info!("Voice downloaded: {} bytes", audio_data.len());

    let transcript = match transcribe_audio(&cfg.gemini_key, &audio_data, state) {
        Ok(t) => t,
        Err(e) => {
            error!("Transcribe: {:?}", e);
            let _ = send_telegram(&cfg.tg_token, chat_id, "Voice transcription failed");
            return;
        }
    };

    info!("Voice transcribed ({} chars)", transcript.len());

    let reply = match ask_gemini_with_context(&cfg.gemini_key, &transcript, state) {
        Ok(r) => r,
        Err(e) => {
            state.diag.log("ERR", &format!("Voice Gemini: {:?}", e));
            format!("AI Error: {}", e)
        }
    };

    let clean_reply = parse_all_ai_commands(&reply, state, nvs);
    let (clean_reply, _) = (clean_reply, false);
    let (clean_reply, control_notes) = parse_control_tags(&clean_reply, nvs, state);
    let (clean_reply, had_mem) = parse_memory_tags(&clean_reply, nvs, state);
    let (clean_reply, had_pc) = parse_pc_tags(&clean_reply, chat_id);
    let (clean_reply, had_py) = parse_py_tags(&clean_reply, chat_id);
    let (clean_reply, had_img) = parse_img_tags(&clean_reply, chat_id);
    let (clean_reply, had_excel) = parse_excel_tags(&clean_reply, chat_id);
    let (clean_reply, had_cam) = parse_camera_tags(&clean_reply, cfg, nvs, state, chat_id);

    let mut notes = Vec::new();
    for note in control_notes { notes.push(note); }
    if had_mem { notes.push("Memory updated".to_string()); }
    if had_pc { notes.push("PC cmd sent".to_string()); }
    if had_py { notes.push("Python sent".to_string()); }
    if had_img { notes.push("Image gen sent".to_string()); }
    if had_excel { notes.push("Excel sent".to_string()); }
    if had_cam { notes.push("\u{1f4f7} Camera".to_string()); }
    let note_str = if notes.is_empty() {
        String::new()
    } else {
        format!("\n[{}]", notes.join("] ["))
    };
    let user_reply = strip_face_tags(&clean_reply);
    let full_reply = format!(
        "\u{1f3a4} \"{}\"\n\n{}{}{}",
        transcript,
        user_reply,
        note_str,
        token_footer(state)
    );

    if let Err(e) = send_telegram(&cfg.tg_token, chat_id, &full_reply) {
        error!("Reply failed: {:?}", e);
    }

    let lcd_pins = load_lcd_pins(nvs);
    let display_reply = format!("{}{}{}", clean_reply, note_str, token_footer(state));
    let _ = show_reply_screen(&lcd_pins, &display_reply);
    if should_speak_onboard(&state.voice_mode) {
        let speech_text = speech_text_for_mode(&state.voice_mode, &user_reply);
        if let Err(err) = speak_onboard_text(nvs, &speech_text) {
            warn!("Onboard TTS failed: {}", err);
            state.diag.log("WARN", &format!("Voice TTS: {}", err));
        }
    }
}

fn handle_audio_upload(
    cfg: &Config,
    nvs: &mut EspNvs<NvsDefault>,
    state: &mut AppState,
    chat_id: i64,
    file_id: &str,
    mime_type: &str,
) {
    let _ = send_telegram(&cfg.tg_token, chat_id, "Transcribing audio...");

    let audio_data = match download_telegram_file(&cfg.tg_token, file_id) {
        Ok(d) => d,
        Err(e) => {
            error!("Download audio: {:?}", e);
            let _ = send_telegram(&cfg.tg_token, chat_id, "Audio download failed");
            return;
        }
    };
    info!("Audio downloaded: {} bytes ({})", audio_data.len(), mime_type);

    let transcript = match transcribe_audio_with_mime(&cfg.gemini_key, &audio_data, mime_type, state) {
        Ok(t) => t,
        Err(e) => {
            error!("Audio transcribe: {:?}", e);
            let _ = send_telegram(&cfg.tg_token, chat_id, "Audio transcription failed");
            return;
        }
    };

    info!("Audio transcribed ({} chars)", transcript.len());

    let reply = match ask_gemini_with_context(&cfg.gemini_key, &transcript, state) {
        Ok(r) => r,
        Err(e) => {
            state.diag.log("ERR", &format!("Audio Gemini: {:?}", e));
            format!("AI Error: {}", e)
        }
    };

    let clean_reply = parse_all_ai_commands(&reply, state, nvs);
    let (clean_reply, _) = (clean_reply, false);
    let (clean_reply, control_notes) = parse_control_tags(&clean_reply, nvs, state);
    let (clean_reply, had_mem) = parse_memory_tags(&clean_reply, nvs, state);
    let (clean_reply, had_pc) = parse_pc_tags(&clean_reply, chat_id);
    let (clean_reply, had_py) = parse_py_tags(&clean_reply, chat_id);
    let (clean_reply, had_img) = parse_img_tags(&clean_reply, chat_id);
    let (clean_reply, had_excel) = parse_excel_tags(&clean_reply, chat_id);
    let (clean_reply, had_cam) = parse_camera_tags(&clean_reply, cfg, nvs, state, chat_id);

    let mut notes = Vec::new();
    for note in control_notes { notes.push(note); }
    if had_mem { notes.push("Memory updated".to_string()); }
    if had_pc { notes.push("PC cmd sent".to_string()); }
    if had_py { notes.push("Python sent".to_string()); }
    if had_img { notes.push("Image gen sent".to_string()); }
    if had_excel { notes.push("Excel sent".to_string()); }
    if had_cam { notes.push("\u{1f4f7} Camera".to_string()); }
    let note_str = if notes.is_empty() {
        String::new()
    } else {
        format!("\n[{}]", notes.join("] ["))
    };
    let user_reply = strip_face_tags(&clean_reply);
    let full_reply = format!(
        "\u{1f3a7} \"{}\"

{}{}{}",
        transcript,
        user_reply,
        note_str,
        token_footer(state)
    );

    if let Err(e) = send_telegram(&cfg.tg_token, chat_id, &full_reply) {
        error!("Audio reply failed: {:?}", e);
    }

    let lcd_pins = load_lcd_pins(nvs);
    let display_reply = format!("{}{}{}", clean_reply, note_str, token_footer(state));
    let _ = show_reply_screen(&lcd_pins, &display_reply);
    if should_speak_onboard(&state.voice_mode) {
        let speech_text = speech_text_for_mode(&state.voice_mode, &user_reply);
        if let Err(err) = speak_onboard_text(nvs, &speech_text) {
            warn!("Audio TTS failed: {}", err);
            state.diag.log("WARN", &format!("Audio TTS: {}", err));
        }
    }
}

fn handle_telegram_photo_upload(
    cfg: &Config,
    nvs: &mut EspNvs<NvsDefault>,
    state: &mut AppState,
    chat_id: i64,
    file_id: &str,
    mime_type: &str,
    caption: Option<&str>,
) {
    let caption_text = caption.unwrap_or("").trim();
    let prompt = if caption_text.is_empty() {
        "請用繁體中文描述這張使用者傳送的照片，描述重點人物、物件、文字、場景，並給出建議。".to_string()
    } else {
        format!(
            "使用者傳送了一張照片。請以繁體中文回覆他想要知道的：{}",
            caption_text
        )
    };

    let _ = send_telegram(&cfg.tg_token, chat_id, "正在處理你傳來的照片..");

    let image_data = match download_telegram_file(&cfg.tg_token, file_id) {
        Ok(d) => d,
        Err(e) => {
            error!("Download photo: {:?}", e);
            let _ = send_telegram(&cfg.tg_token, chat_id, "圖片下載失敗");
            return;
        }
    };
    info!("Photo downloaded: {} bytes ({})", image_data.len(), mime_type);

    let reply = match ask_gemini_with_image_context(&cfg.gemini_key, &prompt, &image_data, mime_type, state) {
        Ok(r) => r,
        Err(e) => {
            state.diag.log("ERR", &format!("Photo Gemini: {:?}", e));
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("圖片處理失敗: {}", e));
            return;
        }
    };

    let clean_reply = parse_all_ai_commands(&reply, state, nvs);
    let (clean_reply, _) = (clean_reply, false);
    let (clean_reply, control_notes) = parse_control_tags(&clean_reply, nvs, state);
    let (clean_reply, had_mem) = parse_memory_tags(&clean_reply, nvs, state);
    let (clean_reply, had_pc) = parse_pc_tags(&clean_reply, chat_id);
    let (clean_reply, had_py) = parse_py_tags(&clean_reply, chat_id);
    let (clean_reply, had_img) = parse_img_tags(&clean_reply, chat_id);
    let (clean_reply, had_excel) = parse_excel_tags(&clean_reply, chat_id);
    let (clean_reply, had_cam) = parse_camera_tags(&clean_reply, cfg, nvs, state, chat_id);

    let mut notes = Vec::new();
    for note in control_notes { notes.push(note); }
    if had_mem { notes.push("Memory updated".to_string()); }
    if had_pc { notes.push("PC cmd sent".to_string()); }
    if had_py { notes.push("Python sent to PC".to_string()); }
    if had_img { notes.push("Image gen sent to PC".to_string()); }
    if had_excel { notes.push("Excel pipeline sent to PC".to_string()); }
    if had_cam { notes.push("\u{1f4f7} Camera".to_string()); }
    let note_str = if notes.is_empty() {
        String::new()
    } else {
        format!("\n[{}]", notes.join("] ["))
    };
    let final_reply = format!("📷 {}{}{}", strip_face_tags(&clean_reply), note_str, token_footer(state));

    if let Err(e) = send_telegram(&cfg.tg_token, chat_id, &final_reply) {
        error!("Photo reply failed: {:?}", e);
    }

    let lcd_pins = load_lcd_pins(nvs);
    let display_reply = format!("{}{}{}", clean_reply, note_str, token_footer(state));
    let _ = show_reply_screen(&lcd_pins, &display_reply);
    if should_speak_onboard(&state.voice_mode) {
        let speech_text = speech_text_for_mode(&state.voice_mode, &strip_face_tags(&clean_reply));
        if let Err(err) = speak_onboard_text(nvs, &speech_text) {
            warn!("Photo TTS failed: {}", err);
            state.diag.log("WARN", &format!("Photo TTS: {}", err));
        }
    }
}

// ===== Text Handler =====

fn handle_text(
    cfg: &Config,
    nvs: &mut EspNvs<NvsDefault>,
    state: &mut AppState,
    chat_id: i64,
    text: &str,
) {
    let trimmed = text.trim();
    // 指令不分大小寫：/TASKS = /tasks = /Tasks
    // 只對指令名稱（第一個空白前）做小寫，保留參數原始大小寫（WiFi 密碼等）
    let trimmed_lc = trimmed.to_ascii_lowercase();
    let trimmed_buf: String;
    let trimmed = if trimmed_lc.starts_with('/') {
        if let Some(sp) = trimmed.find(char::is_whitespace) {
            trimmed_buf = format!("{}{}", &trimmed_lc[..sp], &trimmed[sp..]);
            trimmed_buf.as_str()
        } else {
            trimmed_lc.as_str()
        }
    } else { trimmed };

    // ===== HIGHEST-PRIORITY: Emergency commands (always work, even when paused) =====
    match trimmed {
        "/pause" => {
            PAUSED.store(true, Ordering::Relaxed);
            let count = state.auto_tasks.len();
            state.auto_tasks.clear();
            save_auto_tasks_to_nvs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("⏸️ 系統已暫停！\n已清除 {} 個自動任務\n所有 AI 呼叫已停止。\n\n傳送 /resume 恢復", count));
            return;
        }
        "/resume" => {
            PAUSED.store(false, Ordering::Relaxed);
            GEMINI_CALL_COUNT.store(0, Ordering::Relaxed);
            let _ = send_telegram(&cfg.tg_token, chat_id, "▶️ 系統已恢復！AI 功能已啟用。");
            return;
        }
        "/shutdown" => {
            PAUSED.store(true, Ordering::Relaxed);
            let task_count = state.auto_tasks.len();
            state.auto_tasks.clear();
            save_auto_tasks_to_nvs(nvs, state);
            for r in &mut state.reminders { r.active = false; }
            save_reminders_to_nvs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("🔴 系統已關機！\n已清除 {} 個自動任務\n已停用所有提醒\n所有 AI 功能已停止。\n\n傳送 /resume 重新啟動", task_count));
            return;
        }
        _ => {}
    }

    // If globally paused, reject everything except the above emergency commands
    if PAUSED.load(Ordering::Relaxed) {
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "⏸️ 系統已暫停。\n傳送 /resume 恢復\n傳送 /shutdown 完全關機");
        return;
    }

    match trimmed {
        "/start" | "/help" => {
            let help = format!(
                "\u{1f916} ETHAN v{} | {}\n\n\
                 == System ==\n\
                 /status - System info\n\
                 /wifi - WiFi status\n\
                 /wifi set SSID pass (slot 1)\n\
                 /wifi set2~5 SSID pass\n\
                 /wifi del <1~5> / swap / clear\n\
                 /model - Model & capabilities\n\
                 /model set flash|pro|3.1-pro\n\
                 /voice - Voice reply mode\n\
                 /voice off|normal|brief\n\
                 /briefing - Morning/weather/news brief\n\
                 /email - ESP direct email tool\n\
                 /camera - OV3660 snapshot + Gemini Vision\n\
                 /lcd - ST7789 screen tools\n\
                 /audio - onboard speaker / mic tools\n\
                 /tokens - Token usage\n\
                 /token - Telegram Token 管理\n\
                 /diag - Self-diagnostics\n\n\
                 == Memory ==\n\
                 /memories - View memories\n\
                 /remember key value - Save\n\
                 /forget key - Delete\n\n\
                 == Schedule ==\n\
                 /reminders - List reminders\n\
                 /remind 5m msg - Every N min\n\
                 /remind 8:00 msg - Daily\n\
                 /remind del 3 - Delete #3\n\n\
                 == Skills ==\n\
                 /skills - List installed skills\n\
                 /skill add name|desc|trigger\n\
                 /skill del name\n\n\
                 == AutoTasks ==\n\
                 /tasks - List background tasks\n\
                 /tasks del ID - Stop a task\n\
                 /tasks clear - Clear all tasks\n\
                 /autotask - Natural language setup\n\
                 /stop - Emergency stop all + pause\n\
                 /pause - Pause all AI calls\n\
                 /resume - Resume AI calls\n\
                 /shutdown - Full shutdown\n\n\
                 == PC Control ==\n\
                 /pc <cmd> <args> - PC control\n\
                 /pc_unlock - Allow AI to send PC commands\n\
                 /pc_lock - Block AI PC commands (default)\n\
                 /screenshot - PC desktop screenshot\n\
                 /update - PC rebuild & flash\n\n\
                 == OTA ==\n\
                 /ota <url> - WiFi OTA update\n\
                 Send .bin file -> OTA update\n\
                 /reset - Factory reset\n\n\
                 Text/voice -> AI chat",
                FW_VERSION, state.current_model
            );
            let _ = send_telegram(&cfg.tg_token, chat_id, &help);
            return;
        }
        "/status" => {
            let heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };
            let now = now_epoch();
            let uptime = now - state.boot_time;
            let (h, m) = epoch_to_hhmm(now);
            let active = state.reminders.iter().filter(|r| r.active).count();
            let total_tok = state.tokens_in + state.tokens_out;
            let diag_count = state.diag.entries.len();
            let bot_user = fetch_bot_username(&cfg.tg_token).unwrap_or_else(|_| "unknown".into());
            let sr_active = WAKENET.lock().ok().map_or(false, |g| g.is_some());
            let tts_cached = TTS_CACHE.lock().ok().map_or(0, |c| c.len());
            let status = format!(
                "\u{1f916} ETHAN Status\n\
                 FW: v{} | Model: {}\n\
                 Bot: @{}\n\
                 Voice: {}\n\
                 Wake: {} ({}) | ESP-SR: {}\n\
                 Time: {:02}:{:02}\n\
                 Uptime: {}m\n\
                 RAM: {}KB free\n\
                 Reminders: {} active\n\
                 Memories: {}/{}\n\
                 Skills: {}\n\
                 TTS Cache: {}/{}\n\
                 Tokens: {} total ({} req)\n\
                 History: {} msgs\n\
                 Diag: {} entries",
                FW_VERSION,
                state.current_model,
                bot_user,
                state.voice_mode,
                if state.wake_enabled { "on" } else { "off" },
                state.wake_phrase,
                if sr_active { "active" } else { "off" },
                h, m,
                uptime / 60,
                heap / 1024, active, state.memories.len(), MAX_MEMORIES,
                state.skills.len(),
                tts_cached, TTS_CACHE_MAX,
                total_tok, state.requests,
                state.history.len(), diag_count
            );
            let _ = send_telegram(&cfg.tg_token, chat_id, &status);
            return;
        }
        "/wifi" => {
            handle_wifi_command(cfg, nvs, chat_id, "status");
            return;
        }
        "/model" => {
            let heap = unsafe { esp_idf_svc::sys::esp_get_free_heap_size() };
            let model_info = format!(
                "\u{1f9e0} ETHAN Model Info\n\n\
                 AI Model: {}\n\
                 Default: {}\n\
                 Firmware: v{}\n\
                 Hardware: ESP32-S3 N16R8\n\
                 Flash: 16MB | PSRAM: 8MB\n\
                 Free RAM: {}KB\n\n\
                 == Model Switch ==\n\
                 /model set flash\n\
                 /model set pro\n\
                 /model set 3.1-pro\n\
                 /model set 3.1-flash-lite\n\n\
                 == Supported ==\n\
                 {}\n\
                 == Capabilities ==\n\
                 \u{2705} AI Chat (Traditional Chinese)\n\
                 \u{2705} Voice Transcription\n\
                 \u{2705} PC Control (full access)\n\
                 \u{2705} Memory System ({}/{})\n\
                 \u{2705} Reminder/Scheduler\n\
                 \u{2705} OTA Self-Update\n\
                 \u{2705} Python Code Execution\n\
                 \u{2705} Image Generation\n\
                 \u{2705} Excel Processing\n\
                 \u{2705} Self-Diagnostics\n\
                 \u{2705} Skill System ({} skills)",
                state.current_model, DEFAULT_GEMINI_MODEL, FW_VERSION,
                heap / 1024,
                supported_models_text(),
                state.memories.len(), MAX_MEMORIES,
                state.skills.len()
            );
            let _ = send_telegram(&cfg.tg_token, chat_id, &model_info);
            return;
        }
        "/voice" => {
            let msg = format!(
                "Current voice reply mode: {}\n\nUse:\n/voice off\n/voice normal\n/voice brief\n\n`off` = 不播報\n`normal` = 語音完整播報\n`brief` = 語音簡句播報。",
                state.voice_mode
            );
            let _ = send_telegram(&cfg.tg_token, chat_id, &msg);
            return;
        }
        "/diag" => {
            let report = state.diag.format_report();
            let _ = send_telegram(&cfg.tg_token, chat_id, &report);
            return;
        }
        "/skills" => {
            if state.skills.is_empty() {
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    "No skills installed.\n/skill add name|description|trigger");
            } else {
                let mut s = format!("Skills ({}):\n", state.skills.len());
                for (i, sk) in state.skills.iter().enumerate() {
                    s.push_str(&format!("{}. {} [{}]\n   {}\n",
                        i+1, sk.name, sk.trigger, sk.description));
                }
                s.push_str("\n/skill add name|desc|trigger\n/skill del name");
                let _ = send_telegram(&cfg.tg_token, chat_id, &s);
            }
            return;
        }
        "/tokens" => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &state.token_report());
            return;
        }
        "/reminders" => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &state.list_reminders());
            return;
        }
        "/autotask" => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &list_auto_tasks(state));
            return;
        }
        "/reset" => {
            let _ = send_telegram(&cfg.tg_token, chat_id, "Resetting...");
            for k in &["wifi_ssid", "wifi_pass", "tg_token", "gemini_key", "chat_id", "model_pref", "voice_mode", "wake_phrase", "wake_enabled", "email_api_key", "email_from", "email_to"] {
                let _ = nvs.remove(k);
            }
            unsafe { esp_idf_svc::sys::esp_restart(); }
        }
        "/driver" => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "PC Driver Status:\n\
                 Protocol: USB Serial JSON\n\
                 Baud: 115200\n\
                 Commands sent: check driver.log\n\n\
                 Start driver on PC:\n\
                 python ethan_driver.py");
            return;
        }
        "/briefing" => {
            let greeting = build_daily_briefing(state, now_epoch());
            let _ = send_telegram(&cfg.tg_token, chat_id, &greeting);
            return;
        }
        "/update" => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "Triggering firmware rebuild...\n\
                 PC driver will build & flash.\n\
                 I'll restart in ~2 min.");
            usb_send_pc_cmd("firmware_update", &serde_json::json!({
                "firmware_dir": "C:\\zc",
                "port": "COM5"
            }), chat_id);
            return;
        }
        "/memories" | "/mem" => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &format_memories(&state.memories));
            return;
        }
        _ => {}
    }

    // Skill management commands
    if trimmed.starts_with("/skill ") {
        let arg = &trimmed[7..];
        if let Some(rest) = arg.strip_prefix("add ") {
            let parts: Vec<&str> = rest.splitn(3, '|').collect();
            if parts.len() >= 3 {
                if state.skills.len() >= MAX_SKILLS {
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("Max {} skills reached. Delete one first.", MAX_SKILLS));
                } else {
                    state.skills.push(Skill {
                        name: parts[0].trim().to_string(),
                        description: parts[1].trim().to_string(),
                        trigger: parts[2].trim().to_string(),
                    });
                    save_skills_to_nvs(nvs, &state.skills);
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("\u{2705} Skill added: {} (trigger: {})", parts[0].trim(), parts[2].trim()));
                }
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    "Usage: /skill add name|description|trigger");
            }
        } else if let Some(name) = arg.strip_prefix("del ") {
            let name = name.trim();
            let before = state.skills.len();
            state.skills.retain(|s| s.name != name);
            if state.skills.len() < before {
                save_skills_to_nvs(nvs, &state.skills);
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    &format!("\u{2705} Skill removed: {}", name));
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    &format!("Skill '{}' not found", name));
            }
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "Usage:\n/skill add name|description|trigger\n/skill del name");
        }
        return;
    }

    if trimmed.starts_with("/model ") {
        let arg = trimmed[7..].trim();
        let target = arg.strip_prefix("set ").unwrap_or(arg).trim();
        if let Some(model) = normalize_model_selection(target) {
            state.current_model = model.to_string();
            save_runtime_prefs(nvs, state);
            let _ = send_telegram(
                &cfg.tg_token,
                chat_id,
                &format!("??Model switched to {}", state.current_model),
            );
        } else {
            let _ = send_telegram(
                &cfg.tg_token,
                chat_id,
                &format!("Unknown model: {}\n\nSupported:\n{}", target, supported_models_text()),
            );
        }
        return;
    }

    if trimmed.starts_with("/voice ") {
        let arg = trimmed[7..].trim();
        if let Some(mode) = normalize_voice_mode(arg) {
            state.voice_mode = mode.to_string();
            save_runtime_prefs(nvs, state);
            let _ = send_telegram(
                &cfg.tg_token,
                chat_id,
                &format!("??Voice reply mode set to {}", state.voice_mode),
            );
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id, "Usage: /voice off\n/voice normal\n/voice brief");
        }
        return;
    }

    if trimmed.starts_with("/wifi") {
        let arg = trimmed.strip_prefix("/wifi").unwrap_or("").trim();
        handle_wifi_command(cfg, nvs, chat_id, arg);
        return;
    }

    if trimmed.starts_with("/token ") || trimmed == "/token" {
        let arg = trimmed.strip_prefix("/token").unwrap_or("").trim();
        handle_token_command(cfg, nvs, chat_id, arg);
        return;
    }

    if trimmed.starts_with("/email") {
        let arg = trimmed.strip_prefix("/email").unwrap_or("").trim();
        handle_direct_email(cfg, nvs, chat_id, arg);
        return;
    }

    if trimmed.starts_with("/camera") {
        let arg = trimmed.strip_prefix("/camera").unwrap_or("").trim();
        handle_camera_command(cfg, nvs, state, chat_id, arg);
        return;
    }

    if trimmed.starts_with("/lcd") {
        let arg = trimmed.strip_prefix("/lcd").unwrap_or("").trim();
        handle_lcd_command(cfg, nvs, chat_id, arg);
        return;
    }

    if trimmed.starts_with("/audio") {
        let arg = trimmed.strip_prefix("/audio").unwrap_or("").trim();
        handle_audio_command(cfg, nvs, state, chat_id, arg);
        return;
    }

    // /autotask and /tasks are aliases for the same task management
    if trimmed.starts_with("/autotask") || trimmed.starts_with("/tasks") {
        let arg = if trimmed.starts_with("/autotask") {
            trimmed.strip_prefix("/autotask").unwrap_or("").trim()
        } else {
            trimmed.strip_prefix("/tasks").unwrap_or("").trim()
        };
        if arg == "list" || arg.is_empty() {
            let _ = send_telegram(&cfg.tg_token, chat_id, &list_auto_tasks(state));
        } else if let Some(rest) = arg.strip_prefix("del ") {
            if let Ok(id) = rest.trim().parse::<u32>() {
                let before = state.auto_tasks.len();
                state.auto_tasks.retain(|t| t.id != id);
                if state.auto_tasks.len() < before {
                    save_auto_tasks_to_nvs(nvs, state);
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("✅ 任務 #{} 已刪除", id));
                } else {
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("找不到任務 #{}", id));
                }
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id, "格式：/tasks del <ID>");
            }
        } else if arg == "clear" || arg == "all" {
            let count = state.auto_tasks.len();
            state.auto_tasks.clear();
            save_auto_tasks_to_nvs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("🗑️ 已清空全部 {} 個自動任務", count));
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "📋 任務管理：\n/tasks — 列出目前任務\n/tasks del ID — 停止並刪除任務\n/tasks clear — 清除全部任務\n/autotask del ID — 刪除任務（同 /tasks del ID）");
        }
        return;
    }

    // /stop — 緊急停止所有自動任務 + 暫停系統
    if trimmed == "/stop" {
        PAUSED.store(true, Ordering::Relaxed);
        let count = state.auto_tasks.len();
        state.auto_tasks.clear();
        save_auto_tasks_to_nvs(nvs, state);
        let _ = send_telegram(&cfg.tg_token, chat_id,
            &format!("🛑 緊急停止！\n已清除 {} 個自動任務\n系統已暫停，所有 AI 呼叫已停止。\n\n傳送 /resume 恢復", count));
        return;
    }

    // /reminders — 完整提醒管理（支援 del / clear）
    if trimmed.starts_with("/reminders") {
        let arg = trimmed.strip_prefix("/reminders").unwrap_or("").trim();
        if arg.is_empty() || arg == "list" {
            let _ = send_telegram(&cfg.tg_token, chat_id, &state.list_reminders());
        } else if let Some(rest) = arg.strip_prefix("del ") {
            if let Ok(id) = rest.trim().parse::<u32>() {
                if state.remove_reminder(id) {
                    save_reminders_to_nvs(nvs, state);
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("✅ 提醒 #{} 已刪除", id));
                } else {
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("找不到提醒 #{}", id));
                }
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id, "格式：/reminders del <ID>");
            }
        } else if arg == "clear" || arg == "all" {
            let count = state.reminders.iter().filter(|r| r.active).count();
            for r in &mut state.reminders { r.active = false; }
            save_reminders_to_nvs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("🗑️ 已清空 {} 個提醒", count));
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "📋 提醒管理：\n/reminders — 列出目前提醒\n/reminders del ID — 刪除個別提醒\n/reminders clear — 清除全部提醒");
        }
        return;
    }

    // /sleepwatch on|off ??shortcut for sleep-monitoring autotask
    if trimmed.starts_with("/sleepwatch") {
        let arg = trimmed.strip_prefix("/sleepwatch").unwrap_or("").trim();
        if arg == "on" {
            let id = state.next_autotask_id;
            state.next_autotask_id += 1;
            let now = now_epoch();
            state.auto_tasks.push(AutoTask {
                id,
                interval_secs: 60,
                next_trigger: now + 60,
                action: "camera".to_string(),
                prompt: "請描述畫面中有誰在睡覺或打盹，用繁體中文簡短說明".to_string(),
                speak: true,
                active: true,
            });
            save_auto_tasks_to_nvs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("✅ 睡眠監視已啟動，任務 #{} 每60秒拍照語音播報\n\n停止指令：\n/sleepwatch off\n/autotask del {}", id, id));
        } else if arg == "off" {
            let before = state.auto_tasks.len();
            state.auto_tasks.retain(|t| !(t.interval_secs == 60 && t.action == "camera" && t.prompt.contains("睡覺")));
            let removed = before - state.auto_tasks.len();
            save_auto_tasks_to_nvs(nvs, state);
            if removed > 0 {
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    &format!("✅ 已停止 {} 個睡眠監視任務", removed));
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id, "沒有找到正在執行的任務（可能已停止）");
            }
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "💤 睡眠監視快速指令：\n/sleepwatch on  — 每分鐘拍照語音播報誰在睡覺\n/sleepwatch off — 停止所有睡眠監視任務");
        }
        return;
    }

    // PC Safe Mode toggle
    if trimmed == "/pc_unlock" {
        PC_SAFE_MODE.store(false, Ordering::Relaxed);
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "🔓 PC Safe Mode OFF — AI 可自動執行 PC 指令（shell, python, file_write, email 等）\n\n⚠️ 注意：AI 回覆中的 [PC:] [PY:] [IMG:] [EXCEL:] 標籤將會直接執行！\n\n傳送 /pc_lock 重新鎖定");
        return;
    }
    if trimmed == "/pc_lock" {
        PC_SAFE_MODE.store(true, Ordering::Relaxed);
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "🔒 PC Safe Mode ON — AI 只能執行安全的唯讀指令（screenshot, status, file_read 等）\n\n危險指令（shell, python, file_write, email）需手動用 /pc 執行");
        return;
    }

    if trimmed.starts_with("/pc ") {
        handle_pc_command(cfg, state, chat_id, &trimmed[4..]);
        return;
    }

    // /screenshot shortcut ??/pc screenshot
    if trimmed == "/screenshot" || trimmed == "/ss" {
        handle_pc_command(cfg, state, chat_id, "screenshot");
        return;
    }

    if trimmed.starts_with("/remember ") {
        let arg = &trimmed[10..];
        if let Some((key, val)) = arg.split_once(' ') {
            add_memory(nvs, state, key.trim(), val.trim());
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("\u{2705} Saved: {} = {}", key.trim(), val.trim()));
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "Usage: /remember <key> <value>\nEx: /remember coffee \u{9ed1}\u{5496}\u{5561}\u{4e0d}\u{52a0}\u{7cd6}");
        }
        return;
    }

    if trimmed.starts_with("/forget ") {
        let key = trimmed[8..].trim();
        if remove_memory(nvs, state, key) {
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("\u{2705} Forgot: {}", key));
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Memory '{}' not found", key));
        }
        return;
    }

    if trimmed.starts_with("/ota ") {
        let url = trimmed[5..].trim();
        handle_ota_url(cfg, chat_id, url);
        return;
    }

    // Quick-detect: Chinese screenshot request ??direct PC screenshot (skip Gemini roundtrip)
    {
        let lower = trimmed.to_lowercase();
        if (lower.contains("\u{622a}\u{5716}") || lower.contains("screenshot"))
            && (lower.contains("\u{96fb}\u{8166}") || lower.contains("\u{87a2}\u{5e55}") || lower.contains("pc") || lower.contains("desktop") || lower.contains("screen"))
        {
            let _ = send_telegram(&cfg.tg_token, chat_id, "\u{1f4f8} \u{6b63}\u{5728}\u{622a}\u{5716}\u{96fb}\u{8166}\u{756b}\u{9762}...");
            usb_send_pc_cmd("screenshot", &serde_json::json!({}), chat_id);
            return;
        }
    }

    // AI conversation
    let reply = match ask_gemini_with_context(&cfg.gemini_key, trimmed, state) {
        Ok(r) => r,
        Err(e) => {
            error!("Gemini error: {:?}", e);
            state.diag.log("ERR", &format!("Gemini: {:?}", e));
            format!("AI Error: {}", e)
        }
    };

    let clean_reply = parse_all_ai_commands(&reply, state, nvs);
    let (clean_reply, _) = (clean_reply, false);
    let (clean_reply, control_notes) = parse_control_tags(&clean_reply, nvs, state);
    let (clean_reply, had_mem) = parse_memory_tags(&clean_reply, nvs, state);
    let (clean_reply, had_pc) = parse_pc_tags(&clean_reply, chat_id);
    let (clean_reply, had_py) = parse_py_tags(&clean_reply, chat_id);
    let (clean_reply, had_img) = parse_img_tags(&clean_reply, chat_id);
    let (clean_reply, had_excel) = parse_excel_tags(&clean_reply, chat_id);
    let (clean_reply, had_cam) = parse_camera_tags(&clean_reply, cfg, nvs, state, chat_id);

    let mut notes = Vec::new();
    for note in control_notes { notes.push(note); }
    if had_mem { notes.push("Memory updated".to_string()); }
    if had_pc { notes.push("PC cmd sent".to_string()); }
    if had_py { notes.push("Python sent to PC".to_string()); }
    if had_img { notes.push("Image gen sent to PC".to_string()); }
    if had_excel { notes.push("Excel pipeline sent to PC".to_string()); }
    if had_cam { notes.push("\u{1f4f7} Camera".to_string()); }
    let note_str = if notes.is_empty() {
        String::new()
    } else {
        format!("\n[{}]", notes.join("] ["))
    };
    let final_reply = format!("{}{}{}", strip_face_tags(&clean_reply), note_str, token_footer(state));

    if let Err(e) = send_telegram(&cfg.tg_token, chat_id, &final_reply) {
        error!("Reply failed: {:?}", e);
    }

    let lcd_pins = load_lcd_pins(nvs);
    let display_reply = format!("{}{}{}", clean_reply, note_str, token_footer(state));
    let _ = show_reply_screen(&lcd_pins, &display_reply);
    if should_speak_onboard(&state.voice_mode) {
        let speech_text = speech_text_for_mode(&state.voice_mode, &strip_face_tags(&clean_reply));
        if let Err(err) = speak_onboard_text(nvs, &speech_text) {
            warn!("Onboard TTS failed: {}", err);
            state.diag.log("WARN", &format!("TTS: {}", err));
        }
    }
}

fn handle_lcd_command(cfg: &Config, nvs: &mut EspNvs<NvsDefault>, chat_id: i64, args: &str) {
    let trimmed = args.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        let pins = load_lcd_pins(nvs);
        let msg = format!(
            "ST7789 LCD\n\n{}\n\nCommands:\n/lcd default\n/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45 width=240 height=240\n/lcd test\n/lcd draw 你好 ETHAN",
            pins.status_text(),
        );
        let _ = send_telegram(&cfg.tg_token, chat_id, &msg);
        return;
    }

    if trimmed.eq_ignore_ascii_case("default") {
        let pins = board_hw::LcdPins::default();
        match save_lcd_pins(nvs, &pins) {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??LCD pins reset to default\n\n{}", pins.status_text()));
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("LCD default save failed: {}", err));
            }
        }
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("pins ") {
        let mut pins = load_lcd_pins(nvs);
        match parse_lcd_pin_args(rest, &mut pins).and_then(|_| save_lcd_pins(nvs, &pins)) {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??LCD pins updated\n\n{}", pins.status_text()));
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("LCD pin update failed: {}", err));
            }
        }
        return;
    }

    if trimmed.eq_ignore_ascii_case("test") {
        let pins = load_lcd_pins(nvs);
        match draw_lcd_scene(&pins, "ETHAN", "LCD test pattern") {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, "??LCD test pattern rendered");
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("LCD test failed: {}", err));
            }
        }
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("draw ") {
        let pins = load_lcd_pins(nvs);
        match draw_lcd_scene(&pins, "ETHAN", rest.trim()) {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, "??LCD message rendered");
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("LCD draw failed: {}", err));
            }
        }
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id, "Usage:\n/lcd\n/lcd default\n/lcd pins rst=21 dc=47 bl=38 sclk=19 sda=20 cs=45 width=240 height=240\n/lcd test\n/lcd draw 你好 ETHAN");
}

/// Split "SSID PASSWORD" or "SSID|PASSWORD" into (ssid, password).
/// Supports both space and pipe separators. Leading/trailing whitespace is trimmed.
fn split_ssid_pass(input: &str) -> (String, String) {
    let input = input.trim();
    // Try pipe separator first (backward compat)
    if let Some(pos) = input.find('|') {
        let ssid = input[..pos].trim().to_string();
        let pass = input[pos + 1..].trim().to_string();
        return (ssid, pass);
    }
    // Otherwise split on first space
    if let Some(pos) = input.find(' ') {
        let ssid = input[..pos].trim().to_string();
        let pass = input[pos + 1..].trim().to_string();
        (ssid, pass)
    } else {
        // No separator — SSID only, no password (open network)
        (input.to_string(), String::new())
    }
}

fn wifi_nvs_keys(slot: u8) -> (String, String) {
    if slot == 1 {
        ("wifi_ssid".to_string(), "wifi_pass".to_string())
    } else {
        (format!("wifi_ssid{}", slot), format!("wifi_pass{}", slot))
    }
}

fn handle_wifi_command(cfg: &Config, nvs: &mut EspNvs<NvsDefault>, chat_id: i64, args: &str) {
    let trimmed = args.trim();
    let trimmed_lower = trimmed.to_ascii_lowercase();

    // /wifi (status) — 顯示所有已設定的 WiFi
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        let mut lines = vec!["\u{1f4f6} WiFi \u{8a2d}\u{5b9a}\n".to_string()];
        for i in 1..=5u8 {
            let (sk, pk) = wifi_nvs_keys(i);
            let ssid = nvs_get(nvs, &sk).unwrap_or_default();
            let has_pass = nvs_get(nvs, &pk).map(|v| !v.is_empty()).unwrap_or(false);
            if ssid.is_empty() {
                lines.push(format!("  {} (\u{672a}\u{8a2d}\u{5b9a})", i));
            } else {
                let lock = if has_pass { " \u{1f512}" } else { "" };
                lines.push(format!("  {} {}{}", i, ssid, lock));
            }
        }
        lines.push(String::new());
        lines.push("\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}".to_string());
        lines.push("\u{6307}\u{4ee4}\u{ff1a}".to_string());
        lines.push("/wifi set<N> SSID \u{5bc6}\u{78bc}".to_string());
        lines.push("  N=1~5  \u{4f8b}: /wifi set SSID \u{5bc6}\u{78bc}".to_string());
        lines.push("  \u{4f8b}: /wifi set3 SSID \u{5bc6}\u{78bc}".to_string());
        lines.push("/wifi del <N>".to_string());
        lines.push("/wifi swap".to_string());
        lines.push("/wifi clear".to_string());
        lines.push(String::new());
        lines.push("\u{1f4a1} \u{8a2d}\u{5b9a}\u{5f8c}\u{7cfb}\u{7d71}\u{81ea}\u{52d5}\u{91cd}\u{555f}\u{ff01}".to_string());
        let _ = send_telegram(&cfg.tg_token, chat_id, &lines.join("\n"));
        return;
    }

    // /wifi clear — 清空所有 WiFi 設定
    if trimmed.eq_ignore_ascii_case("clear") {
        for i in 1..=5u8 {
            let (sk, pk) = wifi_nvs_keys(i);
            let _ = nvs.remove(&sk);
            let _ = nvs.remove(&pk);
        }
        let _ = send_telegram(&cfg.tg_token, chat_id, "\u{2705} WiFi \u{8a2d}\u{5b9a}\u{5df2}\u{5168}\u{90e8}\u{6e05}\u{7a7a}\u{ff0c}\u{7cfb}\u{7d71}\u{5c07}\u{91cd}\u{555f}\u{3002}");
        unsafe { esp_idf_svc::sys::esp_restart(); }
    }

    // /wifi swap — 1↔2 互換
    if trimmed.eq_ignore_ascii_case("swap") {
        let s1 = nvs_get(nvs, "wifi_ssid").unwrap_or_default();
        let p1 = nvs_get(nvs, "wifi_pass").unwrap_or_default();
        let s2 = nvs_get(nvs, "wifi_ssid2").unwrap_or_default();
        let p2 = nvs_get(nvs, "wifi_pass2").unwrap_or_default();
        if s2.is_empty() {
            let _ = send_telegram(&cfg.tg_token, chat_id, "\u{274c} WiFi 2 \u{672a}\u{8a2d}\u{5b9a}\u{ff0c}\u{7121}\u{6cd5}\u{4e92}\u{63db}");
            return;
        }
        let _ = nvs.set_str("wifi_ssid", &s2);
        let _ = nvs.set_str("wifi_pass", &p2);
        let _ = nvs.set_str("wifi_ssid2", &s1);
        let _ = nvs.set_str("wifi_pass2", &p1);
        let _ = send_telegram(&cfg.tg_token, chat_id,
            &format!("\u{2705} \u{4e92}\u{63db}\u{5b8c}\u{6210}\u{ff01}\n1: {} \u{2192} {}\n\u{7cfb}\u{7d71}\u{91cd}\u{555f}\u{4e2d}...", s1, s2));
        unsafe { esp_idf_svc::sys::esp_restart(); }
    }

    // /wifi del <N> — 刪除指定插槽
    if trimmed_lower.starts_with("del ") || trimmed_lower.starts_with("del\t") {
        let rest = trimmed[4..].trim();
        if let Ok(slot) = rest.parse::<u8>() {
            if slot >= 1 && slot <= 5 {
                let (sk, pk) = wifi_nvs_keys(slot);
                let _ = nvs.remove(&sk);
                let _ = nvs.remove(&pk);
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    &format!("\u{2705} WiFi {} \u{5df2}\u{522a}\u{9664}", slot));
                return;
            }
        }
        let _ = send_telegram(&cfg.tg_token, chat_id, "\u{274c} \u{7528}\u{6cd5}\u{ff1a}/wifi del <1~5>");
        return;
    }

    // /wifi set<N> SSID PASSWORD — 設定指定插槽 (set = set1, set2..set5)
    // Parse slot number from "set", "set1", "set2" ... "set5"
    let maybe_set = if trimmed_lower.starts_with("set") {
        let after_set = &trimmed_lower[3..];
        // "set SSID pass" → slot 1, rest starts after "set "
        // "set2 SSID pass" → slot 2, rest starts after "set2 "
        if after_set.starts_with(' ') || after_set.starts_with('\t') {
            Some((1u8, &trimmed[4..]))  // "set " = 4 chars
        } else if after_set.len() >= 2 {
            let digit = after_set.as_bytes()[0];
            let sep = after_set.as_bytes()[1];
            if digit >= b'1' && digit <= b'5' && (sep == b' ' || sep == b'\t') {
                Some((digit - b'0', &trimmed[5..]))  // "setN " = 5 chars
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some((slot, rest)) = maybe_set {
        let (ssid, password) = split_ssid_pass(rest);
        if ssid.is_empty() {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("\u{274c} \u{7528}\u{6cd5}\u{ff1a}/wifi set{} SSID \u{5bc6}\u{78bc}", if slot == 1 { String::new() } else { slot.to_string() }));
            return;
        }
        let (sk, pk) = wifi_nvs_keys(slot);
        if nvs.set_str(&sk, &ssid).is_ok() && nvs.set_str(&pk, &password).is_ok() {
            let restart = slot == 1;
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("\u{2705} WiFi {} \u{5df2}\u{8a2d}\u{70ba} {}{}", slot, ssid,
                    if restart { "\n\u{7cfb}\u{7d71}\u{91cd}\u{555f}\u{4e2d}..." } else { "" }));
            if restart {
                unsafe { esp_idf_svc::sys::esp_restart(); }
            }
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id, "\u{274c} WiFi \u{5beb}\u{5165} NVS \u{5931}\u{6557}");
        }
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id,
        "\u{1f4f6} WiFi \u{6307}\u{4ee4}\u{ff1a}\n\n\
         /wifi \u{2014} \u{67e5}\u{770b}\u{76ee}\u{524d}\u{8a2d}\u{5b9a}\n\
         /wifi set SSID \u{5bc6}\u{78bc} (\u{4e3b}\u{7db2}\u{8def})\n\
         /wifi set2~5 SSID \u{5bc6}\u{78bc}\n\
         /wifi del <1~5>\n\
         /wifi swap \u{2014} 1\u{2194}2\u{4e92}\u{63db}\n\
         /wifi clear \u{2014} \u{6e05}\u{7a7a}\u{5168}\u{90e8}");
}

fn handle_token_command(cfg: &Config, nvs: &mut EspNvs<NvsDefault>, chat_id: i64, args: &str) {
    let trimmed = args.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        let current = nvs_get(nvs, "tg_token").unwrap_or_default();
        let masked = if current.len() > 10 {
            format!("{}...{}", &current[..5], &current[current.len()-5..])
        } else {
            "***".to_string()
        };
        let _ = send_telegram(&cfg.tg_token, chat_id,
            &format!("\u{1f511} Telegram Token\n\n\
                      \u{76ee}\u{524d}: {}\n\n\
                      \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n\
                      /token set <\u{65b0}TOKEN> \u{2014} \u{66f4}\u{63db} Token\n\n\
                      \u{26a0}\u{fe0f} \u{66f4}\u{63db}\u{5f8c}\u{7cfb}\u{7d71}\u{81ea}\u{52d5}\u{91cd}\u{555f}\u{ff01}", masked));
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("set ").or_else(|| trimmed.strip_prefix("set\t")) {
        let new_token = rest.trim();
        if new_token.is_empty() || new_token.len() < 20 {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "\u{274c} Token \u{683c}\u{5f0f}\u{4e0d}\u{6b63}\u{78ba}\u{ff0c}\u{61c9}\u{70ba} BotFather \u{63d0}\u{4f9b}\u{7684}\u{5b8c}\u{6574} Token");
            return;
        }
        if nvs.set_str("tg_token", new_token).is_ok() {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "\u{2705} Telegram Token \u{5df2}\u{66f4}\u{65b0}\u{ff0c}\u{7cfb}\u{7d71}\u{91cd}\u{555f}\u{4e2d}...");
            unsafe { esp_idf_svc::sys::esp_restart(); }
        } else {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                "\u{274c} Token \u{5beb}\u{5165} NVS \u{5931}\u{6557}");
        }
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id,
        "\u{1f511} Token \u{6307}\u{4ee4}\u{ff1a}\n\n\
         /token \u{2014} \u{67e5}\u{770b}\u{76ee}\u{524d} Token\n\
         /token set <TOKEN> \u{2014} \u{66f4}\u{63db} Telegram Bot Token\n\n\
         \u{26a0}\u{fe0f} \u{66f4}\u{63db}\u{5f8c}\u{7cfb}\u{7d71}\u{81ea}\u{52d5}\u{91cd}\u{555f}\u{ff01}");
}

fn handle_audio_command(cfg: &Config, nvs: &mut EspNvs<NvsDefault>, state: &mut AppState, chat_id: i64, args: &str) {
    let trimmed = args.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("status") {
        let pins = load_audio_pins(nvs);
        let proxy = load_tts_proxy_url(nvs).unwrap_or_else(|| "disabled".to_string());
        let voice = load_tts_proxy_voice(nvs);
        let msg = format!(
            "Onboard Audio\n\n{}\nWake: {} ({})\nTTS proxy: {}\nTTS voice: {}\n\nCommands:\n/audio default\n/audio pins bclk=40 ws=41 dout=39 mic_ws=1 mic_sck=2 mic_din=42 mclk=-1 rate=24000\n/audio proxy\n/audio proxy set <url>\n/audio proxy off\n/audio proxy voice <name>\n/audio tone\n/audio mic\n/audio transcribe\n/audio say 你好 ETHAN\n/audio wake\n/audio wake on|off\n/audio wake set ethan\n/audio wake now\n/audio level",
            pins.status_text(),
            if state.wake_enabled { "on" } else { "off" },
            state.wake_phrase,
            proxy,
            voice,
        );
        let _ = send_telegram(&cfg.tg_token, chat_id, &msg);
        return;
    }

    if trimmed.eq_ignore_ascii_case("proxy") {
        let proxy = load_tts_proxy_url(nvs).unwrap_or_else(|| "disabled".to_string());
        let voice = load_tts_proxy_voice(nvs);
        let _ = send_telegram(&cfg.tg_token, chat_id, &format!("TTS proxy: {}\nVoice: {}", proxy, voice));
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("proxy ") {
        let arg = rest.trim();
        if arg.eq_ignore_ascii_case("off") {
            let _ = nvs.remove(TTS_PROXY_URL_KEY);
            let _ = send_telegram(&cfg.tg_token, chat_id, "??TTS proxy disabled");
            return;
        }
        if let Some(voice) = arg.strip_prefix("voice ") {
            let voice = voice.trim();
            if voice.is_empty() {
                let _ = send_telegram(&cfg.tg_token, chat_id, "Usage: /audio proxy voice <voice-name>");
            } else if nvs.set_str(TTS_PROXY_VOICE_KEY, voice).is_ok() {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??TTS proxy voice set to {}", voice));
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id, "TTS proxy voice save failed");
            }
            return;
        }
        if let Some(url) = arg.strip_prefix("set ") {
            let url = url.trim();
            if url.is_empty() {
                let _ = send_telegram(&cfg.tg_token, chat_id, "Usage: /audio proxy set <url>");
            } else if nvs.set_str(TTS_PROXY_URL_KEY, url).is_ok() {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??TTS proxy set to {}", url));
            } else {
                let _ = send_telegram(&cfg.tg_token, chat_id, "TTS proxy save failed");
            }
            return;
        }
    }

    if trimmed.eq_ignore_ascii_case("default") {
        let pins = board_hw::AudioPins::default();
        match save_audio_pins(nvs, &pins) {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Audio pins reset to default\n\n{}", pins.status_text()));
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Audio default save failed: {}", err));
            }
        }
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("pins ") {
        let mut pins = load_audio_pins(nvs);
        match parse_audio_pin_args(rest, &mut pins).and_then(|_| save_audio_pins(nvs, &pins)) {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Audio pins updated\n\n{}", pins.status_text()));
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Audio pin update failed: {}", err));
            }
        }
        return;
    }

    if trimmed.eq_ignore_ascii_case("tone") {
        let pins = load_audio_pins(nvs);
        match play_test_tone(&pins) {
            Ok(()) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, "??Audio tone test played");
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Audio tone test failed: {}", err));
            }
        }
        return;
    }

    if trimmed.eq_ignore_ascii_case("mic") {
        let pins = load_audio_pins(nvs);
        match board_hw::capture_mic_snapshot(&pins, 1500) {
            Ok(snapshot) => {
                let _ = send_telegram(
                    &cfg.tg_token,
                    chat_id,
                    &format!(
                        "\u{1f399}\u{fe0f} Mic snapshot\nRMS: {}\nPeak: {}\nSamples: {}",
                        snapshot.rms, snapshot.peak, snapshot.samples
                    ),
                );
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Mic test failed: {}", err));
            }
        }
        return;
    }

    if trimmed.eq_ignore_ascii_case("transcribe") {
        match capture_and_transcribe_mic(cfg, nvs, state, 2500) {
            Ok(transcript) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("🎤 Mic transcript\n{}", transcript));
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Mic transcribe failed: {}", err));
            }
        }
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("say ") {
        match speak_onboard_text(nvs, rest.trim()) {
            Ok(speech) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("🔊 Board spoke: {}", speech));
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Board TTS failed: {}", err));
            }
        }
        return;
    }

    if trimmed.eq_ignore_ascii_case("wake") {
        let _ = send_telegram(
            &cfg.tg_token,
            chat_id,
            &format!("Wake listener: {}\nPhrase: {}\n\nUse:\n/audio wake on\n/audio wake off\n/audio wake set ethan\n/audio wake now", if state.wake_enabled { "on" } else { "off" }, state.wake_phrase),
        );
        return;
    }

    if let Some(rest) = trimmed.strip_prefix("wake ") {
        let arg = rest.trim();
        if arg.eq_ignore_ascii_case("on") {
            state.wake_enabled = true;
            save_runtime_prefs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Wake listener enabled ({})", state.wake_phrase));
            return;
        }
        if arg.eq_ignore_ascii_case("off") {
            state.wake_enabled = false;
            save_runtime_prefs(nvs, state);
            let _ = send_telegram(&cfg.tg_token, chat_id, "??Wake listener disabled");
            return;
        }
        if let Some(phrase) = arg.strip_prefix("set ") {
            let phrase = phrase.trim();
            if phrase.is_empty() {
                let _ = send_telegram(&cfg.tg_token, chat_id, "Usage: /audio wake set <phrase>");
            } else {
                state.wake_phrase = phrase.to_string();
                save_runtime_prefs(nvs, state);
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Wake phrase set to {}", state.wake_phrase));
            }
            return;
        }
        if arg.eq_ignore_ascii_case("now") {
            match detect_wake_phrase(cfg, nvs, state, 2500) {
                Ok(Some(transcript)) => {
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("??Wake phrase matched\n{}", transcript));
                    if should_speak_onboard(&state.voice_mode) {
                        let _ = speak_onboard_text(nvs, "好的，請說！");
                    }
                }
                Ok(None) => {
                    let _ = send_telegram(&cfg.tg_token, chat_id, "⏰ Wake phrase not heard this round");
                }
                Err(err) => {
                    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Wake phrase test failed: {}", err));
                }
            }
            return;
        }
    }

    if trimmed.eq_ignore_ascii_case("level") {
        let pins = load_audio_pins(nvs);
        match wake_probe(&pins, 3000, 3500) {
            Ok(triggered) => {
                let msg = if triggered {
                    "??Audio level probe triggered: detected voice-level audio"
                } else {
                    "\u{1f50a} Audio level probe idle: no voice-level audio detected"
                };
                let _ = send_telegram(&cfg.tg_token, chat_id, msg);
            }
            Err(err) => {
                let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Audio level probe failed: {}", err));
            }
        }
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id, "Usage:\n/audio\n/audio default\n/audio pins bclk=40 ws=41 dout=39 mic_ws=1 mic_sck=2 mic_din=42 mclk=-1 rate=24000\n/audio proxy\n/audio proxy set <url>\n/audio proxy off\n/audio proxy voice <name>\n/audio tone\n/audio mic\n/audio transcribe\n/audio say 你好 ETHAN\n/audio wake\n/audio wake on|off\n/audio wake set ethan\n/audio wake now\n/audio level");
}

// ===== PC Command Handler =====

fn handle_pc_command(cfg: &Config, _state: &mut AppState, chat_id: i64, args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.is_empty() || parts[0].is_empty() {
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "/pc commands:\n\
             shell <cmd> - run command\n\
             file_read <path>\n\
             file_list [path]\n\
             file_write <path> <content>\n\
             screenshot\n\
             status\n\
             open <target>\n\
             click <x> <y>\n\
             type <text>\n\
             hotkey <key1+key2>\n\
             mouse_move <x> <y>\n\
             scroll <amount>\n\
             find_window <title>\n\
             focus_window <title>\n\
             excel_read <path>\n\
             email_send <to> <subj> <body>");
        return;
    }

    let cmd = parts[0];
    let rest = if parts.len() > 1 { parts[1] } else { "" };

    let (driver_cmd, args_json) = match cmd {
        "shell" => ("shell", serde_json::json!({"command": rest})),
        "file_read" => ("file_read", serde_json::json!({"path": rest})),
        "file_list" => ("file_list", serde_json::json!({"path": if rest.is_empty() { "." } else { rest }})),
        "file_write" => {
            let sub: Vec<&str> = rest.splitn(2, ' ').collect();
            ("file_write", serde_json::json!({"path": sub.first().unwrap_or(&""), "content": sub.get(1).unwrap_or(&"")}))
        }
        "screenshot" => ("screenshot", serde_json::json!({})),
        "status" => ("status", serde_json::json!({})),
        "open" => ("open", serde_json::json!({"target": rest})),
        "click" => {
            let sub: Vec<&str> = rest.splitn(2, ' ').collect();
            let x: i32 = sub.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            let y: i32 = sub.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            ("click", serde_json::json!({"x": x, "y": y, "button": "left"}))
        }
        "type" => ("type", serde_json::json!({"text": rest})),
        "hotkey" => {
            let keys: Vec<&str> = rest.split('+').map(|s| s.trim()).collect();
            ("hotkey", serde_json::json!({"keys": keys}))
        }
        "mouse_move" => {
            let sub: Vec<&str> = rest.splitn(2, ' ').collect();
            let x: i32 = sub.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            let y: i32 = sub.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
            ("mouse_move", serde_json::json!({"x": x, "y": y}))
        }
        "scroll" => {
            let amount: i32 = rest.trim().parse().unwrap_or(3);
            ("scroll", serde_json::json!({"amount": amount}))
        }
        "find_window" => ("find_window", serde_json::json!({"title": rest})),
        "focus_window" => ("focus_window", serde_json::json!({"title": rest})),
        "excel_read" => ("excel", serde_json::json!({"action": "read", "path": rest})),
        "excel_write" => {
            let sub: Vec<&str> = rest.splitn(3, ' ').collect();
            ("excel", serde_json::json!({"action": "write", "path": sub.first().unwrap_or(&""), "cell": sub.get(1).unwrap_or(&"A1"), "value": sub.get(2).unwrap_or(&"")}))
        }
        "email_send" => {
            let sub: Vec<&str> = rest.splitn(3, ' ').collect();
            ("email", serde_json::json!({"action": "send", "to": sub.first().unwrap_or(&""), "subject": sub.get(1).unwrap_or(&""), "body": sub.get(2).unwrap_or(&"")}))
        }
        "clipboard" => ("clipboard", serde_json::json!({"action": "get"})),
        "process" => ("process", serde_json::json!({"action": "list"})),
        _ => (cmd, serde_json::json!({"raw": rest})),
    };

    if !DRIVER_CONNECTED.load(Ordering::Relaxed) {
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "\u{26a0}\u{fe0f} PC Driver \u{672a}\u{9023}\u{7dda}\u{ff01}\u{8acb}\u{5148}\u{5728}\u{96fb}\u{8166}\u{57f7}\u{884c} ethan_driver.py\n\
             (\u{547d}\u{4ee4}\u{5df2}\u{9001}\u{51fa}\u{ff0c}\u{82e5} Driver \u{5df2}\u{555f}\u{52d5}\u{8acb}\u{7b49}\u{5f85}\u{56de}\u{61c9})");
    }
    let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Sending to PC: {} ...", cmd));
    usb_send_pc_cmd(driver_cmd, &args_json, chat_id);
}

// ===== Remind Command =====

fn handle_remind_command(cfg: &Config, state: &mut AppState, nvs: &mut EspNvs<NvsDefault>, chat_id: i64, args: &str) {
    let parts: Vec<&str> = args.splitn(2, ' ').collect();

    if parts[0] == "del" || parts[0] == "delete" {
        if let Some(id_str) = parts.get(1) {
            if let Ok(id) = id_str.trim().parse::<u32>() {
                if state.remove_reminder(id) {
                    save_reminders_to_nvs(nvs, state);
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("Deleted reminder #{}", id));
                } else {
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("Reminder #{} not found", id));
                }
                return;
            }
        }
        let _ = send_telegram(&cfg.tg_token, chat_id, "Usage: /remind del <ID>");
        return;
    }

    if parts[0] == "list" {
        let _ = send_telegram(&cfg.tg_token, chat_id, &state.list_reminders());
        return;
    }

    if parts.len() < 2 {
        let _ = send_telegram(&cfg.tg_token, chat_id,
            "Usage:\n/remind 5m message\n/remind 8:00 message\n/remind del 3");
        return;
    }

    let pattern = parts[0];
    let message = parts[1];
    let now = now_epoch();

    // Interval: 5m, 30m, 1h
    if pattern.ends_with('m') {
        if let Ok(mins) = pattern[..pattern.len()-1].parse::<u64>() {
            if mins > 0 && mins <= 1440 {
                let id = state.add_reminder(
                    ReminderType::Interval, mins * 60, 0, 0, message, now);
                save_reminders_to_nvs(nvs, state);
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    &format!("Reminder #{}: every {}min \"{}\"", id, mins, message));
                return;
            }
        }
    }
    if pattern.ends_with('h') {
        if let Ok(hrs) = pattern[..pattern.len()-1].parse::<u64>() {
            if hrs > 0 && hrs <= 24 {
                let id = state.add_reminder(
                    ReminderType::Interval, hrs * 3600, 0, 0, message, now);
                save_reminders_to_nvs(nvs, state);
                let _ = send_telegram(&cfg.tg_token, chat_id,
                    &format!("Reminder #{}: every {}hr \"{}\"", id, hrs, message));
                return;
            }
        }
    }

    // Daily: HH:MM
    if pattern.contains(':') {
        let hm: Vec<&str> = pattern.split(':').collect();
        if hm.len() == 2 {
            if let (Ok(h), Ok(m)) = (hm[0].parse::<u32>(), hm[1].parse::<u32>()) {
                if h < 24 && m < 60 {
                    let id = state.add_reminder(
                        ReminderType::Daily, 0, h, m, message, now);
                    save_reminders_to_nvs(nvs, state);
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("Reminder #{}: daily {:02}:{:02} \"{}\"",
                            id, h, m, message));
                    return;
                }
            }
        }
    }

    let _ = send_telegram(&cfg.tg_token, chat_id,
        "Format error.\nUse: /remind 5m msg | /remind 8:00 msg");
}

// ===== Reminder Parser (from AI response) =====

fn parse_and_apply_reminder(reply: &str, state: &mut AppState) -> (String, bool) {
    let now = now_epoch();
    let mut clean = reply.to_string();
    let mut applied = false;

    while let Some(start) = clean.find("[REMIND:") {
        if let Some(end) = clean[start..].find(']') {
            let tag = &clean[start+8..start+end];
            let parts: Vec<&str> = tag.splitn(3, ':').collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "interval" => {
                        if let Ok(mins) = parts[1].parse::<u64>() {
                            let msg = if parts.len() > 2 { parts[2] } else { "reminder" };
                            state.add_reminder(
                                ReminderType::Interval, mins * 60, 0, 0, msg, now);
                            applied = true;
                        }
                    }
                    "daily" => {
                        let time_str = parts[1];
                        let hm: Vec<&str> = time_str.split(':').collect();
                        // Handle "daily:HH:MM:msg" where parts = ["daily", "HH", "MM:msg"]
                        // or the splitn(3) may give ["daily", "08", "00:message"]
                        if hm.len() == 2 {
                            if let (Ok(h), Ok(m)) = (hm[0].parse::<u32>(), hm[1].parse::<u32>()) {
                                let msg = if parts.len() > 2 { parts[2] } else { "reminder" };
                                state.add_reminder(
                                    ReminderType::Daily, 0, h, m, msg, now);
                                applied = true;
                            }
                        } else if let Ok(h) = time_str.parse::<u32>() {
                            // parts = ["daily", "08", "00:msg"]
                            if parts.len() > 2 {
                                let rest = parts[2];
                                let sub: Vec<&str> = rest.splitn(2, ':').collect();
                                if let Ok(m) = sub[0].parse::<u32>() {
                                    let msg = if sub.len() > 1 { sub[1] } else { "reminder" };
                                    state.add_reminder(
                                        ReminderType::Daily, 0, h, m, msg, now);
                                    applied = true;
                                }
                            }
                        }
                    }
                    "once" => {
                        let time_str = parts[1];
                        if let Ok(h) = time_str.parse::<u32>() {
                            if parts.len() > 2 {
                                let rest = parts[2];
                                let sub: Vec<&str> = rest.splitn(2, ':').collect();
                                if let Ok(m) = sub[0].parse::<u32>() {
                                    let msg = if sub.len() > 1 { sub[1] } else { "reminder" };
                                    state.add_reminder(
                                        ReminderType::Once, 0, h, m, msg, now);
                                    applied = true;
                                }
                            }
                        }
                    }
                    "del" => {
                        if let Ok(id) = parts[1].parse::<u32>() {
                            state.remove_reminder(id);
                            applied = true;
                        }
                    }
                    "list" => {
                        // 自動任務特殊回覆（對比 [AUTOTASK:list] 編輯）
let remind_list = state.list_reminders();
                        clean = format!("{}", remind_list);
                    }
                    _ => {}
                }
            }
            clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

/// 統一處理 AI 回覆中的所有命令標籤（記憶 + 自動任務），並儲存到 NVS。
/// 回傳清理後的純文字回覆。
fn parse_all_ai_commands(reply: &str, state: &mut AppState, nvs: &mut EspNvs<NvsDefault>) -> String {
    let now = now_epoch();
    let (mut clean, reminder_changed) = parse_and_apply_reminder(reply, state);

    // Parse [AUTOTASK:...] tags
    let mut autotask_changed = false;
    loop {
        let start = match clean.find("[AUTOTASK:") {
            Some(s) => s,
            None => break,
        };
        let end = match clean[start..].find(']') {
            Some(e) => e,
            None => break,
        };
        let tag = clean[start + 10..start + end].to_string();
        clean = format!("{}{}", &clean[..start], clean[start + end + 1..].trim_start());

        let parts: Vec<&str> = tag.splitn(5, ':').collect();
        if parts.is_empty() { continue; }
        match parts[0] {
            "camera" => {
                if parts.len() >= 3 {
                    if let Ok(mins) = parts[1].parse::<u64>() {
                        if mins > 0 {
                            let prompt = parts[2].to_string();
                            let speak = parts.get(3).map_or(false, |&s| s.eq_ignore_ascii_case("speak"));
                            let id = state.next_autotask_id;
                            state.next_autotask_id += 1;
                            state.auto_tasks.push(AutoTask {
                                id,
                                interval_secs: mins * 60,
                                next_trigger: now + mins * 60,
                                action: "camera".to_string(),
                                prompt, speak, active: true,
                            });
                            autotask_changed = true;
                            info!("AutoTask #{} added: camera every {}min", id, mins);
                        }
                    }
                }
            }
            "del" => {
                if let Some(id_str) = parts.get(1) {
                    if let Ok(id) = id_str.parse::<u32>() {
                        let before = state.auto_tasks.len();
                        state.auto_tasks.retain(|t| t.id != id);
                        if state.auto_tasks.len() < before { autotask_changed = true; }
                    }
                }
            }
            "list" => {
                // 自動任務特殊回覆，AI 不需要再發給用戶
                let task_list = list_auto_tasks(state);
                clean = format!("{}\n{}\n{}", &clean[..start].trim(), task_list, &clean[start..].trim());
            }
            _ => {}
        }
    }

    if reminder_changed {
        save_reminders_to_nvs(nvs, state);
    }
    if autotask_changed {
        save_auto_tasks_to_nvs(nvs, state);
    }

    clean.trim().to_string()
}

fn token_footer(state: &AppState) -> String {
    let total = state.tokens_in + state.tokens_out;
    let last_in = state.tokens_in.saturating_sub(state.last_tokens_in);
    let last_out = state.tokens_out.saturating_sub(state.last_tokens_out);
    let last = last_in + last_out;
    if state.requests > 0 {
        format!("\n\n[{} | {} tok (in:{}/out:{}) | total: {} | #{}]",
            state.current_model, last, last_in, last_out, total, state.requests)
    } else {
        String::new()
    }
}

// ===== Memory System =====

fn load_memories_from_nvs(nvs: &EspNvs<NvsDefault>, state: &mut AppState) {
    let count: usize = nvs_get(nvs, "mem_cnt")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    for i in 0..count.min(MAX_MEMORIES) {
        let nkey = format!("m{:02}", i);
        if let Some(val) = nvs_get(nvs, &nkey) {
            if let Some((k, v)) = val.split_once('|') {
                state.memories.push((k.to_string(), v.to_string()));
            }
        }
    }
}

fn save_memories_to_nvs(nvs: &mut EspNvs<NvsDefault>, state: &AppState) {
    let _ = nvs.set_str("mem_cnt", &state.memories.len().to_string());
    for (i, (k, v)) in state.memories.iter().enumerate() {
        let nkey = format!("m{:02}", i);
        let combined = format!("{}|{}", k, v);
        let _ = nvs.set_str(&nkey, &combined);
    }
    for i in state.memories.len()..MAX_MEMORIES {
        let nkey = format!("m{:02}", i);
        let _ = nvs.remove(&nkey);
    }
}

// ===== Reminder NVS Persistence =====

fn save_reminders_to_nvs(nvs: &mut EspNvs<NvsDefault>, state: &AppState) {
    let active: Vec<&Reminder> = state.reminders.iter().filter(|r| r.active).collect();
    let save_count = active.len().min(MAX_REMINDERS);
    let _ = nvs.set_str("r_cnt", &save_count.to_string());
    let _ = nvs.set_str("r_nid", &state.next_reminder_id.to_string());
    for (i, r) in active.iter().enumerate().take(save_count) {
        let nkey = format!("r{:02}", i);
        let rtype_int: u8 = match r.rtype {
            ReminderType::Interval => 0,
            ReminderType::Daily => 1,
            ReminderType::Once => 2,
        };
        let val = format!("{}|{}|{}|{}|{}|{}", r.id, rtype_int, r.interval_secs, r.hour, r.minute, r.message);
        let _ = nvs.set_str(&nkey, &val);
    }
    for i in save_count..MAX_REMINDERS {
        let nkey = format!("r{:02}", i);
        let _ = nvs.remove(&nkey);
    }
    info!("Saved {} reminders to NVS", save_count);
}

fn load_reminders_from_nvs(nvs: &EspNvs<NvsDefault>, state: &mut AppState, now: u64) {
    let count: usize = nvs_get(nvs, "r_cnt")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let next_id: u32 = nvs_get(nvs, "r_nid")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    state.next_reminder_id = next_id.max(1);
    for i in 0..count.min(MAX_REMINDERS) {
        let nkey = format!("r{:02}", i);
        if let Some(val) = nvs_get(nvs, &nkey) {
            let parts: Vec<&str> = val.splitn(6, '|').collect();
            if parts.len() < 6 { continue; }
            let id: u32 = parts[0].parse().unwrap_or(0);
            let rtype_int: u8 = parts[1].parse().unwrap_or(0);
            let interval_secs: u64 = parts[2].parse().unwrap_or(60);
            let hour: u32 = parts[3].parse().unwrap_or(0);
            let minute: u32 = parts[4].parse().unwrap_or(0);
            let message = parts[5].to_string();
            let rtype = match rtype_int {
                1 => ReminderType::Daily,
                2 => ReminderType::Once,
                _ => ReminderType::Interval,
            };
            let next_trigger = match rtype {
                ReminderType::Daily => next_daily_trigger(hour, minute, now),
                ReminderType::Interval => now + interval_secs,
                ReminderType::Once => {
                    let t = next_daily_trigger(hour, minute, now);
                    if t <= now { continue; } // already passed, skip
                    t
                }
            };
            state.reminders.push(Reminder {
                id, rtype, interval_secs, next_trigger, hour, minute, message, active: true,
            });
        }
    }
    info!("Loaded {} reminders from NVS", state.reminders.len());
}

// ===== AutoTask NVS Persistence =====

fn save_auto_tasks_to_nvs(nvs: &mut EspNvs<NvsDefault>, state: &AppState) {
    let active: Vec<&AutoTask> = state.auto_tasks.iter().filter(|t| t.active).collect();
    let save_count = active.len().min(MAX_AUTO_TASKS);
    let _ = nvs.set_str("at_cnt", &save_count.to_string());
    let _ = nvs.set_str("at_nid", &state.next_autotask_id.to_string());
    for (i, t) in active.iter().enumerate().take(save_count) {
        let nkey = format!("at{:02}", i);
        let speak_int = if t.speak { 1u8 } else { 0u8 };
        // Format: id|interval_secs|speak|action|prompt (splitn 5 so prompt can contain |)
        let val = format!("{}|{}|{}|{}|{}", t.id, t.interval_secs, speak_int, t.action, t.prompt);
        let _ = nvs.set_str(&nkey, &val);
    }
    for i in save_count..MAX_AUTO_TASKS {
        let nkey = format!("at{:02}", i);
        let _ = nvs.remove(&nkey);
    }
    info!("Saved {} auto tasks to NVS", save_count);
}

fn load_auto_tasks_from_nvs(nvs: &EspNvs<NvsDefault>, state: &mut AppState, now: u64) {
    let count: usize = nvs_get(nvs, "at_cnt")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let next_id: u32 = nvs_get(nvs, "at_nid")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    state.next_autotask_id = next_id.max(1);
    for i in 0..count.min(MAX_AUTO_TASKS) {
        let nkey = format!("at{:02}", i);
        if let Some(val) = nvs_get(nvs, &nkey) {
            let parts: Vec<&str> = val.splitn(5, '|').collect();
            if parts.len() < 5 { continue; }
            let id: u32 = parts[0].parse().unwrap_or(0);
            let interval_secs: u64 = parts[1].parse().unwrap_or(300);
            let speak = parts[2] == "1";
            let action = parts[3].to_string();
            let prompt = parts[4].to_string();
            state.auto_tasks.push(AutoTask {
                id, interval_secs,
                next_trigger: now + interval_secs, // restart interval from each boot
                action, prompt, speak, active: true,
            });
        }
    }
    info!("Loaded {} auto tasks from NVS", state.auto_tasks.len());
}

fn list_auto_tasks(state: &AppState) -> String {
    let active: Vec<&AutoTask> = state.auto_tasks.iter().filter(|t| t.active).collect();
    if active.is_empty() {
        return "📭 目前沒有進行中的自動任務\n\n新增方式：告訴 AI 你要的任務，例如：\n每5分鐘拍照問誰在睡覺用講的\n或：/tasks clear".to_string();
    }
    let now = now_epoch();
    let mut s = format!("📋 自動任務清單 ({} 個):\n", active.len());
    for t in &active {
        let mins = t.interval_secs / 60;
        let secs = t.interval_secs % 60;
        let interval_str = if secs == 0 { format!("每{}分", mins) } else { format!("每{}分{}秒", mins, secs) };
        let speak_str = if t.speak { "📢語音" } else { "📨Telegram" };
        let countdown = if t.next_trigger > now {
            let rem = t.next_trigger - now;
            if rem >= 60 { format!("{}分{}秒後", rem / 60, rem % 60) } else { format!("{}秒後", rem) }
        } else {
            "已觸發/等待".to_string()
        };
        s.push_str(&format!("  #{} {} {} {}\n", t.id, interval_str, speak_str, countdown));
        if !t.prompt.is_empty() {
            let short = if t.prompt.chars().count() > 20 {
                format!("{}...", t.prompt.chars().take(20).collect::<String>())
            } else {
                t.prompt.clone()
            };
            s.push_str(&format!("     ??{}\n", short));
        }
    }
    s.push_str("\n────────────────\n");
    s.push_str("🗑️ /tasks del [ID編號]\n");
    s.push_str("清除全部：/tasks clear");
    s
}

fn add_memory(nvs: &mut EspNvs<NvsDefault>, state: &mut AppState, key: &str, value: &str) {
    if let Some(item) = state.memories.iter_mut().find(|(k, _)| k == key) {
        item.1 = value.to_string();
    } else if state.memories.len() < MAX_MEMORIES {
        state.memories.push((key.to_string(), value.to_string()));
    } else {
        state.memories.remove(0);
        state.memories.push((key.to_string(), value.to_string()));
    }
    save_memories_to_nvs(nvs, state);
    info!("Memory saved: {} = {}", key, value);
}

fn remove_memory(nvs: &mut EspNvs<NvsDefault>, state: &mut AppState, key: &str) -> bool {
    let len = state.memories.len();
    state.memories.retain(|(k, _)| k != key);
    if state.memories.len() < len {
        save_memories_to_nvs(nvs, state);
        info!("Memory removed: {}", key);
        true
    } else {
        false
    }
}

fn format_memories(memories: &[(String, String)]) -> String {
    if memories.is_empty() {
        return "No memories stored.\n/remember <key> <value> to add".to_string();
    }
    let mut s = format!("Memories ({}/{}):\n", memories.len(), MAX_MEMORIES);
    for (k, v) in memories {
        s.push_str(&format!("  {} = {}\n", k, v));
    }
    s.push_str("\n/remember <key> <val>\n/forget <key>");
    s
}

fn parse_memory_tags(reply: &str, nvs: &mut EspNvs<NvsDefault>, state: &mut AppState) -> (String, bool) {
    let mut clean = reply.to_string();
    let mut applied = false;

    while let Some(start) = clean.find("[MEM:") {
        if let Some(end) = clean[start..].find(']') {
            let tag = &clean[start+5..start+end];
            if let Some(rest) = tag.strip_prefix("save:") {
                if let Some((key, val)) = rest.split_once(':') {
                    add_memory(nvs, state, key.trim(), val.trim());
                    applied = true;
                }
            } else if let Some(key) = tag.strip_prefix("del:") {
                remove_memory(nvs, state, key.trim());
                applied = true;
            }
            clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

// ===== Python Code Tag Parser =====

/// Parse [PY:code] and [PYBLOCK]...[/PYBLOCK] tags
fn parse_py_tags(reply: &str, chat_id: i64) -> (String, bool) {
    // PC Safe Mode: python_exec is a dangerous command
    if !pc_cmd_allowed("python_exec", true) {
        // Strip tags but don't execute
        let mut clean = reply.to_string();
        while let Some(s) = clean.find("[PYBLOCK]") {
            if let Some(e) = clean.find("[/PYBLOCK]") {
                clean = format!("{}{}", &clean[..s], clean[e+10..].trim_start());
            } else { break; }
        }
        while let Some(s) = clean.find("[PY:") {
            if let Some(e) = clean[s..].find(']') {
                clean = format!("{}{}", &clean[..s], clean[s+e+1..].trim_start());
            } else { break; }
        }
        if clean.trim() != reply.trim() {
            warn!("PC Safe Mode: blocked AI python_exec tags");
        }
        return (clean.trim().to_string(), false);
    }

    let mut clean = reply.to_string();
    let mut applied = false;

    // Handle [PYBLOCK]...[/PYBLOCK] first
    while let Some(start) = clean.find("[PYBLOCK]") {
        if let Some(end) = clean.find("[/PYBLOCK]") {
            let code = &clean[start+9..end];
            usb_send_pc_cmd("python_exec", &serde_json::json!({
                "code": code.trim(),
                "mode": "block"
            }), chat_id);
            applied = true;
            clean = format!("{}{}", &clean[..start], clean[end+10..].trim_start());
        } else {
            break;
        }
    }

    // Handle [PY:code] inline
    while let Some(start) = clean.find("[PY:") {
        if let Some(end) = clean[start..].find(']') {
            let code = &clean[start+4..start+end];
            usb_send_pc_cmd("python_exec", &serde_json::json!({
                "code": code,
                "mode": "inline"
            }), chat_id);
            applied = true;
            clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

// ===== Image Generation Tag Parser =====

/// Parse [IMG:prompt] tags
fn parse_img_tags(reply: &str, chat_id: i64) -> (String, bool) {
    if !pc_cmd_allowed("image_gen", true) {
        let mut clean = reply.to_string();
        while let Some(s) = clean.find("[IMG:") {
            if let Some(e) = clean[s..].find(']') {
                clean = format!("{}{}", &clean[..s], clean[s+e+1..].trim_start());
            } else { break; }
        }
        if clean.trim() != reply.trim() {
            warn!("PC Safe Mode: blocked AI image_gen tags");
        }
        return (clean.trim().to_string(), false);
    }

    let mut clean = reply.to_string();
    let mut applied = false;

    while let Some(start) = clean.find("[IMG:") {
        if let Some(end) = clean[start..].find(']') {
            let prompt = &clean[start+5..start+end];
            usb_send_pc_cmd("image_gen", &serde_json::json!({
                "prompt": prompt,
            }), chat_id);
            applied = true;
            clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

// ===== Excel Pipeline Tag Parser =====

/// Parse [EXCEL:path>>>instruction] tags
fn parse_excel_tags(reply: &str, chat_id: i64) -> (String, bool) {
    if !pc_cmd_allowed("excel", true) {
        let mut clean = reply.to_string();
        while let Some(s) = clean.find("[EXCEL:") {
            if let Some(e) = clean[s..].find(']') {
                clean = format!("{}{}", &clean[..s], clean[s+e+1..].trim_start());
            } else { break; }
        }
        if clean.trim() != reply.trim() {
            warn!("PC Safe Mode: blocked AI excel tags");
        }
        return (clean.trim().to_string(), false);
    }

    let mut clean = reply.to_string();
    let mut applied = false;

    while let Some(start) = clean.find("[EXCEL:") {
        if let Some(end) = clean[start..].find(']') {
            let tag = &clean[start+7..start+end];
            let parts: Vec<&str> = tag.splitn(2, ">>>").collect();
            let path = parts.first().unwrap_or(&"");
            let instruction = parts.get(1).unwrap_or(&"");
            usb_send_pc_cmd("excel_pipeline", &serde_json::json!({
                "path": path,
                "instruction": instruction,
            }), chat_id);
            applied = true;
            clean = format!("{}{}", &clean[..start], clean[start+end+1..].trim_start());
        } else {
            break;
        }
    }

    (clean.trim().to_string(), applied)
}

// ===== Bot Info =====

fn fetch_bot_username(token: &str) -> Result<String> {
    let url = format!("https://api.telegram.org/bot{}/getMe", token);
    let resp = http_get(&url)?;
    // parse "username":"xxx" from JSON
    if let Some(pos) = resp.find("\"username\"") {
        let rest = &resp[pos + 10..];
        if let Some(start) = rest.find('"') {
            let rest2 = &rest[start + 1..];
            if let Some(end) = rest2.find('"') {
                return Ok(rest2[..end].to_string());
            }
        }
    }
    bail!("username not found in getMe response");
}

// ===== OTA System =====

fn perform_ota(firmware: &[u8]) -> Result<()> {
    if firmware.len() < 1024 {
        bail!("Firmware too small ({} bytes)", firmware.len());
    }
    if firmware.len() > 4 * 1024 * 1024 {
        bail!("Firmware too large ({} bytes)", firmware.len());
    }

    info!("OTA starting: {} bytes", firmware.len());

    unsafe {
        let partition = esp_idf_svc::sys::esp_ota_get_next_update_partition(std::ptr::null());
        if partition.is_null() {
            bail!("No OTA partition found");
        }

        let mut handle: u32 = 0;
        let err = esp_idf_svc::sys::esp_ota_begin(
            partition,
            firmware.len(),
            &mut handle as *mut u32 as *mut _,
        );
        if err != 0 {
            bail!("esp_ota_begin failed: err={}", err);
        }

        info!("OTA writing to partition...");
        for (i, chunk) in firmware.chunks(4096).enumerate() {
            let err = esp_idf_svc::sys::esp_ota_write(
                handle,
                chunk.as_ptr() as *const core::ffi::c_void,
                chunk.len(),
            );
            if err != 0 {
                bail!("OTA write chunk {} failed: err={}", i, err);
            }
            if (i + 1) % 100 == 0 {
                info!("OTA: {}KB / {}KB", (i + 1) * 4, firmware.len() / 1024);
            }
        }

        let err = esp_idf_svc::sys::esp_ota_end(handle);
        if err != 0 {
            bail!("OTA validation failed: err={}", err);
        }

        let err = esp_idf_svc::sys::esp_ota_set_boot_partition(partition);
        if err != 0 {
            bail!("Set boot partition failed: err={}", err);
        }

        info!("OTA complete! Ready to reboot.");
    }

    Ok(())
}

fn handle_ota_url(cfg: &Config, chat_id: i64, url: &str) {
    if !url.starts_with("https://") {
        let _ = send_telegram(&cfg.tg_token, chat_id, "⚠️ 安全限制：OTA 只接受 HTTPS URL。\nUsage: /ota https://...");
        return;
    }

    let _ = send_telegram(&cfg.tg_token, chat_id,
        &format!("\u{2b07}\u{fe0f} Downloading firmware...\n{}", url));

    match http_get_binary(url) {
        Ok(data) => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("Downloaded {} bytes. Starting OTA...", data.len()));
            match perform_ota(&data) {
                Ok(()) => {
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("\u{2705} OTA success! {} bytes\nRebooting in 3s...", data.len()));
                    std::thread::sleep(Duration::from_secs(3));
                    unsafe { esp_idf_svc::sys::esp_restart(); }
                }
                Err(e) => {
                    let _ = send_telegram(&cfg.tg_token, chat_id,
                        &format!("\u{274c} OTA failed: {}", e));
                }
            }
        }
        Err(e) => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("Download failed: {}", e));
        }
    }
}

fn handle_ota_document(cfg: &Config, chat_id: i64, file_id: &str, file_name: &str) {
    let _ = send_telegram(&cfg.tg_token, chat_id,
        &format!("\u{1f4e6} Firmware: {}\nDownloading...", file_name));

    let file_url = format!("{}{}/getFile?file_id={}", TELEGRAM_API, cfg.tg_token, file_id);
    let file_resp = match http_get(&file_url) {
        Ok(r) => r,
        Err(e) => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("getFile failed: {}", e));
            return;
        }
    };

    let file_info: TgFileResponse = match serde_json::from_str(&file_resp) {
        Ok(f) => f,
        Err(e) => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Parse error: {}", e));
            return;
        }
    };

    let file_path = match file_info.result.and_then(|r| r.file_path) {
        Some(p) => p,
        None => {
            let _ = send_telegram(&cfg.tg_token, chat_id, "File path not found");
            return;
        }
    };

    let download_url = format!("{}{}/{}", TELEGRAM_FILE_API, cfg.tg_token, file_path);
    let data = match http_get_binary(&download_url) {
        Ok(d) => d,
        Err(e) => {
            let _ = send_telegram(&cfg.tg_token, chat_id, &format!("Download failed: {}", e));
            return;
        }
    };

    let _ = send_telegram(&cfg.tg_token, chat_id,
        &format!("Downloaded {} bytes. Starting OTA...", data.len()));

    match perform_ota(&data) {
        Ok(()) => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("\u{2705} OTA success! {} bytes\nRebooting in 3s...", data.len()));
            std::thread::sleep(Duration::from_secs(3));
            unsafe { esp_idf_svc::sys::esp_restart(); }
        }
        Err(e) => {
            let _ = send_telegram(&cfg.tg_token, chat_id,
                &format!("\u{274c} OTA failed: {}", e));
        }
    }
}

// ===== Gemini API =====

/// Gate check before any Gemini API call.
/// Returns Ok(()) if allowed, Err with user-facing message if blocked.
fn gemini_gate() -> Result<()> {
    if PAUSED.load(Ordering::Relaxed) {
        bail!("⏸️ 系統已暫停，所有 AI 呼叫已停止。\n傳送 /resume 恢復。");
    }
    let now = now_epoch() as u32; // lower 32 bits, fine for 60s windows
    let win_start = GEMINI_WINDOW_START.load(Ordering::Relaxed);
    if now.wrapping_sub(win_start) >= GEMINI_RATE_WINDOW_SECS as u32 {
        // New window
        GEMINI_WINDOW_START.store(now, Ordering::Relaxed);
        GEMINI_CALL_COUNT.store(1, Ordering::Relaxed);
        Ok(())
    } else {
        let count = GEMINI_CALL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count > MAX_GEMINI_CALLS_PER_WINDOW {
            GEMINI_CALL_COUNT.fetch_sub(1, Ordering::Relaxed); // undo
            let remaining = (GEMINI_RATE_WINDOW_SECS as u32).saturating_sub(now.wrapping_sub(win_start));
            bail!("🚫 已達到速率限制（每 {}秒 最多 {} 次 AI 呼叫）。\n請等待 {} 秒後再試。",
                GEMINI_RATE_WINDOW_SECS, MAX_GEMINI_CALLS_PER_WINDOW, remaining);
        }
        Ok(())
    }
}

fn ask_gemini_with_context(api_key: &str, question: &str, state: &mut AppState) -> Result<String> {
    gemini_gate()?;

    // Save token counts before this request for accurate per-request tracking
    state.last_tokens_in = state.tokens_in;
    state.last_tokens_out = state.tokens_out;

    let url = format!("{}/{}:generateContent?key={}", GEMINI_API, state.current_model, api_key);

    let mut contents: Vec<GeminiContent> = Vec::new();
    for (role, text) in &state.history {
        contents.push(GeminiContent {
            role: role.clone(),
            parts: vec![GeminiPartOut::Text { text: text.clone() }],
        });
    }
    contents.push(GeminiContent {
        role: "user".into(),
        parts: vec![GeminiPartOut::Text { text: question.to_string() }],
    });

    let mut sys_text = build_system_prompt(state);
    if !state.memories.is_empty() {
        sys_text.push_str("\n\nUser's memories:\n");
        for (k, v) in &state.memories {
            sys_text.push_str(&format!("- {}: {}\n", k, v));
        }
    }
    // Append skills to system prompt
    sys_text.push_str(&format_skills_for_prompt(&state.skills));
    // Append recent diagnostics for self-awareness
    sys_text.push_str(&state.diag.format_for_gemini());

    let request = GeminiRequest {
        system_instruction: GeminiSysInstr {
            parts: vec![GeminiTextPart { text: sys_text }],
        },
        contents,
        generation_config: GenConfig {
            max_output_tokens: 4096,   // 從 2048 升至 4096，防止長回覆被截斷
temperature: 0.3,
        },
    };

    let json_body = serde_json::to_string(&request)?;
    let resp_body = http_post(&url, &json_body)?;
    let resp: GeminiResponse = serde_json::from_str(&resp_body).map_err(|e| {
        error!("Gemini parse err: {} body: {}", e,
            &resp_body[..resp_body.len().min(200)]);
        e
    })?;

    if let Some(usage) = &resp.usage_metadata {
        state.tokens_in += usage.prompt_token_count.unwrap_or(0);
        state.tokens_out += usage.candidates_token_count.unwrap_or(0);
        state.requests += 1;
    }

    let answer = resp.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_else(|| "...".to_string());

    state.add_history("user", question);
    state.add_history("model", &answer);

    Ok(answer)
}

fn ask_gemini_with_image_context(
    api_key: &str,
    question: &str,
    image_data: &[u8],
    mime_type: &str,
    state: &mut AppState,
) -> Result<String> {
    gemini_gate()?;
    state.last_tokens_in = state.tokens_in;
    state.last_tokens_out = state.tokens_out;

    let url = format!("{}/{}:generateContent?key={}", GEMINI_API, state.current_model, api_key);

    let mut contents: Vec<GeminiContent> = Vec::new();
    for (role, text) in &state.history {
        contents.push(GeminiContent {
            role: role.clone(),
            parts: vec![GeminiPartOut::Text { text: text.clone() }],
        });
    }
    contents.push(GeminiContent {
        role: "user".into(),
        parts: vec![
            GeminiPartOut::Text {
                text: question.to_string(),
            },
            GeminiPartOut::InlineData {
                inline_data: InlineData {
                    mime_type: mime_type.to_string(),
                    data: base64_encode(image_data),
                },
            },
        ],
    });

    let mut sys_text = build_system_prompt(state);
    sys_text.push_str("\n\nThe user may attach Telegram photos. When an image is present, analyze the actual image content before answering.");
    if !state.memories.is_empty() {
        sys_text.push_str("\n\nUser's memories:\n");
        for (k, v) in &state.memories {
            sys_text.push_str(&format!("- {}: {}\n", k, v));
        }
    }
    sys_text.push_str(&format_skills_for_prompt(&state.skills));
    sys_text.push_str(&state.diag.format_for_gemini());

    let request = GeminiRequest {
        system_instruction: GeminiSysInstr {
            parts: vec![GeminiTextPart { text: sys_text }],
        },
        contents,
        generation_config: GenConfig {
            max_output_tokens: 4096,   // 從 2048 升至 4096，防止視覺回覆被中斷
            temperature: 0.5,
        },
    };

    let json_body = serde_json::to_string(&request)?;
    let resp_body = http_post(&url, &json_body)?;
    let resp: GeminiResponse = serde_json::from_str(&resp_body).map_err(|e| {
        error!("Gemini image parse err: {} body: {}", e, &resp_body[..resp_body.len().min(200)]);
        e
    })?;

    if let Some(usage) = &resp.usage_metadata {
        state.tokens_in += usage.prompt_token_count.unwrap_or(0);
        state.tokens_out += usage.candidates_token_count.unwrap_or(0);
        state.requests += 1;
    }

    let answer = resp.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_else(|| "...".to_string());

    state.add_history("user", &format!("[photo] {}", question));
    state.add_history("model", &answer);

    Ok(answer)
}

fn transcribe_audio_with_mime(api_key: &str, audio_data: &[u8], mime_type: &str, state: &mut AppState) -> Result<String> {
    gemini_gate()?;
    let url = format!("{}/{}:generateContent?key={}", GEMINI_API, TRANSCRIBE_MODEL, api_key);

    let b64 = base64_encode(audio_data);

    let request = GeminiRequest {
        system_instruction: GeminiSysInstr {
            parts: vec![GeminiTextPart {
                text: "You are a speech transcriber. Output ONLY the transcribed text. No extra words.".into(),
            }],
        },
        contents: vec![GeminiContent {
            role: "user".into(),
            parts: vec![
                GeminiPartOut::InlineData {
                    inline_data: InlineData {
                        mime_type: mime_type.into(),
                        data: b64,
                    },
                },
                GeminiPartOut::Text {
                    text: "Transcribe this voice message.".into(),
                },
            ],
        }],
        generation_config: GenConfig {
            max_output_tokens: 512,
            temperature: 0.1,
        },
    };

    let json_body = serde_json::to_string(&request)?;
    let resp_body = http_post(&url, &json_body)?;
    let resp: GeminiResponse = serde_json::from_str(&resp_body)?;

    if let Some(usage) = &resp.usage_metadata {
        state.tokens_in += usage.prompt_token_count.unwrap_or(0);
        state.tokens_out += usage.candidates_token_count.unwrap_or(0);
        state.requests += 1;
    }

    resp.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .ok_or_else(|| anyhow::anyhow!("No transcription"))
}

fn transcribe_audio(api_key: &str, audio_data: &[u8], state: &mut AppState) -> Result<String> {
    transcribe_audio_with_mime(api_key, audio_data, "audio/ogg", state)
}

fn telegram_get_file_path(token: &str, file_id: &str) -> Result<String> {
    let file_url = format!("{}{}/getFile?file_id={}", TELEGRAM_API, token, file_id);
    let file_resp = http_get(&file_url)?;
    let file_info: TgFileResponse = serde_json::from_str(&file_resp)?;
    if !file_info.ok {
        bail!("Telegram getFile returned not ok");
    }
    file_info
        .result
        .and_then(|r| r.file_path)
        .ok_or_else(|| anyhow::anyhow!("Telegram file path not found"))
}

fn download_telegram_file(token: &str, file_id: &str) -> Result<Vec<u8>> {
    let file_path = telegram_get_file_path(token, file_id)?;
    let download_url = format!("{}{}/{}", TELEGRAM_FILE_API, token, file_path);
    http_get_binary(&download_url)
}

fn infer_media_mime(file_name: Option<&str>, mime_type: Option<&str>) -> Option<String> {
    if let Some(mime) = mime_type {
        return Some(mime.to_string());
    }
    let ext = file_name
        .and_then(|name| name.rsplit('.').next())
        .map(|ext| ext.to_ascii_lowercase())?;
    match ext.as_str() {
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "png" => Some("image/png".to_string()),
        "webp" => Some("image/webp".to_string()),
        "ogg" | "oga" => Some("audio/ogg".to_string()),
        "mp3" => Some("audio/mpeg".to_string()),
        "wav" => Some("audio/wav".to_string()),
        "m4a" => Some("audio/mp4".to_string()),
        _ => None,
    }
}

fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*byte as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn sanitize_speech_text(text: &str) -> String {
    let first = text.split("\n[").next().unwrap_or(text);
    let cleaned = first
        .replace('\n', " ")
        .replace('*', " ")
        .replace('`', " ")
        .replace('#', " ")
        .replace('_', " ");
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(TTS_MAX_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

fn tts_download_mp3(nvs: &EspNvs<NvsDefault>, text: &str) -> Result<Vec<u8>> {
    let speech = sanitize_speech_text(text);
    if speech.is_empty() {
        bail!("speech text is empty");
    }

    let mp3 = if let Some(proxy_base) = load_tts_proxy_url(nvs) {
        let body = serde_json::json!({
            "text": speech,
            "voice": load_tts_proxy_voice(nvs),
            "format": "json",
        });
        let json_body = serde_json::to_string(&body)?;
        info!("TTS proxy download: {} chars via {}", speech.chars().count(), proxy_base);
        let resp_body = http_post(&proxy_base, &json_body)?;
        let resp: TtsProxyJsonResponse = serde_json::from_str(&resp_body)?;
        if !resp.ok {
            bail!("tts proxy error: {}", resp.error.unwrap_or_else(|| "unknown proxy error".to_string()));
        }
        let audio_b64 = resp.audio_b64.ok_or_else(|| anyhow!("tts proxy missing audio_b64"))?;
        base64::engine::general_purpose::STANDARD.decode(audio_b64)?
    } else {
        info!("TTS fallback download: {} chars", speech.chars().count());
        // Try multiple Google TTS client IDs to reduce rate-limit failures
        let clients = ["tw-ob", "gtx", "t"];
        let mut mp3_result: Result<Vec<u8>> = Err(anyhow::anyhow!("TTS: no clients tried"));
        for client in &clients {
            let url = format!(
                "https://translate.google.com/translate_tts?ie=UTF-8&client={}&tl=zh-TW&ttsspeed=0.88&q={}",
                client,
                url_encode(&speech)
            );
            let try_result = http_get_binary_with_headers(
                &url,
                &[
                    ("User-Agent", "Mozilla/5.0 (Linux; Android 10) AppleWebKit/537.36 ETHAN/4.7"),
                    ("Accept", "audio/mpeg,audio/*;q=0.9,*/*;q=0.5"),
                    ("Referer", "https://translate.google.com/"),
                    ("Accept-Language", "zh-TW,zh;q=0.9,en;q=0.7"),
                ],
            );
            match &try_result {
                Ok(data) if data.len() > 100 => {
                    info!("TTS client='{}' OK: {} bytes", client, data.len());
                    mp3_result = try_result;
                    break;
                }
                Ok(data) => warn!("TTS client='{}' got {} bytes (too small), trying next", client, data.len()),
                Err(e) => warn!("TTS client='{}' failed: {:?}, trying next", client, e),
            }
        }
        mp3_result?
    };

    info!("TTS download OK: {} bytes", mp3.len());
    Ok(mp3)
}

fn decode_mp3_to_pcm(mp3_data: &[u8]) -> Result<(Vec<i16>, u32, usize)> {
    let mut decoder = Mp3Decoder::new();
    let mut pcm = Vec::new();
    let mut sample_rate = 0u32;
    let mut channels = 1usize;
    let mut offset = 0usize;
    let mut frame_pcm = vec![0f32; MP3_MAX_SAMPLES_PER_FRAME];
    let mut frame_count = 0usize;

    while offset < mp3_data.len() {
        let (consumed, info) = decoder.decode(&mp3_data[offset..], &mut frame_pcm[..]);
        if consumed == 0 {
            break;
        }
        offset += consumed;

        if let Some(info) = info {
            frame_count += 1;
            if sample_rate == 0 {
                sample_rate = info.sample_rate.max(8_000);
                channels = info.channels.num() as usize;
            }
            for sample in frame_pcm[..info.samples_produced].iter() {
                let scaled = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                pcm.push(scaled);
            }
        }
    }

    if pcm.is_empty() || sample_rate == 0 {
        bail!("decoded mp3 produced no pcm samples");
    }

    info!(
        "TTS decode OK: {} frames, {} samples, {} Hz, {} ch",
        frame_count,
        pcm.len(),
        sample_rate,
        channels
    );

    Ok((pcm, sample_rate, channels))
}

fn normalize_tts_pcm_for_board(
    pcm: &[i16],
    input_rate: u32,
    input_channels: usize,
    target_rate: u32,
) -> Result<(Vec<i16>, u32, usize)> {
    if pcm.is_empty() {
        bail!("tts pcm is empty");
    }

    let channels = input_channels.max(1);
    let mono = if channels == 1 {
        pcm.to_vec()
    } else {
        let mut mixed = Vec::with_capacity(pcm.len() / channels.max(1));
        for frame in pcm.chunks(channels) {
            let sum: i32 = frame.iter().map(|sample| *sample as i32).sum();
            mixed.push((sum / frame.len() as i32) as i16);
        }
        mixed
    };

    let target = target_rate.max(8_000);
    let resampled = if input_rate == target {
        mono
    } else {
        let out_len = ((mono.len() as u64 * target as u64) / input_rate.max(1) as u64).max(1) as usize;
        let mut out = Vec::with_capacity(out_len);
        for idx in 0..out_len {
            let src_pos = idx as f32 * input_rate as f32 / target as f32;
            let base = src_pos.floor() as usize;
            let frac = src_pos - base as f32;
            let current = mono.get(base).copied().unwrap_or_default() as f32;
            let next = mono.get(base + 1).copied().unwrap_or(current as i16) as f32;
            let interpolated = current + ((next - current) * frac);
            out.push(interpolated as i16);
        }
        out
    };

    let gain = 0.42f32;
    let shaped = resampled
        .into_iter()
        .map(|sample| ((sample as f32 * gain).clamp(i16::MIN as f32, i16::MAX as f32)) as i16)
        .collect::<Vec<_>>();

    info!(
        "TTS normalized for board: {} -> {} Hz, {} ch -> mono, {} samples",
        input_rate,
        target,
        channels,
        shaped.len()
    );

    Ok((shaped, target, 1))
}

fn pcm16_to_wav_bytes(samples: &[i16], sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample = 16u16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

fn speak_onboard_text(nvs: &EspNvs<NvsDefault>, text: &str) -> Result<String> {
    let speech = sanitize_speech_text(text);
    if speech.is_empty() {
        bail!("speech text is empty");
    }

    let audio_pins = load_audio_pins(nvs);
    let key = tts_cache_key(&speech);

    // Try TTS cache first
    let cached = TTS_CACHE.lock().ok().and_then(|c| {
        c.iter().find(|(k, _)| *k == key).map(|(_, mp3)| mp3.clone())
    });

    let mp3 = if let Some(mp3) = cached {
        info!("TTS cache hit: {} chars", speech.chars().count());
        mp3
    } else {
        info!("TTS download: {} chars", speech.chars().count());
        let mp3 = tts_download_mp3(nvs, &speech)?;
        if let Ok(mut cache) = TTS_CACHE.lock() {
            if cache.len() >= TTS_CACHE_MAX {
                cache.remove(0);
            }
            cache.push((key, mp3.clone()));
        }
        mp3
    };

    let (pcm, sample_rate, channels) = decode_mp3_to_pcm(&mp3)?;
    let (pcm, sample_rate, channels) = normalize_tts_pcm_for_board(
        &pcm,
        sample_rate,
        channels,
        audio_pins.sample_rate,
    )?;

    info!("TTS playback start: {} pcm samples", pcm.len());
    play_pcm16(&audio_pins, &pcm, sample_rate, channels)?;
    info!("TTS playback done");
    Ok(speech)
}

fn capture_and_transcribe_mic(
    cfg: &Config,
    nvs: &EspNvs<NvsDefault>,
    state: &mut AppState,
    duration_ms: u32,
) -> Result<String> {
    let audio_pins = load_audio_pins(nvs);
    let pcm = capture_mic_pcm(
        &audio_pins,
        duration_ms,
        audio_pins.sample_rate as usize * duration_ms as usize / 1000 + 2048,
    )?;
    let wav = pcm16_to_wav_bytes(&pcm, audio_pins.sample_rate, 1);
    transcribe_audio_with_mime(&cfg.gemini_key, &wav, "audio/wav", state)
}

fn normalize_for_match(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| ch.to_lowercase())
        .filter(|ch| !ch.is_whitespace() && !ch.is_ascii_punctuation())
        .collect()
}

fn should_speak_onboard(mode: &str) -> bool {
    matches!(mode, "normal" | "brief")
}

fn speech_text_for_mode(mode: &str, text: &str) -> String {
    let sanitized = sanitize_speech_text(text);
    if mode == "brief" {
        // Smart truncation: take first sentence or first 40 chars
        let first_sentence_end = sanitized.find(|c| c == '\u{3002}' || c == '\u{ff01}' || c == '\u{ff1f}' || c == '.' || c == '!' || c == '?');
        if let Some(pos) = first_sentence_end {
            let end = sanitized.ceil_char_boundary(pos + 3).min(sanitized.len());
            if end <= 60 {
                return sanitized[..end].to_string();
            }
        }
        sanitized.chars().take(40).collect::<String>()
    } else {
        sanitized
    }
}

fn detect_wake_phrase(
    cfg: &Config,
    nvs: &EspNvs<NvsDefault>,
    state: &mut AppState,
    duration_ms: u32,
) -> Result<Option<String>> {
    let audio_pins = load_audio_pins(nvs);
    let max_samples = audio_pins.sample_rate as usize * duration_ms as usize / 1000 + 2048;
    let pcm = capture_mic_pcm(&audio_pins, duration_ms, max_samples)?;

    // Try ESP-SR WakeNet first (local, free, fast)
    let has_sr = WAKENET.lock().ok().map_or(false, |g| g.is_some());
    if has_sr {
        let pcm16k = if audio_pins.sample_rate != 16000 {
            resample_linear(&pcm, audio_pins.sample_rate, 16000)
        } else {
            pcm.clone()
        };
        if let Some(word) = detect_wake_sr(&pcm16k) {
            return Ok(Some(word));
        }
        // WakeNet only recognises its built-in phrase (Hi,ESP).
        // Fall through to Gemini STT with the SAME audio for custom phrases.
    }

    // RMS silence check: skip Gemini API call if audio is too quiet (no speech)
    let energy: u64 = pcm.iter().map(|&s| (s as i64 * s as i64) as u64).sum();
    let rms = if !pcm.is_empty() {
        ((energy / pcm.len() as u64) as f64).sqrt() as u32
    } else {
        0
    };
    if rms < 800 {
        return Ok(None); // Silence ??no need to call Gemini
    }

    // Cloud fallback: reuse captured audio for Gemini STT transcription
    let wav = pcm16_to_wav_bytes(&pcm, audio_pins.sample_rate, 1);
    let transcript = transcribe_audio_with_mime(&cfg.gemini_key, &wav, "audio/wav", state)?;
    if transcript.trim().is_empty() {
        return Ok(None);
    }
    let heard = normalize_for_match(&transcript);
    let target = normalize_for_match(&state.wake_phrase);
    if !target.is_empty() && heard.contains(&target) {
        Ok(Some(transcript))
    } else {
        Ok(None)
    }
}

/// Simple linear interpolation resampling
fn resample_linear(pcm: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate || pcm.is_empty() { return pcm.to_vec(); }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((pcm.len() as f64) / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let idx = src as usize;
        let frac = src - idx as f64;
        let s0 = pcm[idx] as f64;
        let s1 = if idx + 1 < pcm.len() { pcm[idx + 1] as f64 } else { s0 };
        out.push((s0 + frac * (s1 - s0)) as i16);
    }
    out
}

fn describe_image_with_gemini(api_key: &str, state: &mut AppState, jpeg_data: &[u8], prompt: &str) -> Result<String> {
    gemini_gate()?;
    let url = format!("{}/{}:generateContent?key={}", GEMINI_API, state.current_model, api_key);
    let b64 = base64_encode(jpeg_data);

    let request = GeminiRequest {
        system_instruction: GeminiSysInstr {
            parts: vec![GeminiTextPart {
                text: "你是 ETHAN ── Gemini Vision 模組，請用繁體中文回覆。請描述畫面中的人物、物件、文字，並提供執行建議。".into(),
            }],
        },
        contents: vec![GeminiContent {
            role: "user".into(),
            parts: vec![
                GeminiPartOut::Text {
                    text: prompt.to_string(),
                },
                GeminiPartOut::InlineData {
                    inline_data: InlineData {
                        mime_type: "image/jpeg".into(),
                        data: b64,
                    },
                },
            ],
        }],
        generation_config: GenConfig {
            max_output_tokens: 768,
            temperature: 0.4,
        },
    };

    let json_body = serde_json::to_string(&request)?;
    let resp_body = http_post(&url, &json_body)?;
    let resp: GeminiResponse = serde_json::from_str(&resp_body)?;

    if let Some(usage) = &resp.usage_metadata {
        state.tokens_in += usage.prompt_token_count.unwrap_or(0);
        state.tokens_out += usage.candidates_token_count.unwrap_or(0);
        state.requests += 1;
    }

    resp.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .ok_or_else(|| anyhow::anyhow!("No vision response"))
}

// ===== HTTP =====

fn new_http_config(timeout_secs: u64) -> HttpConfig {
    HttpConfig {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        timeout: Some(Duration::from_secs(timeout_secs)),
        buffer_size: Some(16384),      // 加大：Gemini JSON 回覆可達 20-30KB
        buffer_size_tx: Some(8192),    // 加大：system prompt + history 輸出
        ..Default::default()
    }
}

fn http_get(url: &str) -> Result<String> {
    let mut client = EspHttpConnection::new(&new_http_config(30))?;
    client.initiate_request(esp_idf_svc::http::Method::Get, url, &[])?;
    client.initiate_response()?;

    let mut body = Vec::with_capacity(4096);
    let mut buf = [0u8; 2048];
    loop {
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(e) => { warn!("read: {:?}", e); break; }
        }
    }
    Ok(String::from_utf8_lossy(&body).to_string())
}

fn http_get_binary(url: &str) -> Result<Vec<u8>> {
    http_get_binary_with_headers(url, &[])
}

fn http_get_binary_with_headers(url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>> {
    info!("HTTP GET binary: create client");
    let mut client = EspHttpConnection::new(&new_http_config(120))?;
    // Mask sensitive tokens in URL for logging
    let safe_url = if url.contains("/bot") {
        url.split("/bot").next().unwrap_or("").to_string() + "/bot***"
    } else {
        url.to_string()
    };
    info!("HTTP GET binary: initiate request {}", safe_url);
    client.initiate_request(esp_idf_svc::http::Method::Get, url, headers)?;
    info!("HTTP GET binary: await response");
    client.initiate_response()?;

    let mut body = Vec::with_capacity(64 * 1024);
    let mut buf = [0u8; 4096];
    info!("HTTP GET binary: reading body");
    loop {
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&buf[..n]);
                if body.len() > 4 * 1024 * 1024 {
                    bail!("File too large (>4MB)");
                }
            }
            Err(e) => { warn!("read: {:?}", e); break; }
        }
    }
    info!("HTTP GET binary: complete {} bytes", body.len());
    Ok(body)
}

fn http_post(url: &str, json_body: &str) -> Result<String> {
    let body_bytes = json_body.as_bytes();
    let content_len = body_bytes.len().to_string();

    let mut client = EspHttpConnection::new(&new_http_config(60))?;
    client.initiate_request(
        esp_idf_svc::http::Method::Post,
        url,
        &[
            ("Content-Type", "application/json"),
            ("Content-Length", &content_len),
        ],
    )?;
    client.write_all(body_bytes)?;
    client.initiate_response()?;

    let mut body = Vec::with_capacity(8192);
    let mut buf = [0u8; 2048];
    loop {
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(e) => { warn!("read: {:?}", e); break; }
        }
    }
    Ok(String::from_utf8_lossy(&body).to_string())
}

fn http_post_bytes(url: &str, headers: &[(&str, &str)], body_bytes: &[u8]) -> Result<String> {
    let content_len = body_bytes.len().to_string();
    let mut request_headers: Vec<(&str, &str)> = headers.to_vec();
    request_headers.push(("Content-Length", &content_len));

    let mut client = EspHttpConnection::new(&new_http_config(90))?;
    client.initiate_request(
        esp_idf_svc::http::Method::Post,
        url,
        &request_headers,
    )?;
    client.write_all(body_bytes)?;
    client.initiate_response()?;

    let mut body = Vec::with_capacity(8192);
    let mut buf = [0u8; 2048];
    loop {
        match client.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&buf[..n]),
            Err(e) => {
                warn!("read: {:?}", e);
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&body).to_string())
}


// ===== Telegram =====

fn poll_telegram(token: &str, offset: i64) -> Result<Vec<TgUpdate>> {
    let url = format!(
        "{}{}/getUpdates?offset={}&timeout=10&allowed_updates=[\"message\"]",
        TELEGRAM_API, token, offset
    );
    let body = http_get(&url)?;
    let resp: TgResponse = serde_json::from_str(&body)?;
    if !resp.ok {
        bail!("Telegram API error");
    }
    Ok(resp.result.unwrap_or_default())
}

fn prepare_telegram_polling(token: &str, nvs: &EspNvs<NvsDefault>) -> Result<i64> {
    let delete_url = format!("{}{}/deleteWebhook?drop_pending_updates=false", TELEGRAM_API, token);
    let _ = http_get(&delete_url)?;
    Ok(load_last_telegram_update_id(nvs))
}

fn split_telegram_text(text: &str, max_len: usize) -> Vec<String> {
    if text.chars().count() <= max_len {
        return vec![text.to_string()];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < chars.len() {
        let mut end = (start + max_len).min(chars.len());
        if end < chars.len() {
            for idx in (start..end).rev() {
                if chars[idx].is_whitespace() || chars[idx] == '\n' {
                    end = idx + 1;
                    break;
                }
            }
            if end <= start {
                end = (start + max_len).min(chars.len());
            }
        }
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk.trim().to_string());
        start = end;
    }
    chunks.retain(|chunk| !chunk.is_empty());
    chunks
}

fn send_telegram(token: &str, chat_id: i64, text: &str) -> Result<()> {
    let url = format!("{}{}/sendMessage", TELEGRAM_API, token);
    for chunk in split_telegram_text(text, 3800) {
        let payload = serde_json::json!({
            "chat_id": chat_id,
            "text": chunk
        });
        let resp = http_post(&url, &payload.to_string())?;
        ensure_telegram_api_ok(&resp)?;
        info!("Sent ({}B)", resp.len());
        std::thread::sleep(Duration::from_millis(120));
    }
    Ok(())
}

fn send_telegram_camera_jpeg(token: &str, chat_id: i64, jpeg: &[u8], caption: &str) -> Result<()> {
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=3 {
        match send_telegram_photo_jpeg(token, chat_id, jpeg, caption) {
            Ok(()) => return Ok(()),
            Err(photo_err) => {
                warn!("sendPhoto attempt {} failed: {}", attempt, photo_err);
                match send_telegram_document_jpeg(token, chat_id, jpeg, caption) {
                    Ok(()) => return Ok(()),
                    Err(doc_err) => {
                        warn!("sendDocument attempt {} failed: {}", attempt, doc_err);
                        last_err = Some(anyhow!("photo err: {}; document err: {}", photo_err, doc_err));
                        std::thread::sleep(Duration::from_millis(400 * attempt as u64));
                    }
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("telegram jpeg upload failed after retries")))
}

fn send_telegram_photo_jpeg(token: &str, chat_id: i64, jpeg: &[u8], caption: &str) -> Result<()> {
    let boundary = "----ethan-photo-boundary";
    let mut body = Vec::with_capacity(jpeg.len() + 1024);
    let chat_id_text = chat_id.to_string();
    let caption_text = if caption.len() > 900 { &caption[..900] } else { caption };

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"chat_id\"\r\n\r\n");
    body.extend_from_slice(chat_id_text.as_bytes());
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"caption\"\r\n\r\n");
    body.extend_from_slice(caption_text.as_bytes());
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"photo\"; filename=\"ethan.jpg\"\r\n");
    body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
    body.extend_from_slice(jpeg);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let url = format!("{}{}/sendPhoto", TELEGRAM_API, token);
    let content_type = format!("multipart/form-data; boundary={}", boundary);
    let headers = [("Content-Type", content_type.as_str())];
    let resp = http_post_bytes(&url, &headers, &body)?;
    ensure_telegram_api_ok(&resp)?;
    info!("Photo sent ({}B)", resp.len());
    Ok(())
}

fn send_telegram_document_jpeg(token: &str, chat_id: i64, jpeg: &[u8], caption: &str) -> Result<()> {
    let boundary = "----ethan-doc-boundary";
    let mut body = Vec::with_capacity(jpeg.len() + 1024);
    let chat_id_text = chat_id.to_string();
    let caption_text = if caption.len() > 900 { &caption[..900] } else { caption };

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"chat_id\"\r\n\r\n");
    body.extend_from_slice(chat_id_text.as_bytes());
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"caption\"\r\n\r\n");
    body.extend_from_slice(caption_text.as_bytes());
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"document\"; filename=\"ethan-camera.jpg\"\r\n");
    body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
    body.extend_from_slice(jpeg);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let url = format!("{}{}/sendDocument", TELEGRAM_API, token);
    let content_type = format!("multipart/form-data; boundary={}", boundary);
    let headers = [("Content-Type", content_type.as_str())];
    let resp = http_post_bytes(&url, &headers, &body)?;
    ensure_telegram_api_ok(&resp)?;
    info!("Document sent ({}B)", resp.len());
    Ok(())
}

// ===== Base64 =====

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

