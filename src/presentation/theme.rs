use std::collections::BTreeMap;

use eframe::egui::{self, Color32, RichText, Stroke};
use serde::Deserialize;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Preset,
    Info,
    Primary,
    Warning,
    Success,
    Danger,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeBase {
    Dark,
    Light,
}

impl ThemeBase {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(ThemeBase::Dark),
            "light" => Some(ThemeBase::Light),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ThemeOverrides {
    pub base: Option<ThemeBase>,
    pub colors: BTreeMap<String, Color32>,
}

impl ThemeOverrides {
    pub fn from_json(json: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Raw {
            base: Option<String>,
            colors: Option<BTreeMap<String, String>>,
        }
        let raw: Raw = serde_json::from_str(json).map_err(|err| err.to_string())?;
        let mut overrides = ThemeOverrides::default();
        if let Some(base) = raw.base {
            overrides.base = ThemeBase::parse(&base);
        }
        if let Some(colors) = raw.colors {
            for (name, value) in colors {
                if let Some(color) = parse_color(&value) {
                    overrides.colors.insert(name, color);
                }
            }
        }
        Ok(overrides)
    }

    pub fn apply(&self) -> Theme {
        let mut theme = match self.base.unwrap_or(ThemeBase::Dark) {
            ThemeBase::Dark => Theme::dark(),
            ThemeBase::Light => Theme::light(),
        };
        for (name, color) in &self.colors {
            theme.set_token(name, *color);
        }
        theme
    }
}

fn parse_color(value: &str) -> Option<Color32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    let (r, g, b, a) = match hex.len() {
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some(Color32::from_rgba_unmultiplied(r, g, b, a))
}

#[derive(Clone, Copy)]
pub struct ButtonPalette {
    pub preset: Color32,
    pub info: Color32,
    pub primary: Color32,
    pub warning: Color32,
    pub success: Color32,
    pub danger: Color32,
}

#[derive(Clone)]
pub struct Theme {
    pub is_dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub panel_border: Color32,
    pub text: Color32,
    pub muted_text: Color32,
    pub accent: Color32,
    pub section: Color32,
    pub saved: Color32,
    pub dirty: Color32,
    pub status_bg: Color32,
    pub status_text: Color32,
    pub widget_bg: Color32,
    pub hover_bg: Color32,
    pub active_bg: Color32,
    pub selection: Color32,
    pub buttons: ButtonPalette,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            is_dark: true,
            bg: Color32::from_rgb(0x19, 0x19, 0x19),
            panel: Color32::from_rgb(0x23, 0x23, 0x23),
            panel_border: Color32::from_rgb(0x3C, 0x3C, 0x3C),
            text: Color32::WHITE,
            muted_text: Color32::from_rgb(0xAA, 0xAA, 0xAA),
            accent: Color32::from_rgb(0x87, 0xCE, 0xFA),
            section: Color32::from_rgb(0xFF, 0xA0, 0x7A),
            saved: Color32::from_rgb(0x90, 0xEE, 0x90),
            dirty: Color32::from_rgb(0xFF, 0xD7, 0x00),
            status_bg: Color32::from_rgb(0x2D, 0x2D, 0x2D),
            status_text: Color32::from_rgb(0xAA, 0xAA, 0xAA),
            widget_bg: Color32::from_rgb(0x30, 0x30, 0x30),
            hover_bg: Color32::from_rgb(0x3A, 0x3A, 0x3A),
            active_bg: Color32::from_rgb(0x45, 0x45, 0x45),
            selection: Color32::from_rgb(0x1F, 0x4D, 0x8C),
            buttons: ButtonPalette {
                preset: Color32::from_rgb(0xE6, 0x51, 0x00),
                info: Color32::from_rgb(0x0D, 0x47, 0xA1),
                primary: Color32::from_rgb(0x19, 0x76, 0xD2),
                warning: Color32::from_rgb(0xF5, 0x7F, 0x17),
                success: Color32::from_rgb(0x2E, 0x7D, 0x32),
                danger: Color32::from_rgb(0xC6, 0x28, 0x28),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            bg: Color32::from_rgb(0xF2, 0xF2, 0xF2),
            panel: Color32::WHITE,
            panel_border: Color32::from_rgb(0xD5, 0xD5, 0xD5),
            text: Color32::from_rgb(0x1C, 0x1C, 0x1C),
            muted_text: Color32::from_rgb(0x5F, 0x5F, 0x5F),
            accent: Color32::from_rgb(0x15, 0x65, 0xC0),
            section: Color32::from_rgb(0xE6, 0x51, 0x00),
            saved: Color32::from_rgb(0x2E, 0x7D, 0x32),
            dirty: Color32::from_rgb(0x9A, 0x67, 0x00),
            status_bg: Color32::from_rgb(0xE6, 0xE6, 0xE6),
            status_text: Color32::from_rgb(0x4A, 0x4A, 0x4A),
            widget_bg: Color32::from_rgb(0xE8, 0xE8, 0xE8),
            hover_bg: Color32::from_rgb(0xDC, 0xDC, 0xDC),
            active_bg: Color32::from_rgb(0xCF, 0xCF, 0xCF),
            selection: Color32::from_rgb(0xBB, 0xDE, 0xFB),
            buttons: ButtonPalette {
                preset: Color32::from_rgb(0xE6, 0x51, 0x00),
                info: Color32::from_rgb(0x0D, 0x47, 0xA1),
                primary: Color32::from_rgb(0x19, 0x76, 0xD2),
                warning: Color32::from_rgb(0xF5, 0x7F, 0x17),
                success: Color32::from_rgb(0x2E, 0x7D, 0x32),
                danger: Color32::from_rgb(0xC6, 0x28, 0x28),
            },
        }
    }

