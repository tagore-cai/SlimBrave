//! 内嵌 CJK 字体：保证中文在 egui 默认字体之外可用。

use eframe::egui;

pub fn cjk_font_bytes() -> &'static [u8] {
    include_bytes!("../../../assets/NotoSansSC-Regular.otf")
}

pub fn install_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "cjk".to_owned(),
        egui::FontData::from_static(cjk_font_bytes()).into(),
    );
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
            defs.families
                .get_mut(&family)
                .unwrap()
                .push("cjk".to_owned());
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
}
