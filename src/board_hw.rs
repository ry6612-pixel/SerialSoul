use anyhow::{anyhow, bail, Result};
use display_interface_spi::SPIInterface;
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X10};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle, Triangle};
use embedded_graphics::text::{Alignment, Text};
use embedded_hal::spi::MODE_0;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::{AnyIOPin, AnyOutputPin, PinDriver};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::i2s::config::{DataBitWidth, StdConfig};
use esp_idf_hal::i2s::{I2sDriver, I2sRx, I2sTx};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::{self, SpiDeviceDriver, SpiDriverConfig};
use esp_idf_hal::units::FromValueType;
use esp_idf_sys::{gpio_mode_t_GPIO_MODE_OUTPUT, gpio_reset_pin, gpio_set_direction, gpio_set_level};
use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use log::{info, warn};
use mipidsi::models::ST7789;
use mipidsi::options::{ColorOrder, Orientation};
use mipidsi::Builder;
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;
use std::thread;
use std::time::{Duration, Instant};

pub const LCD_NVS_KEY: &str = "lcd_pins";
pub const AUDIO_NVS_KEY: &str = "audio_pins";
const BOARD_IO_SDA_PIN: i32 = 1;
const BOARD_IO_SCL_PIN: i32 = 2;
const BOARD_IO_EXPANDER_ADDRS: [u8; 4] = [0x18, 0x19, 0x1A, 0x1B];
const ES8311_ADDRS: [u8; 2] = [0x18, 0x19];
const ES8311_RESET_REG00: u8 = 0x00;
const ES8311_CLK_MANAGER_REG01: u8 = 0x01;
const ES8311_CLK_MANAGER_REG02: u8 = 0x02;
const ES8311_CLK_MANAGER_REG03: u8 = 0x03;
const ES8311_CLK_MANAGER_REG04: u8 = 0x04;
const ES8311_CLK_MANAGER_REG05: u8 = 0x05;
const ES8311_CLK_MANAGER_REG06: u8 = 0x06;
const ES8311_CLK_MANAGER_REG07: u8 = 0x07;
const ES8311_CLK_MANAGER_REG08: u8 = 0x08;
const ES8311_SDPIN_REG09: u8 = 0x09;
const ES8311_SDPOUT_REG0A: u8 = 0x0A;
const ES8311_SYSTEM_REG0B: u8 = 0x0B;
const ES8311_SYSTEM_REG0C: u8 = 0x0C;
const ES8311_SYSTEM_REG0D: u8 = 0x0D;
const ES8311_SYSTEM_REG0E: u8 = 0x0E;
const ES8311_SYSTEM_REG10: u8 = 0x10;
const ES8311_SYSTEM_REG11: u8 = 0x11;
const ES8311_SYSTEM_REG12: u8 = 0x12;
const ES8311_SYSTEM_REG13: u8 = 0x13;
const ES8311_SYSTEM_REG14: u8 = 0x14;
const ES8311_ADC_REG15: u8 = 0x15;
const ES8311_ADC_REG16: u8 = 0x16;
const ES8311_ADC_REG17: u8 = 0x17;
const ES8311_ADC_REG1B: u8 = 0x1B;
const ES8311_ADC_REG1C: u8 = 0x1C;
const ES8311_DAC_REG31: u8 = 0x31;
const ES8311_DAC_REG32: u8 = 0x32;
const ES8311_DAC_REG37: u8 = 0x37;
const ES8311_GPIO_REG44: u8 = 0x44;
const ES8311_GP_REG45: u8 = 0x45;
const ES8311_CHD1_REGFD: u8 = 0xFD;
const ES8311_CHD2_REGFE: u8 = 0xFE;
const ES8311_CHVER_REGFF: u8 = 0xFF;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LcdPins {
    pub rst: i32,
    pub dc: i32,
    pub bl: i32,
    pub sclk: i32,
    pub sda: i32,
    pub cs: i32,
    pub width: u16,
    pub height: u16,
}

impl Default for LcdPins {
    fn default() -> Self {
        Self {
            rst: 21,
            dc: 47,
            bl: 38,
            sclk: 19,
            sda: 20,
            cs: 45,
            width: 240,
            height: 320,
        }
    }
}

