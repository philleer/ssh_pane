//! Single pane: path bar + file list + drag/drop.

use egui::{Id, RichText, Vec2};

use ssh_split_pane::pathutil;
use ssh_split_pane::ssh_worker::{PaneSide, RemoteEntry};

/// Payload carried during an egui drag.
#[derive(Clone, Debug)]
pub struct DragPayload {
    pub side: PaneSide,
    pub remote_path: String,
    #[allow(dead_code)]
    pub is_dir: bool,
    #[allow(dead_code)]
    pub name: String,
}

pub struct PaneState {
    pub side: PaneSide,
    pub cwd: String,
    pub path_edit: String,
    pub entries: Vec<RemoteEntry>,
    pub loading: bool,
    pub error: Option<String>,
    pub selected: Option<String>,
}

impl PaneState {
    pub fn new(side: PaneSide) -> Self {
        Self {
            side,
            cwd: "/".into(),
            path_edit: "/".into(),
            entries: Vec::new(),
            loading: false,
            error: None,
            selected: None,
        }
    }

    pub fn set_path(&mut self, path: String) {
        self.cwd = path.clone();
        self.path_edit = path;
        self.selected = None;
    }
}

pub struct PaneAction {
    pub go_path: Option<String>,
    pub drop: Option<DragPayload>,
    pub new_folder: bool,
    pub activated: bool,
}

pub fn show_pane(ui: &mut egui::Ui, pane: &mut PaneState, allow_dnd: bool) -> PaneAction {
    let mut action = PaneAction {
        go_path: None,
        drop: None,
        new_folder: false,
        activated: false,
    };

    let title = match pane.side {
        PaneSide::Left => "Left",
        PaneSide::Right => "Right",
    };

    // Fill the allocated dual-pane slot so the frame grows with maximize
    // instead of leaving the old content-sized outline behind.
    let avail = ui.available_size();
    ui.set_min_size(avail);

    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let stroke = if maximized {
        // Maximized: no group stroke — old Frame::group outline was the "pre-maximize rim".
        egui::Stroke::NONE
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let frame = egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(stroke)
        .rounding(if maximized { 0.0 } else { 6.0 })
        .inner_margin(egui::Margin::same(12.0));

    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        ui.horizontal(|ui| {
            ui.heading(title);
            ui.add_space(8.0);
            if ui
                .add_enabled(
                    !pane.loading,
                    egui::Button::new(RichText::new("⬆ Up").size(18.0)).min_size(Vec2::new(72.0, 36.0)),
                )
                .clicked()
            {
                action.go_path = Some(pathutil::parent(&pane.cwd));
            }
            if ui
                .add_enabled(
                    !pane.loading,
                    egui::Button::new(RichText::new("⟳").size(18.0)).min_size(Vec2::new(44.0, 36.0)),
                )
                .on_hover_text("Refresh")
                .clicked()
            {
                action.go_path = Some(pane.cwd.clone());
            }
            if ui
                .add_enabled(
                    !pane.loading && allow_dnd,
                    egui::Button::new(RichText::new("+ Folder").size(18.0))
                        .min_size(Vec2::new(108.0, 36.0)),
                )
                .on_hover_text("Create a new folder here")
                .clicked()
            {
                action.new_folder = true;
            }
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut pane.path_edit)
                    .desired_width(ui.available_width() - 72.0)
                    .desired_rows(1)
                    .margin(egui::Margin::symmetric(10.0, 8.0))
                    .hint_text("/remote/path"),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter
                || ui
                    .add(egui::Button::new(RichText::new("Go").size(18.0)).min_size(Vec2::new(60.0, 36.0)))
                    .clicked()
            {
                let p = pane.path_edit.trim();
                if !p.is_empty() {
                    action.go_path = Some(pathutil::normalize(p));
                }
            }
        });

        if pane.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Loading…").size(18.0));
            });
        }
        if let Some(err) = &pane.error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
        }

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let list_id = Id::new(("pane_list", pane.side as u8));
        let min_h = ui.available_height().max(240.0);

        let scroll_out = egui::ScrollArea::vertical()
            .id_salt(list_id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_height(min_h);
                ui.set_min_width(ui.available_width());

                if pane.entries.is_empty() && !pane.loading {
                    ui.weak(RichText::new("(empty)").size(18.0));
                }

                for entry in &pane.entries {
                    let full_path = pathutil::join(&pane.cwd, &entry.name);
                    let label = if entry.is_dir {
                        format!("📁  {}", entry.name)
                    } else {
                        format!("📄  {}  ({})", entry.name, format_size(entry.size))
                    };

                    let selected = pane.selected.as_deref() == Some(entry.name.as_str());
                    let resp = ui.add_sized(
                        [ui.available_width(), 36.0],
                        egui::SelectableLabel::new(selected, RichText::new(label).size(19.0)),
                    );

                    if resp.clicked() {
                        pane.selected = Some(entry.name.clone());
                        action.activated = true;
                    }
                    if resp.double_clicked() && entry.is_dir {
                        action.go_path = Some(full_path.clone());
                    }

                    if allow_dnd {
                        let payload = DragPayload {
                            side: pane.side,
                            remote_path: full_path,
                            is_dir: entry.is_dir,
                            name: entry.name.clone(),
                        };
                        resp.dnd_set_drag_payload(payload);
                    }
                }
            });

        let drop_resp = ui.interact(
            scroll_out.inner_rect,
            list_id.with("drop_target"),
            egui::Sense::hover(),
        );

        if allow_dnd {
            if let Some(payload) = drop_resp.dnd_hover_payload::<DragPayload>() {
                if payload.side != pane.side {
                    ui.painter().rect_stroke(
                        scroll_out.inner_rect,
                        6.0,
                        egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(40, 110, 180)),
                    );
                }
            }
            if let Some(payload) = drop_resp.dnd_release_payload::<DragPayload>() {
                if payload.side != pane.side {
                    action.drop = Some((*payload).clone());
                }
            }
        }
    });

    action
}

fn format_size(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.1} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{n} B")
    }
}
