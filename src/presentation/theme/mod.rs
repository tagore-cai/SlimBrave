//! egui 视觉主题：设计令牌、深浅配色、JSON 覆盖与 CJK 字体。
//!
//! - [`tokens`]：颜色令牌、主题构造与 widget 绘制助手
//! - [`fonts`]：内嵌 CJK 字体的注册

mod fonts;
mod tokens;

pub use fonts::install_cjk_fonts;
pub use tokens::{ButtonStyle, Theme, ThemeOverrides};