impl LcdPins {
    pub fn status_text(&self) -> String {
        format!(
            "ST7789 pins\nrst={} dc={} bl={} sclk={} sda={} cs={}\nsize={}x{}",
            self.rst, self.dc, self.bl, self.sclk, self.sda, self.cs, self.width, self.height
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioPins {
    pub bclk: i32,
    pub ws: i32,
    pub dout: i32,
    pub din: i32,
    #[serde(default = "default_mic_sck_pin")]
    pub mic_sck: i32,
    #[serde(default = "default_mic_ws_pin")]
    pub mic_ws: i32,
    #[serde(default = "default_mic_din_pin")]
    pub mic_din: i32,
    pub mclk: i32,
    pub sample_rate: u32,
}

fn default_mic_sck_pin() -> i32 { 2 }
fn default_mic_ws_pin() -> i32 { 1 }
fn default_mic_din_pin() -> i32 { 42 }

impl Default for AudioPins {
    fn default() -> Self {
        Self {
            bclk: 40,
            ws: 41,
            dout: 39,
            din: 42,
            mic_sck: default_mic_sck_pin(),
            mic_ws: default_mic_ws_pin(),
            mic_din: default_mic_din_pin(),
            mclk: -1,
            sample_rate: 24_000,
        }
    }
}

impl AudioPins {
    pub fn status_text(&self) -> String {
        format!(
            "Speaker pins\nbclk={} ws={} dout={} mclk={}\nMic pins\nws={} sck={} din={}\nrate={}Hz",
            self.bclk, self.ws, self.dout, self.mclk, self.mic_ws, self.mic_sck, self.mic_din, self.sample_rate
        )
    }
}

#[derive(Clone, Debug)]
pub struct MicSnapshot {
    pub rms: u32,
    pub peak: i16,
    pub samples: usize,
}

pub fn load_lcd_pins(nvs: &EspNvs<NvsDefault>) -> LcdPins {
    super::nvs_get(nvs, LCD_NVS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_lcd_pins(nvs: &mut EspNvs<NvsDefault>, pins: &LcdPins) -> Result<()> {
    nvs.set_str(LCD_NVS_KEY, &serde_json::to_string(pins)?)?;
    Ok(())
}

pub fn parse_lcd_pin_args(input: &str, pins: &mut LcdPins) -> Result<()> {
    for token in input.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| anyhow!("bad lcd token: {}", token))?;
        match key {
            "rst" => pins.rst = value.parse()?,
            "dc" => pins.dc = value.parse()?,
            "bl" => pins.bl = value.parse()?,
            "sclk" => pins.sclk = value.parse()?,
            "sda" => pins.sda = value.parse()?,
            "cs" => pins.cs = value.parse()?,
            "width" => pins.width = value.parse()?,
            "height" => pins.height = value.parse()?,
            _ => bail!("unknown lcd field: {}", key),
        }
    }
    Ok(())
}

pub fn load_audio_pins(nvs: &EspNvs<NvsDefault>) -> AudioPins {
    super::nvs_get(nvs, AUDIO_NVS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_audio_pins(nvs: &mut EspNvs<NvsDefault>, pins: &AudioPins) -> Result<()> {
    nvs.set_str(AUDIO_NVS_KEY, &serde_json::to_string(pins)?)?;
    Ok(())
}

pub fn parse_audio_pin_args(input: &str, pins: &mut AudioPins) -> Result<()> {
    for token in input.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| anyhow!("bad audio token: {}", token))?;
        match key {
            "bclk" => pins.bclk = value.parse()?,
            "ws" => pins.ws = value.parse()?,
            "dout" => pins.dout = value.parse()?,
            "din" => {
                let parsed = value.parse()?;
                pins.din = parsed;
                pins.mic_din = parsed;
            }
            "mic_sck" | "mic_bclk" => pins.mic_sck = value.parse()?,
            "mic_ws" => pins.mic_ws = value.parse()?,
            "mic_din" => pins.mic_din = value.parse()?,
            "mclk" => pins.mclk = value.parse()?,
            "rate" | "sample_rate" => pins.sample_rate = value.parse()?,
            _ => bail!("unknown audio field: {}", key),
        }
    }
    Ok(())
}

pub fn show_boot_screen(pins: &LcdPins) -> Result<()> {
    draw_lcd_scene(pins, "ETHAN", "Booting hardware...")
}

pub fn show_ready_screen(pins: &LcdPins) -> Result<()> {
    draw_lcd_scene(pins, "ETHAN", "Telegram + Gemini ready")
}

pub fn show_reply_screen(pins: &LcdPins, text: &str) -> Result<()> {
    let clean = text.replace('\n', " ");
    let summary: String = clean.chars().take(36).collect();
    draw_lcd_scene(pins, "ETHAN", &summary)
}

pub fn draw_lcd_scene(pins: &LcdPins, headline: &str, subtitle: &str) -> Result<()> {
    ensure_pin(pins.dc, "lcd.dc")?;
    ensure_pin(pins.bl, "lcd.bl")?;
    ensure_pin(pins.sclk, "lcd.sclk")?;
    ensure_pin(pins.sda, "lcd.sda")?;

    if is_zhengchen_cam_lcd(pins) {
        if let Err(err) = configure_board_io_expander_bit(0, false) {
            warn!("LCD power enable via PCA9557 failed: {}", err);
        }
    }

    let peripherals = unsafe { Peripherals::steal() };
    let spi = peripherals.spi2;

    let dc = PinDriver::output(unsafe { AnyOutputPin::steal(pins.dc as u8) })?;
    let sclk = unsafe { AnyIOPin::steal(pins.sclk as u8) };
    let sda = unsafe { AnyIOPin::steal(pins.sda as u8) };

    let mut delay = Ets;
    let (panel_width, panel_height) = normalize_st7789_size(pins.width, pins.height);
    let config = spi::config::Config::new()
        .baudrate(26.MHz().into())
        .data_mode(MODE_0);

    let device = SpiDeviceDriver::new_single(
        spi,
        sclk,
        sda,
        Option::<AnyIOPin>::None,
        if pins.cs >= 0 {
            Some(unsafe { AnyIOPin::steal(pins.cs as u8) })
        } else {
            Option::<AnyIOPin>::None
        },
        &SpiDriverConfig::new(),
        &config,
    )?;

    let di = SPIInterface::new(device, dc);
    if pins.rst >= 0 {
        let rst = PinDriver::output(unsafe { AnyOutputPin::steal(pins.rst as u8) })?;
        let mut display = Builder::new(ST7789, di)
            .display_size(panel_width, panel_height)
            .orientation(Orientation::new())
            .color_order(ColorOrder::Bgr)
            .reset_pin(rst)
            .init(&mut delay)
            .map_err(|_| anyhow!("st7789 init failed"))?;
        set_lcd_backlight(pins)?;
        render_lcd_content(&mut display, headline, subtitle)?;
    } else {
        let mut display = Builder::new(ST7789, di)
            .display_size(panel_width, panel_height)
            .orientation(Orientation::new())
            .color_order(ColorOrder::Bgr)
            .init(&mut delay)
            .map_err(|_| anyhow!("st7789 init failed"))?;
        set_lcd_backlight(pins)?;
        render_lcd_content(&mut display, headline, subtitle)?;
    }

    info!("LCD scene rendered: {} | {}", headline, subtitle);
    Ok(())
}

fn render_lcd_content<D>(display: &mut D, headline: &str, subtitle: &str) -> Result<()>
where
    D: DrawTarget<Color = Rgb565> + OriginDimensions,
{
    let (headline, subtitle, expression) = parse_lcd_expression(headline, subtitle);
    let state = classify_lcd_state(&headline, &subtitle);
    let (bg, face_fill, face_stroke, eye_white_c, pupil_c, accent_c, blush_c, mouth_c, title_color, body_color, meta_color, state_label) =
        emoji_palette(state, expression);

    let face_style = PrimitiveStyleBuilder::new()
        .fill_color(face_fill)
        .stroke_color(face_stroke)
        .stroke_width(3)
        .build();
    let eye_white_style = PrimitiveStyleBuilder::new()
        .fill_color(eye_white_c)
        .stroke_color(face_stroke)
        .stroke_width(1)
        .build();
    let pupil_style = PrimitiveStyleBuilder::new()
        .fill_color(pupil_c)
        .stroke_color(pupil_c)
        .stroke_width(1)
        .build();
    let accent_style = PrimitiveStyleBuilder::new()
        .fill_color(accent_c)
        .stroke_color(accent_c)
        .stroke_width(1)
        .build();
    let blush_style = PrimitiveStyleBuilder::new()
        .fill_color(blush_c)
        .stroke_color(blush_c)
        .stroke_width(1)
        .build();
    let mouth_style = PrimitiveStyleBuilder::new()
        .fill_color(mouth_c)
        .stroke_color(mouth_c)
        .stroke_width(1)
        .build();
    let _bar_style = PrimitiveStyleBuilder::new()
        .fill_color(face_stroke)
        .stroke_color(face_stroke)
        .stroke_width(1)
        .build();
    let highlight_style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::new(31, 63, 31))
        .stroke_color(Rgb565::new(31, 63, 31))
        .stroke_width(1)
        .build();
    let title_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(title_color)
        .build();
    let body_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(body_color)
        .build();
    let meta_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(meta_color)
        .build();

    display.clear(bg).map_err(|_| anyhow!("lcd clear failed"))?;
    let size = display.bounding_box().size;
    let w = size.width as i32;
    let h = size.height as i32;

    // ===== Top status bar =====
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(4, 2), Size::new((w - 8) as u32, 20)),
        Size::new(10, 10),
    )
    .into_styled(PrimitiveStyleBuilder::new().fill_color(face_stroke).build())
    .draw(display)
    .map_err(|_| anyhow!("lcd bar draw failed"))?;
    Text::with_alignment("ETHAN", Point::new(32, 16), meta_style, Alignment::Center)
        .draw(display).map_err(|_| anyhow!("lcd text failed"))?;
    Text::with_alignment(state_label, Point::new(w / 2, 16), meta_style, Alignment::Center)
        .draw(display).map_err(|_| anyhow!("lcd text failed"))?;
    // Green dot
    Circle::new(Point::new(8, 8), 7)
        .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::new(8, 50, 8)).build())
        .draw(display).map_err(|_| anyhow!("lcd dot failed"))?;

    // ===== Main emoji face - large centered circle =====
    let face_cx = w / 2;
    let face_cy = (26 + h - 50) / 2;
    let face_r: i32 = if h >= 300 { 80 } else { 72 };

    // Shadow behind face
    Circle::new(Point::new(face_cx - face_r + 3, face_cy - face_r + 3), (face_r * 2) as u32)
        .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::new(
            (face_fill.r() as u16).saturating_sub(6) as u8,
            (face_fill.g() as u16).saturating_sub(12) as u8,
            (face_fill.b() as u16).saturating_sub(6) as u8,
        )).build())
        .draw(display).map_err(|_| anyhow!("lcd shadow failed"))?;

    // Main face circle
    Circle::new(Point::new(face_cx - face_r, face_cy - face_r), (face_r * 2) as u32)
        .into_styled(face_style)
        .draw(display).map_err(|_| anyhow!("lcd face failed"))?;

    // Highlight on face (top-left)
    Circle::new(Point::new(face_cx - 42, face_cy - 52), 24)
        .into_styled(highlight_style)
        .draw(display).map_err(|_| anyhow!("lcd highlight failed"))?;

    // ===== Expression-specific eyes and mouth =====
    let left_eye_cx = face_cx - 24;
    let right_eye_cx = face_cx + 24;
    let eye_cy = face_cy - 10;

    match expression {
        LcdExpression::Neutral => {
            // Round eyes
            Circle::new(Point::new(left_eye_cx - 12, eye_cy - 12), 24)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(right_eye_cx - 12, eye_cy - 12), 24)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            // Pupils
            Circle::new(Point::new(left_eye_cx - 5, eye_cy - 5), 10)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(right_eye_cx - 5, eye_cy - 5), 10)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            // Eye highlights
            Circle::new(Point::new(left_eye_cx - 2, eye_cy - 8), 4)
                .into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd hl failed"))?;
            Circle::new(Point::new(right_eye_cx - 2, eye_cy - 8), 4)
                .into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd hl failed"))?;
            // Blush
            Circle::new(Point::new(left_eye_cx - 22, eye_cy + 14), 12)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            Circle::new(Point::new(right_eye_cx + 10, eye_cy + 14), 12)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            // Small smile
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 10, face_cy + 22), Size::new(20, 6)),
                Size::new(3, 3),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
        }
        LcdExpression::Happy => {
            // Happy arc eyes (closed upward)
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(left_eye_cx - 12, eye_cy - 2), Size::new(24, 8)),
                Size::new(4, 4),
            ).into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(right_eye_cx - 12, eye_cy - 2), Size::new(24, 8)),
                Size::new(4, 4),
            ).into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            // Blush
            Circle::new(Point::new(left_eye_cx - 22, eye_cy + 10), 14)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            Circle::new(Point::new(right_eye_cx + 8, eye_cy + 10), 14)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            // Big smile
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 18, face_cy + 16), Size::new(36, 18)),
                Size::new(9, 9),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
            // Teeth highlight
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 12, face_cy + 16), Size::new(24, 8)),
                Size::new(4, 4),
            ).into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd teeth failed"))?;
        }
        LcdExpression::Wink => {
            // Left eye closed (wink line)
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(left_eye_cx - 12, eye_cy - 1), Size::new(24, 5)),
                Size::new(2, 2),
            ).into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            // Right eye open
            Circle::new(Point::new(right_eye_cx - 12, eye_cy - 12), 24)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(right_eye_cx - 5, eye_cy - 5), 10)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(right_eye_cx - 2, eye_cy - 8), 4)
                .into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd hl failed"))?;
            // Blush
            Circle::new(Point::new(left_eye_cx - 22, eye_cy + 12), 12)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            Circle::new(Point::new(right_eye_cx + 10, eye_cy + 12), 12)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            // Cheeky smile
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 14, face_cy + 18), Size::new(28, 12)),
                Size::new(6, 6),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
        }
        LcdExpression::Love => {
            // Heart eyes
            for cx in [left_eye_cx, right_eye_cx] {
                Circle::new(Point::new(cx - 8, eye_cy - 10), 14)
                    .into_styled(accent_style).draw(display).map_err(|_| anyhow!("lcd heart failed"))?;
                Circle::new(Point::new(cx + 2, eye_cy - 10), 14)
                    .into_styled(accent_style).draw(display).map_err(|_| anyhow!("lcd heart failed"))?;
                Triangle::new(
                    Point::new(cx - 10, eye_cy - 2),
                    Point::new(cx + 14, eye_cy - 2),
                    Point::new(cx + 2, eye_cy + 12),
                ).into_styled(accent_style).draw(display).map_err(|_| anyhow!("lcd heart failed"))?;
            }
            // Blush
            Circle::new(Point::new(left_eye_cx - 22, eye_cy + 14), 14)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            Circle::new(Point::new(right_eye_cx + 8, eye_cy + 14), 14)
                .into_styled(blush_style).draw(display).map_err(|_| anyhow!("lcd blush failed"))?;
            // Happy open mouth
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 12, face_cy + 18), Size::new(24, 16)),
                Size::new(8, 8),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
        }
        LcdExpression::Thinking => {
            // One eye squinting, one looking up
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(left_eye_cx - 12, eye_cy - 1), Size::new(24, 6)),
                Size::new(3, 3),
            ).into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(right_eye_cx - 12, eye_cy - 14), 24)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(right_eye_cx - 3, eye_cy - 12), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(right_eye_cx, eye_cy - 14), 3)
                .into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd hl failed"))?;
            // Thinking dots (top-right)
            for (dx, dy, sz) in [(46, -36, 5u32), (54, -44, 7), (64, -50, 9)] {
                Circle::new(Point::new(face_cx + dx, face_cy + dy), sz)
                    .into_styled(accent_style).draw(display).map_err(|_| anyhow!("lcd dot failed"))?;
            }
            // Wavy mouth
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 8, face_cy + 22), Size::new(18, 5)),
                Size::new(2, 2),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
        }
        LcdExpression::Sad => {
            // Droopy eyes
            Circle::new(Point::new(left_eye_cx - 10, eye_cy - 10), 20)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(right_eye_cx - 10, eye_cy - 10), 20)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(left_eye_cx - 4, eye_cy - 2), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(right_eye_cx - 4, eye_cy - 2), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            // Tear drop
            Triangle::new(
                Point::new(right_eye_cx + 6, eye_cy + 6),
                Point::new(right_eye_cx + 10, eye_cy + 6),
                Point::new(right_eye_cx + 8, eye_cy + 18),
            ).into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::new(12, 22, 31)).build())
            .draw(display).map_err(|_| anyhow!("lcd tear failed"))?;
            Circle::new(Point::new(right_eye_cx + 4, eye_cy + 12), 8)
                .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::new(12, 22, 31)).build())
                .draw(display).map_err(|_| anyhow!("lcd tear failed"))?;
            // Frown
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(face_cx - 10, face_cy + 24), Size::new(20, 5)),
                Size::new(2, 2),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
            // Down-turned corners
            Circle::new(Point::new(face_cx - 14, face_cy + 22), 5)
                .into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
            Circle::new(Point::new(face_cx + 9, face_cy + 22), 5)
                .into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
        }
        LcdExpression::Surprise => {
            // Big round eyes (wide open)
            Circle::new(Point::new(left_eye_cx - 14, eye_cy - 14), 28)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(right_eye_cx - 14, eye_cy - 14), 28)
                .into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            // Small pupils (shocked)
            Circle::new(Point::new(left_eye_cx - 4, eye_cy - 4), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(right_eye_cx - 4, eye_cy - 4), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(left_eye_cx - 1, eye_cy - 7), 3)
                .into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd hl failed"))?;
            Circle::new(Point::new(right_eye_cx - 1, eye_cy - 7), 3)
                .into_styled(highlight_style).draw(display).map_err(|_| anyhow!("lcd hl failed"))?;
            // O-shaped mouth
            Circle::new(Point::new(face_cx - 10, face_cy + 16), 20)
                .into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
            Circle::new(Point::new(face_cx - 6, face_cy + 20), 12)
                .into_styled(face_style).draw(display).map_err(|_| anyhow!("lcd inner mouth failed"))?;
        }
        LcdExpression::Angry => {
            // Angry eyebrows \\ //
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(left_eye_cx - 14, eye_cy - 20), Size::new(26, 5)),
                Size::new(2, 2),
            ).into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd brow failed"))?;
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(right_eye_cx - 10, eye_cy - 20), Size::new(26, 5)),
                Size::new(2, 2),
            ).into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd brow failed"))?;
            // Narrow angry eyes
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(left_eye_cx - 12, eye_cy - 6), Size::new(24, 14)),
                Size::new(5, 5),
            ).into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            RoundedRectangle::with_equal_corners(
                Rectangle::new(Point::new(right_eye_cx - 12, eye_cy - 6), Size::new(24, 14)),
                Size::new(5, 5),
            ).into_styled(eye_white_style).draw(display).map_err(|_| anyhow!("lcd eye failed"))?;
            Circle::new(Point::new(left_eye_cx - 4, eye_cy - 4), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            Circle::new(Point::new(right_eye_cx - 4, eye_cy - 4), 8)
                .into_styled(pupil_style).draw(display).map_err(|_| anyhow!("lcd pupil failed"))?;
            // Angry frown mouth (wide V)
            Triangle::new(
                Point::new(face_cx - 18, face_cy + 18),
                Point::new(face_cx + 18, face_cy + 18),
                Point::new(face_cx, face_cy + 28),
            ).into_styled(mouth_style).draw(display).map_err(|_| anyhow!("lcd mouth failed"))?;
            // Anger marks (cross) top-right
            for (bx, by) in [(face_cx + 50, face_cy - 50)] {
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(Point::new(bx - 1, by - 8), Size::new(4, 16)),
                    Size::new(2, 2),
                ).into_styled(accent_style).draw(display).map_err(|_| anyhow!("lcd anger mark failed"))?;
                RoundedRectangle::with_equal_corners(
                    Rectangle::new(Point::new(bx - 7, by - 1), Size::new(16, 4)),
                    Size::new(2, 2),
                ).into_styled(accent_style).draw(display).map_err(|_| anyhow!("lcd anger mark failed"))?;
            }
        }
    }

    // ===== Bottom info bar =====
    let bar_h: i32 = if h >= 300 { 56 } else { 44 };
    RoundedRectangle::with_equal_corners(
        Rectangle::new(Point::new(4, h - bar_h - 2), Size::new((w - 8) as u32, bar_h as u32)),
        Size::new(12, 12),
    )
    .into_styled(PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::new(
            bg.r().saturating_add(2).min(31),
            bg.g().saturating_add(4).min(63),
            bg.b().saturating_add(2).min(31),
        ))
        .stroke_color(face_stroke)
        .stroke_width(1)
        .build())
    .draw(display).map_err(|_| anyhow!("lcd info bar failed"))?;

    Text::with_alignment(&headline, Point::new(14, h - 30), title_style, Alignment::Left)
        .draw(display).map_err(|_| anyhow!("lcd title failed"))?;

    for (index, line) in wrap_lcd_text(&subtitle, 28, 2).iter().enumerate() {
        let y = h - 16 + (index as i32 * 10);
        Text::with_alignment(line, Point::new(14, y), body_style, Alignment::Left)
            .draw(display).map_err(|_| anyhow!("lcd subtitle failed"))?;
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum LcdState {
    Boot,
    Ready,
    Wake,
    Speaking,
    Error,
    Message,
}

#[derive(Clone, Copy)]
enum LcdExpression {
    Neutral,
    Happy,
    Wink,
    Love,
    Thinking,
    Sad,
    Surprise,
    Angry,
}

fn normalize_lcd_expression(raw: &str) -> Option<LcdExpression> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "neutral" | "calm" | "staticstate" => Some(LcdExpression::Neutral),
        "happy" | "smile" => Some(LcdExpression::Happy),
        "wink" | "playful" => Some(LcdExpression::Wink),
        "love" | "heart" => Some(LcdExpression::Love),
        "thinking" | "think" | "confused" | "buxue" => Some(LcdExpression::Thinking),
        "sad" | "shy" | "down" => Some(LcdExpression::Sad),
        "surprise" | "surprised" | "scare" | "shocked" => Some(LcdExpression::Surprise),
        "angry" | "anger" | "mad" => Some(LcdExpression::Angry),
        _ => None,
    }
}

