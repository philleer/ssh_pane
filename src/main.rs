mod app;
mod pathutil;
mod ssh_worker;
mod ui;

use app::App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("SSH Split Pane"),
        ..Default::default()
    };

    eframe::run_native(
        "SSH Split Pane",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
