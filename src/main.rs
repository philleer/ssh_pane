mod app;
mod ui;

use app::App;

fn main() -> eframe::Result<()> {
    ssh_pane::logging::init();
    log::info!("SSH Split Pane starting");

    // Disable OS/CSD decorations. Their drop-shadow often remains visible as a
    // "pre-maximize outline" after maximizing (Wayland CSD / WSLg / DWM).
    // We draw a custom title bar instead and paint edge-to-edge when maximized.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_title("SSH Split Pane"),
        ..Default::default()
    };

    eframe::run_native(
        "SSH Split Pane",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