fn strip_face_tag(text: &str) -> (String, Option<LcdExpression>) {
    let mut clean = text.to_string();
    let mut expression = None;
    while let Some(start) = clean.find("[FACE:") {
        if let Some(end) = clean[start..].find(']') {
            let tag = &clean[start + 6..start + end];
            if let Some(parsed) = normalize_lcd_expression(tag) {
                expression = Some(parsed);
            }
            clean = format!("{}{}", &clean[..start], clean[start + end + 1..].trim_start());
        } else {
            break;
        }
    }
    (clean.trim().to_string(), expression)
}

fn default_expression_for_state(state: LcdState) -> LcdExpression {
    match state {
        LcdState::Boot => LcdExpression::Thinking,
        LcdState::Ready => LcdExpression::Happy,
        LcdState::Wake => LcdExpression::Surprise,
        LcdState::Speaking => LcdExpression::Happy,
        LcdState::Error => LcdExpression::Sad,
        LcdState::Message => LcdExpression::Neutral,
    }
}

fn parse_lcd_expression(headline: &str, subtitle: &str) -> (String, String, LcdExpression) {
    let (headline, head_expression) = strip_face_tag(headline);
    let (subtitle, sub_expression) = strip_face_tag(subtitle);
    let state = classify_lcd_state(&headline, &subtitle);
    let expression = sub_expression
        .or(head_expression)
        .unwrap_or_else(|| default_expression_for_state(state));
    (headline, subtitle, expression)
}

