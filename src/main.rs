mod application;
mod domain;
mod infrastructure;
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