    pub fn from_egui(theme: egui::Theme) -> Self {
        match theme {
            egui::Theme::Dark => Self::dark(),
            egui::Theme::Light => Self::light(),
        }
    }

    pub fn button(&self, text: &str, style: ButtonStyle) -> egui::Button<'_> {
        let color = match style {
            ButtonStyle::Preset => self.buttons.preset,
            ButtonStyle::Info => self.buttons.info,
            ButtonStyle::Primary => self.buttons.primary,
            ButtonStyle::Warning => self.buttons.warning,
            ButtonStyle::Success => self.buttons.success,
            ButtonStyle::Danger => self.buttons.danger,
        };
        egui::Button::new(RichText::new(text).color(self.text).strong())
            .fill(color)
            .stroke(Stroke::NONE)
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };        visuals.panel_fill = self.bg;
        visuals.window_fill = self.bg;
        visuals.override_text_color = Some(self.text);
        visuals.selection.bg_fill = self.selection;
        visuals.faint_bg_color = self.panel;

        for widget in [
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
        ] {
            widget.fg_stroke = Stroke::new(1.0_f32, self.text);
        }
        visuals.widgets.noninteractive.bg_fill = self.panel;
        visuals.widgets.inactive.bg_fill = self.widget_bg;
        visuals.widgets.hovered.bg_fill = self.hover_bg;
        visuals.widgets.active.bg_fill = self.active_bg;

        ctx.set_visuals(visuals);
    }

    pub fn panel_frame(&self) -> egui::Frame {
        egui::Frame::NONE
            .fill(self.panel)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .stroke(Stroke::new(1.0_f32, self.panel_border))
    }

    pub fn top_bar_frame(&self) -> egui::Frame {
        egui::Frame::NONE
            .fill(self.bg)
            .inner_margin(egui::Margin::symmetric(12, 6))
    }

    pub fn status_frame(&self) -> egui::Frame {
        egui::Frame::NONE
            .fill(self.status_bg)
            .inner_margin(egui::Margin::symmetric(12, 6))
    }

    pub fn section_title(&self, ui: &mut egui::Ui, text: &str) {
        ui.label(
            RichText::new(text)
                .color(self.section)
                .strong()
                .size(14.0),
        );
    }

    pub fn accent_label(&self, ui: &mut egui::Ui, text: &str) {
        ui.label(RichText::new(text).color(self.accent).strong());
    }

    pub fn status_label(&self, ui: &mut egui::Ui, text: &str) {
        ui.label(
            RichText::new(text)
                .color(self.status_text)
                .font(egui::FontId::monospace(11.0)),
        );
    }

    fn set_token(&mut self, name: &str, color: Color32) {
        match name {
            "bg" => self.bg = color,
            "panel" => self.panel = color,
            "panel_border" => self.panel_border = color,
            "text" => self.text = color,
            "muted_text" => self.muted_text = color,
            "accent" => self.accent = color,
            "section" => self.section = color,
            "saved" => self.saved = color,
            "dirty" => self.dirty = color,
            "status_bg" => self.status_bg = color,
            "status_text" => self.status_text = color,
            "widget_bg" => self.widget_bg = color,
            "hover_bg" => self.hover_bg = color,
            "active_bg" => self.active_bg = color,
            "selection" => self.selection = color,
            "button_preset" => self.buttons.preset = color,
            "button_info" => self.buttons.info = color,
            "button_primary" => self.buttons.primary = color,
            "button_warning" => self.buttons.warning = color,
            "button_success" => self.buttons.success = color,
            "button_danger" => self.buttons.danger = color,
            _ => {}
        }
    }
}