fn classify_lcd_state(headline: &str, subtitle: &str) -> LcdState {
    let headline_upper = headline.to_ascii_uppercase();
    let subtitle_upper = subtitle.to_ascii_uppercase();
    if headline_upper.contains("ERROR") || subtitle_upper.contains("ERROR") || subtitle_upper.contains("FAILED") {
        LcdState::Error
    } else if headline_upper.contains("SPEAK") || subtitle_upper.contains("TTS") {
        LcdState::Speaking
    } else if subtitle_upper.contains("WAKE") {
        LcdState::Wake
    } else if subtitle_upper.contains("BOOT") {
        LcdState::Boot
    } else if subtitle_upper.contains("READY") {
        LcdState::Ready
    } else {
        LcdState::Message
    }
}

fn emoji_palette(
    state: LcdState,
    expr: LcdExpression,
) -> (Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, Rgb565, &'static str) {
    // State → background, bar/stroke, title, body, meta text, label
    // Dark modern theme — high contrast, clean look
    let (bg, stroke, title_c, body_c, meta_c, label) = match state {
        LcdState::Boot     => (Rgb565::new(1, 2, 5),  Rgb565::new(4, 10, 14), Rgb565::new(22, 50, 31), Rgb565::new(16, 38, 24), Rgb565::new(10, 24, 16), "BOOT"),
        LcdState::Ready    => (Rgb565::new(1, 4, 3),  Rgb565::new(3, 14, 10), Rgb565::new(10, 56, 16), Rgb565::new(8, 44, 12),  Rgb565::new(6, 32, 10),  "READY"),
        LcdState::Wake     => (Rgb565::new(2, 2, 6),  Rgb565::new(6, 10, 18), Rgb565::new(20, 44, 31), Rgb565::new(14, 34, 26), Rgb565::new(10, 24, 20), "WAKE"),
        LcdState::Speaking => (Rgb565::new(4, 3, 1),  Rgb565::new(12, 10, 4), Rgb565::new(31, 56, 12), Rgb565::new(26, 44, 10), Rgb565::new(18, 32, 8),  "VOICE"),
        LcdState::Error    => (Rgb565::new(5, 1, 1),  Rgb565::new(14, 4, 4),  Rgb565::new(31, 20, 16), Rgb565::new(26, 16, 12), Rgb565::new(18, 10, 8),  "ERROR"),
        LcdState::Message  => (Rgb565::new(1, 3, 5),  Rgb565::new(4, 10, 14), Rgb565::new(24, 52, 31), Rgb565::new(18, 40, 26), Rgb565::new(12, 28, 18), "LIVE"),
    };
    // Expression → face fill, eye white, pupil, accent, blush, mouth
    // Warm, expressive color scheme
    let (face, eye_w, pupil, accent, blush, mouth) = match expr {
        LcdExpression::Neutral  => (Rgb565::new(31, 58, 10), Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(8, 42, 22),  Rgb565::new(31, 36, 22), Rgb565::new(8, 12, 6)),
        LcdExpression::Happy    => (Rgb565::new(31, 60, 6),  Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(31, 16, 8),  Rgb565::new(31, 34, 22), Rgb565::new(8, 12, 6)),
        LcdExpression::Wink     => (Rgb565::new(31, 58, 12), Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(26, 20, 28), Rgb565::new(31, 36, 24), Rgb565::new(8, 12, 6)),
        LcdExpression::Love     => (Rgb565::new(31, 54, 16), Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(31, 10, 14), Rgb565::new(31, 30, 24), Rgb565::new(8, 12, 6)),
        LcdExpression::Thinking => (Rgb565::new(28, 56, 14), Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(8, 36, 31),  Rgb565::new(28, 38, 22), Rgb565::new(8, 12, 6)),
        LcdExpression::Sad      => (Rgb565::new(24, 52, 16), Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(8, 24, 31),  Rgb565::new(24, 34, 22), Rgb565::new(8, 12, 6)),
        LcdExpression::Surprise => (Rgb565::new(31, 60, 8),  Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(31, 24, 6),  Rgb565::new(31, 36, 22), Rgb565::new(8, 12, 6)),
        LcdExpression::Angry    => (Rgb565::new(31, 36, 6),  Rgb565::new(31, 63, 31), Rgb565::new(3, 5, 3),  Rgb565::new(31, 8, 4),   Rgb565::new(31, 24, 14), Rgb565::new(8, 12, 6)),
    };
    (bg, face, stroke, eye_w, pupil, accent, blush, mouth, title_c, body_c, meta_c, label)
}

