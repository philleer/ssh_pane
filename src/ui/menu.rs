//! Menu bar: File / View / Help.

use ssh_pane::settings::Theme;
use ssh_pane::ssh_worker::{OpMode, PaneSide};

#[derive(Default)]
pub struct MenuActions {
    pub disconnect: bool,
    pub exit: bool,
    pub refresh_left: bool,
    pub refresh_right: bool,
    pub refresh_both: bool,
    pub new_folder: Option<PaneSide>,
    pub show_about: bool,
    pub set_mode: Option<OpMode>,
    pub set_theme: Option<Theme>,
}

pub fn show_menu_bar(
    ui: &mut egui::Ui,
    connected: bool,
    busy: bool,
    mode: OpMode,
    active: PaneSide,
    theme: Theme,
) -> MenuActions {
    let mut actions = MenuActions::default();

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            ui.set_min_width(200.0);
            let can_edit = connected && !busy;

            if ui
                .add_enabled(
                    can_edit,
                    egui::Button::new(format!(
                        "New Folder… ({})",
                        match active {
                            PaneSide::Left => "Left",
                            PaneSide::Right => "Right",
                        }
                    )),
                )
                .clicked()
            {
                actions.new_folder = Some(active);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add_enabled(can_edit, egui::Button::new("New Folder in Left…"))
                .clicked()
            {
                actions.new_folder = Some(PaneSide::Left);
                ui.close_menu();
            }
            if ui
                .add_enabled(can_edit, egui::Button::new("New Folder in Right…"))
                .clicked()
            {
                actions.new_folder = Some(PaneSide::Right);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add_enabled(connected && !busy, egui::Button::new("Disconnect"))
                .clicked()
            {
                actions.disconnect = true;
                ui.close_menu();
            }
            if ui.button("Exit").clicked() {
                actions.exit = true;
                ui.close_menu();
            }
        });

        ui.menu_button("View", |ui| {
            ui.set_min_width(180.0);
            if ui
                .add_enabled(
                    connected && !busy && mode != OpMode::Copy,
                    egui::Button::new("Mode: Copy (CP)"),
                )
                .clicked()
            {
                actions.set_mode = Some(OpMode::Copy);
                ui.close_menu();
            }
            if ui
                .add_enabled(
                    connected && !busy && mode != OpMode::Move,
                    egui::Button::new("Mode: Move (MV)"),
                )
                .clicked()
            {
                actions.set_mode = Some(OpMode::Move);
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add_enabled(connected && !busy, egui::Button::new("Refresh Left"))
                .clicked()
            {
                actions.refresh_left = true;
                ui.close_menu();
            }
            if ui
                .add_enabled(connected && !busy, egui::Button::new("Refresh Right"))
                .clicked()
            {
                actions.refresh_right = true;
                ui.close_menu();
            }
            if ui
                .add_enabled(connected && !busy, egui::Button::new("Refresh Both"))
                .clicked()
            {
                actions.refresh_both = true;
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button("Theme", |ui| {
                ui.set_min_width(120.0);
                if ui
                    .add_enabled(theme != Theme::Dark, egui::Button::new("Dark"))
                    .clicked()
                {
                    actions.set_theme = Some(Theme::Dark);
                    ui.close_menu();
                }
                if ui
                    .add_enabled(theme != Theme::Light, egui::Button::new("Light"))
                    .clicked()
                {
                    actions.set_theme = Some(Theme::Light);
                    ui.close_menu();
                }
            });
        });

        ui.menu_button("Help", |ui| {
            ui.set_min_width(140.0);
            if ui.button("About").clicked() {
                actions.show_about = true;
                ui.close_menu();
            }
        });
    });

    actions
}
