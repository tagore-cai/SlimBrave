//! egui 主界面布局编排。
//!
//! - [`layout`]：顶栏 / 底栏 / 内容面板渲染助手
//! - [`widgets`]：策略列表、权限面板、名称 / 提示本地化
//! - [`windows`]：策略管理窗口

mod layout;
mod widgets;
mod windows;

use eframe::egui;

use crate::application::app::SlimBraveApp;

impl eframe::App for SlimBraveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_theme(self.theme_pref);
        let egui_theme = ctx.theme();
        if self.last_theme != Some(egui_theme) {
            self.last_theme = Some(egui_theme);
            self.theme(egui_theme).apply(ctx);
        }
        let theme = self.theme(egui_theme);

        self.confirm_dialog(ctx);
        self.policy_manager_window(ctx);
        self.header_panel(ctx, &theme);
        self.status_panel(ctx, &theme);
        self.content_panels(ctx, &theme);
    }
}