fn wrap_lcd_text(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::from("Waiting for input")];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let candidate_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };

        if candidate_len > max_chars && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
            if lines.len() >= max_lines {
                break;
            }
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if lines.len() < max_lines && !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(text.chars().take(max_chars).collect());
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
    }

    if let Some(last) = lines.last_mut() {
        if last.chars().count() > max_chars {
            *last = last.chars().take(max_chars.saturating_sub(1)).collect::<String>() + "…";
        }
    }

    lines
}

pub fn play_test_tone(pins: &AudioPins) -> Result<()> {
    ensure_pin(pins.bclk, "audio.bclk")?;
    ensure_pin(pins.ws, "audio.ws")?;
    ensure_pin(pins.dout, "audio.dout")?;

    if is_zhengchen_cam_audio(pins) {
        enable_zhengchen_cam_speaker(pins.sample_rate)?;
        let _ = configure_board_io_expander_bit(1, true);
    }

    let peripherals = unsafe { Peripherals::steal() };
    let cfg = StdConfig::philips(pins.sample_rate, DataBitWidth::Bits16);
    let mut i2s = I2sDriver::<I2sTx>::new_std_tx(
        peripherals.i2s0,
        &cfg,
        unsafe { AnyIOPin::steal(pins.bclk as u8) },
        unsafe { AnyIOPin::steal(pins.dout as u8) },
        if pins.mclk >= 0 {
            Some(unsafe { AnyIOPin::steal(pins.mclk as u8) })
        } else {
            AnyIOPin::none()
        },
        unsafe { AnyIOPin::steal(pins.ws as u8) },
    )?;

    let mut frames = Vec::new();
    for freq in [392.0f32, 493.88f32, 587.33f32] {
        append_sine(&mut frames, pins.sample_rate, freq, 120, 0.12);
        append_silence(&mut frames, pins.sample_rate, 40);
    }

    i2s.tx_enable()?;
    i2s.write_all(&frames, 1000)?;
    info!("Audio tone test played: {} bytes", frames.len());
    Ok(())
}