pub fn cjk_font_bytes() -> &'static [u8] {
    include_bytes!("../../assets/NotoSansSC-Regular.otf")
}

pub fn install_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), egui::FontData::from_static(cjk_font_bytes()).into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .unwrap()
            .push("cjk".to_owned());
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_font_parses_and_has_cjk_glyphs() {
        use ab_glyph::Font;
        let font = ab_glyph::FontArc::try_from_vec(cjk_font_bytes().to_vec())
            .unwrap_or_else(|err| panic!("font failed to parse: {err}"));
        for ch in ['中', '文', '打', '开', '设', '置'] {
            assert!(font.glyph_id(ch).0 != 0, "glyph missing for {ch}");
        }
    }

    #[test]
    fn egui_layouts_cjk_via_fallback() {
        let mut defs = egui::FontDefinitions::default();
        defs.font_data.insert(
            "cjk".to_owned(),
            egui::FontData::from_static(cjk_font_bytes()).into(),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            defs.families.get_mut(&family).unwrap().push("cjk".to_owned());
        }
        let fonts = egui::epaint::text::Fonts::new(1.0, 4096, defs);
        let font_id = egui::FontId::proportional(14.0);
        assert!(
            fonts.has_glyphs(&font_id, "中文测试"),
            "fallback font must provide CJK glyphs"
        );

        let default_fonts =
            egui::epaint::text::Fonts::new(1.0, 4096, egui::FontDefinitions::default());
        assert!(
            !default_fonts.has_glyphs(&font_id, "中文测试"),
            "default egui fonts must NOT have CJK glyphs (sanity check)"
        );
    }

    #[test]
    fn dark_and_light_themes_are_consistent() {
        let dark = Theme::dark();
        let light = Theme::light();
        assert!(dark.is_dark);
        assert!(!light.is_dark);
        assert_ne!(dark.bg, light.bg);
        assert_ne!(dark.text, light.text);
    }

    #[test]
    fn parse_color_accepts_hex_formats() {
        assert_eq!(
            parse_color("#ff0000"),
            Some(Color32::from_rgb(255, 0, 0))
        );
        assert_eq!(
            parse_color("ff000080"),
            Some(Color32::from_rgba_unmultiplied(255, 0, 0, 128))
        );
        assert_eq!(parse_color("nope"), None);
        assert_eq!(parse_color("#12345"), None);
    }

    #[test]
    fn theme_overrides_apply_over_base() {
        let json = r##"{
            "base": "light",
            "colors": {
                "bg": "#ff0000",
                "panel": "#00ff00",
                "button_danger": "#0000ff",
                "not_a_token": "#123456"
            }
        }"##;
        let overrides = ThemeOverrides::from_json(json).unwrap();
        let theme = overrides.apply();
        assert!(!theme.is_dark, "base light must be applied");
        assert_eq!(theme.bg, Color32::from_rgb(255, 0, 0));
        assert_eq!(theme.panel, Color32::from_rgb(0, 255, 0));
        assert_eq!(theme.buttons.danger, Color32::from_rgb(0, 0, 255));
        assert_ne!(theme.accent, theme.bg, "untouched tokens keep base values");
    }

    #[test]
    fn theme_overrides_default_to_dark_base() {
        let overrides = ThemeOverrides::from_json(r##"{"colors": {"bg": "#101010"}}"##).unwrap();
        assert!(overrides.apply().is_dark);
    }

    #[test]
    fn theme_overrides_reject_invalid_json() {
        assert!(ThemeOverrides::from_json("not json").is_err());
    }
}
