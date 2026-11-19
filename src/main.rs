// Windows GUI 程序默认使用 console 子系统，双击 exe 会附带弹出一个终端窗口。
// 声明为 windows 子系统后与控制台分离，不再弹出终端；该属性在非 Windows 目标自动忽略。
// 代价：stderr/stdout 输出不再显示，日志需落盘（见下方 logging 初始化）。
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod application;
mod domain;
mod infrastructure;
mod logging;
mod presentation;

use application::app::SlimBraveApp;
use eframe::egui;

fn main() -> eframe::Result {
    let platform = infrastructure::platform::detect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 550.0])
            .with_min_inner_size([900.0, 400.0])
            .with_title("SlimBrave"),
        ..Default::default()
    };

    eframe::run_native(
        "SlimBrave",
        options,
        Box::new(move |cc| Ok(Box::new(SlimBraveApp::new(cc, platform)))),
    )
}