pub fn play_pcm16(pins: &AudioPins, pcm: &[i16], sample_rate: u32, channels: usize) -> Result<usize> {
    ensure_pin(pins.bclk, "audio.bclk")?;
    ensure_pin(pins.ws, "audio.ws")?;
    ensure_pin(pins.dout, "audio.dout")?;

    if is_zhengchen_cam_audio(pins) {
        enable_zhengchen_cam_speaker(sample_rate)?;
        let _ = configure_board_io_expander_bit(1, true);
    }

    if pcm.is_empty() {
        bail!("pcm buffer is empty");
    }

    let peripherals = unsafe { Peripherals::steal() };
    let cfg = StdConfig::philips(sample_rate, DataBitWidth::Bits16);
    let mut i2s = I2sDriver::<I2sTx>::new_std_tx(
        peripherals.i2s0,
        &cfg,
        unsafe { AnyIOPin::steal(pins.bclk as u8) },
        unsafe { AnyIOPin::steal(pins.dout as u8) },
        if pins.mclk >= 0 {
            Some(unsafe { AnyIOPin::steal(pins.mclk as u8) })
        } else {
            AnyIOPin::none()
        },
        unsafe { AnyIOPin::steal(pins.ws as u8) },
    )?;

    let mut frames = Vec::with_capacity(pcm.len() * 4);
    if channels <= 1 {
        for sample in pcm {
            let bytes = sample.to_le_bytes();
            frames.extend_from_slice(&bytes);
            frames.extend_from_slice(&bytes);
        }
    } else {
        for sample in pcm {
            frames.extend_from_slice(&sample.to_le_bytes());
        }
    }

    i2s.tx_enable()?;
    i2s.write_all(&frames, 2000)?;
    info!("PCM audio played: {} samples @ {} Hz ({} ch)", pcm.len(), sample_rate, channels);
    Ok(frames.len())
}

pub fn capture_mic_pcm(pins: &AudioPins, duration_ms: u32, max_samples: usize) -> Result<Vec<i16>> {
    ensure_pin(pins.mic_sck, "audio.mic_sck")?;
    ensure_pin(pins.mic_ws, "audio.mic_ws")?;
    ensure_pin(pins.mic_din, "audio.mic_din")?;

    let peripherals = unsafe { Peripherals::steal() };
    let cfg = StdConfig::philips(pins.sample_rate, DataBitWidth::Bits16);
    let mut i2s = I2sDriver::<I2sRx>::new_std_rx(
        peripherals.i2s1,
        &cfg,
        unsafe { AnyIOPin::steal(pins.mic_sck as u8) },
        unsafe { AnyIOPin::steal(pins.mic_din as u8) },
        if pins.mclk >= 0 {
            Some(unsafe { AnyIOPin::steal(pins.mclk as u8) })
        } else {
            AnyIOPin::none()
        },
        unsafe { AnyIOPin::steal(pins.mic_ws as u8) },
    )?;

    let deadline = Instant::now() + Duration::from_millis(duration_ms as u64);
    let mut buf = [0u8; 2048];
    let mut pcm = Vec::with_capacity(max_samples.min((pins.sample_rate as usize * duration_ms as usize / 1000) + 1024));

    i2s.rx_enable()?;
    while Instant::now() < deadline && pcm.len() < max_samples {
        let read = i2s.read(&mut buf, 100)?;
        for chunk in buf[..read].chunks_exact(2) {
            pcm.push(i16::from_le_bytes([chunk[0], chunk[1]]));
            if pcm.len() >= max_samples {
                break;
            }
        }
    }

    if pcm.is_empty() {
        bail!("mic returned no pcm samples");
    }

    Ok(pcm)
}

