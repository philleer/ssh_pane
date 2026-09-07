//! Top toolbar: CP/MV mode and disconnect.

use egui::{RichText, Vec2};

use ssh_split_pane::ssh_worker::OpMode;
use crate::ui::connect::mode_button;

pub struct ToolbarResult {
    pub disconnect: bool,
}

pub fn show_toolbar(
    ui: &mut egui::Ui,
    mode: &mut OpMode,
    host_label: &str,
    busy: bool,
) -> ToolbarResult {
    let mut disconnect = false;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Mode:").strong().size(19.0));
        if mode_button(ui, "CP", *mode == OpMode::Copy).clicked() && !busy {
            *mode = OpMode::Copy;
        }
        if mode_button(ui, "MV", *mode == OpMode::Move).clicked() && !busy {
            *mode = OpMode::Move;
        }
        ui.separator();
        ui.label(RichText::new(match *mode {
            OpMode::Copy => "Drag to copy on remote",
            OpMode::Move => "Drag to move on remote",
        })
        .size(18.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(
                    !busy,
                    egui::Button::new(RichText::new("Disconnect").size(18.0))
                        .min_size(Vec2::new(120.0, 38.0)),
                )
                .clicked()
            {
                disconnect = true;
            }
            ui.label(RichText::new(host_label).size(18.0));
        });
    });
    ui.add_space(4.0);

    ToolbarResult { disconnect }
}