pub fn capture_mic_snapshot(pins: &AudioPins, duration_ms: u32) -> Result<MicSnapshot> {
    let pcm = capture_mic_pcm(
        pins,
        duration_ms,
        pins.sample_rate as usize * duration_ms as usize / 1000 + 1024,
    )?;
    let mut energy: u64 = 0;
    let mut peak: i16 = 0;
    let samples = pcm.len();

    for sample in pcm {
        let abs = sample.saturating_abs();
        if abs > peak {
            peak = abs;
        }
        let signed = sample as i32;
        energy += (signed * signed) as u64;
    }

    if samples == 0 {
        bail!("mic returned no samples");
    }

    let rms = ((energy / samples as u64) as f64).sqrt() as u32;
    Ok(MicSnapshot { rms, peak, samples })
}

pub fn wake_probe(pins: &AudioPins, duration_ms: u32, threshold: u32) -> Result<bool> {
    let snapshot = capture_mic_snapshot(pins, duration_ms)?;
    Ok(snapshot.rms >= threshold)
}

fn ensure_pin(pin: i32, label: &str) -> Result<i32> {
    if pin < 0 {
        bail!("{} is not configured", label);
    }
    Ok(pin)
}

fn append_sine(out: &mut Vec<u8>, sample_rate: u32, freq: f32, duration_ms: u32, volume: f32) {
    let frame_count = (sample_rate as u64 * duration_ms as u64 / 1000) as usize;
    for idx in 0..frame_count {
        let phase = TAU * freq * (idx as f32 / sample_rate as f32);
        let sample = (phase.sin() * volume * i16::MAX as f32) as i16;
        out.extend_from_slice(&sample.to_le_bytes());
        out.extend_from_slice(&sample.to_le_bytes());
    }
}

fn append_silence(out: &mut Vec<u8>, sample_rate: u32, duration_ms: u32) {
    let frame_count = (sample_rate as u64 * duration_ms as u64 / 1000) as usize;
    for _ in 0..frame_count {
        out.extend_from_slice(&0i16.to_le_bytes());
        out.extend_from_slice(&0i16.to_le_bytes());
    }
}

fn is_zhengchen_cam_lcd(pins: &LcdPins) -> bool {
    pins.dc == 39 && pins.bl == 42 && pins.sclk == 41 && pins.sda == 40
}

fn is_zhengchen_cam_audio(pins: &AudioPins) -> bool {
    pins.bclk == 14 && pins.ws == 13 && pins.dout == 45 && pins.din == 12 && pins.mclk == 38
}

fn normalize_st7789_size(width: u16, height: u16) -> (u16, u16) {
    match (width, height) {
        (320, 240) => (240, 320),
        (w, h) if w > 240 && h <= 240 => (240, 320),
        (w, h) if w == 0 || h == 0 => (240, 320),
        (w, h) => (w.min(240), h.min(320)),
    }
}

fn set_lcd_backlight(pins: &LcdPins) -> Result<()> {
    let level = if is_zhengchen_cam_lcd(pins) { 0 } else { 1 };
    unsafe {
        gpio_reset_pin(pins.bl as i32);
        gpio_set_direction(pins.bl as i32, gpio_mode_t_GPIO_MODE_OUTPUT);
        gpio_set_level(pins.bl as i32, level);
    }
    info!("LCD backlight forced to level {} on GPIO{}", level, pins.bl);
    Ok(())
}

fn enable_zhengchen_cam_speaker(sample_rate: u32) -> Result<()> {
    info!("Audio bring-up: enabling PCA9557 output bit for speaker path");
    configure_board_io_expander_bit(1, true)?;

    info!("Audio bring-up: opening board I2C on GPIO{} / GPIO{}", BOARD_IO_SDA_PIN, BOARD_IO_SCL_PIN);
    let mut i2c = open_board_i2c()?;
    let codec_addr = detect_es8311_addr(&mut i2c)?;
    let chip_id1 = es8311_read_reg(&mut i2c, codec_addr, ES8311_CHD1_REGFD)?;
    let chip_id2 = es8311_read_reg(&mut i2c, codec_addr, ES8311_CHD2_REGFE)?;
    let version = es8311_read_reg(&mut i2c, codec_addr, ES8311_CHVER_REGFF)?;
    info!(
        "ES8311 detected @0x{:02X}: chip={:02X}{:02X} version={:02X} sample_rate={}Hz",
        codec_addr, chip_id1, chip_id2, version, sample_rate
    );

    es8311_write_reg(&mut i2c, codec_addr, ES8311_GPIO_REG44, 0x08)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_GPIO_REG44, 0x08)?;

    es8311_write_reg(&mut i2c, codec_addr, ES8311_CLK_MANAGER_REG01, 0x30)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_CLK_MANAGER_REG02, 0x00)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_CLK_MANAGER_REG03, 0x10)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_ADC_REG16, 0x24)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_CLK_MANAGER_REG04, 0x10)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_CLK_MANAGER_REG05, 0x00)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG0B, 0x00)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG0C, 0x00)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG10, 0x1F)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG11, 0x7F)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_RESET_REG00, 0x80)?;

    let mut reset_reg = es8311_read_reg(&mut i2c, codec_addr, ES8311_RESET_REG00)?;
    reset_reg &= 0xBF;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_RESET_REG00, reset_reg)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_CLK_MANAGER_REG01, 0x3F)?;

    configure_es8311_sample_rate(&mut i2c, codec_addr, sample_rate)?;

    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG13, 0x10)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_ADC_REG1B, 0x0A)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_ADC_REG1C, 0x6A)?;

    let dac_iface = es8311_read_reg(&mut i2c, codec_addr, ES8311_SDPIN_REG09)? & 0xFC;
    let adc_iface = es8311_read_reg(&mut i2c, codec_addr, ES8311_SDPOUT_REG0A)? & 0xFC;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SDPIN_REG09, dac_iface | 0x0C)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SDPOUT_REG0A, adc_iface | 0x0C)?;

    let dac_start = es8311_read_reg(&mut i2c, codec_addr, ES8311_SDPIN_REG09)? & !0x40;
    let adc_stop = es8311_read_reg(&mut i2c, codec_addr, ES8311_SDPOUT_REG0A)? | 0x40;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SDPIN_REG09, dac_start)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SDPOUT_REG0A, adc_stop)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_ADC_REG17, 0xBF)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG0E, 0x02)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG12, 0x00)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG14, 0x1A)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_SYSTEM_REG0D, 0x01)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_ADC_REG15, 0x40)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_DAC_REG37, 0x08)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_GP_REG45, 0x00)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_GPIO_REG44, 0x58)?;

    let mute_reg = es8311_read_reg(&mut i2c, codec_addr, ES8311_DAC_REG31)? & 0x9F;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_DAC_REG31, mute_reg)?;
    es8311_write_reg(&mut i2c, codec_addr, ES8311_DAC_REG32, 0xC0)?;

    info!("ES8311 output path enabled");

    thread::sleep(Duration::from_millis(60));
    Ok(())
}

fn configure_es8311_sample_rate(i2c: &mut I2cDriver<'_>, codec_addr: u8, sample_rate: u32) -> Result<()> {
    let (reg02, reg03, reg04, reg05, reg06, reg07, reg08) = match sample_rate {
        24_000 => (0x00, 0x10, 0x10, 0x00, 0x03, 0x00, 0xFF),
        16_000 => (0x08, 0x10, 0x10, 0x00, 0x03, 0x00, 0xFF),
        32_000 => (0x18, 0x10, 0x10, 0x00, 0x03, 0x00, 0xFF),
        48_000 => (0x00, 0x10, 0x10, 0x00, 0x03, 0x00, 0xFF),
        other => {
            warn!("ES8311 sample rate {}Hz not in tuned table, using 24kHz clock plan", other);
            (0x00, 0x10, 0x10, 0x00, 0x03, 0x00, 0xFF)
        }
    };

    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG02, reg02)?;
    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG03, reg03)?;
    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG04, reg04)?;
    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG05, reg05)?;
    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG06, reg06)?;
    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG07, reg07)?;
    es8311_write_reg(i2c, codec_addr, ES8311_CLK_MANAGER_REG08, reg08)?;
    Ok(())
}

fn open_board_i2c() -> Result<I2cDriver<'static>> {
    let peripherals = unsafe { Peripherals::steal() };
    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(
        peripherals.i2c1,
        unsafe { AnyIOPin::steal(BOARD_IO_SDA_PIN as u8) },
        unsafe { AnyIOPin::steal(BOARD_IO_SCL_PIN as u8) },
        &config,
    )
    .map_err(|err| anyhow!("board i2c open failed on SDA={} SCL={}: {}", BOARD_IO_SDA_PIN, BOARD_IO_SCL_PIN, err))?;
    Ok(i2c)
}

fn detect_es8311_addr(i2c: &mut I2cDriver<'_>) -> Result<u8> {
    let mut last_error = None;
    for addr in ES8311_ADDRS {
        match es8311_read_reg(i2c, addr, ES8311_CHD1_REGFD) {
            Ok(chip) => {
                info!("ES8311 probe ok at 0x{:02X}, chip id high=0x{:02X}", addr, chip);
                return Ok(addr);
            }
            Err(err) => {
                warn!("ES8311 probe failed at 0x{:02X}: {}", addr, err);
                last_error = Some(err);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("no ES8311 address responded")))
}

fn es8311_write_reg(i2c: &mut I2cDriver<'_>, codec_addr: u8, reg: u8, value: u8) -> Result<()> {
    i2c.write(codec_addr, &[reg, value], 100)
        .map_err(|err| anyhow!("es8311 addr 0x{codec_addr:02X} write 0x{reg:02X} failed: {err}"))?;
    Ok(())
}

fn es8311_read_reg(i2c: &mut I2cDriver<'_>, codec_addr: u8, reg: u8) -> Result<u8> {
    let mut buf = [0u8; 1];
    i2c.write_read(codec_addr, &[reg], &mut buf, 100)
        .map_err(|err| anyhow!("es8311 addr 0x{codec_addr:02X} read 0x{reg:02X} failed: {err}"))?;
    Ok(buf[0])
}

fn configure_board_io_expander_bit(bit: u8, level_high: bool) -> Result<()> {
    let peripherals = unsafe { Peripherals::steal() };
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let mut i2c = I2cDriver::new(
        peripherals.i2c1,
        unsafe { AnyIOPin::steal(BOARD_IO_SDA_PIN as u8) },
        unsafe { AnyIOPin::steal(BOARD_IO_SCL_PIN as u8) },
        &config,
    )
    .map_err(|err| anyhow!("io expander i2c open failed on SDA={} SCL={}: {}", BOARD_IO_SDA_PIN, BOARD_IO_SCL_PIN, err))?;

    let mut last_error = None;
    for addr in BOARD_IO_EXPANDER_ADDRS {
        match configure_expander_bit_at_addr(&mut i2c, addr, bit, level_high) {
            Ok(value) => {
                info!("Board expander @0x{:02X} bit {} set to {} (0x{:02X})", addr, bit, if level_high { 1 } else { 0 }, value);
                return Ok(());
            }
            Err(err) => {
                warn!("Board expander probe at 0x{:02X} failed: {}", addr, err);
                last_error = Some(err);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow!("no board expander address responded")))
}

fn configure_expander_bit_at_addr(i2c: &mut I2cDriver<'_>, addr: u8, bit: u8, level_high: bool) -> Result<u8> {
    i2c.write(addr, &[0x01, 0x03], 100)
        .map_err(|err| anyhow!("write output reg failed: {}", err))?;
    i2c.write(addr, &[0x03, 0xF8], 100)
        .map_err(|err| anyhow!("write config reg failed: {}", err))?;

    let mut current = [0u8; 1];
    i2c.write_read(addr, &[0x01], &mut current, 100)
        .map_err(|err| anyhow!("read output reg failed: {}", err))?;
    let value = if level_high {
        current[0] | (1 << bit)
    } else {
        current[0] & !(1 << bit)
    };
    i2c.write(addr, &[0x01, value], 100)
        .map_err(|err| anyhow!("set bit {} failed: {}", bit, err))?;
    Ok(value)
}
