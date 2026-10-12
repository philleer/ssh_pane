//! Single pane: path bar + file list + drag/drop + optional image preview.

use std::collections::BTreeSet;
use std::sync::Arc;

use egui::{CursorIcon, Id, RichText, Sense, Vec2};

use ssh_pane::pathutil;
use ssh_pane::ssh_worker::{PaneSide, RemoteEntry};

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
    /// Multi-select (Ctrl+click). Ordered for stable UI.
    pub selected: BTreeSet<String>,
    /// Last focused name (preview / primary highlight).
    pub focus: Option<String>,
    /// Image preview toggle (OFF / ON).
    pub preview_enabled: bool,
    pub preview_loading: bool,
    pub preview_error: Option<String>,
    /// Remote path currently requested or shown.
    pub preview_path: Option<String>,
    pub preview_texture: Option<egui::TextureHandle>,
    /// Fraction of list+preview height used by the preview panel (0.2–0.75).
    pub preview_ratio: f32,
    /// Case-insensitive substring filter for the file list.
    pub filter: String,
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
            selected: BTreeSet::new(),
            focus: None,
            preview_enabled: false,
            preview_loading: false,
            preview_error: None,
            preview_path: None,
            preview_texture: None,
            preview_ratio: 0.40,
            filter: String::new(),
        }
    }

    pub fn set_path(&mut self, path: String) {
        if self.cwd != path {
            self.clear_selection();
            self.clear_preview_content();
        }
        self.cwd = path.clone();
        self.path_edit = path;
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.focus = None;
    }

    pub fn clear_preview_content(&mut self) {
        self.preview_loading = false;
        self.preview_error = None;
        self.preview_path = None;
        self.preview_texture = None;
    }

    pub fn set_preview_enabled(&mut self, enabled: bool) {
        self.preview_enabled = enabled;
        if !enabled {
            self.clear_preview_content();
        }
    }

    /// Drop selection entries that no longer exist after a refresh.
    pub fn prune_selection(&mut self) {
        let names: BTreeSet<_> = self.entries.iter().map(|e| e.name.clone()).collect();
        self.selected.retain(|n| names.contains(n));
        if let Some(f) = &self.focus {
            if !self.selected.contains(f) {
                self.focus = self.selected.iter().next().cloned();
            }
        }
    }

    pub fn selected_paths(&self) -> Vec<String> {
        self.selected
            .iter()
            .map(|n| pathutil::join(&self.cwd, n))
            .collect()
    }

    /// Entries visible under the current filter (case-insensitive substring).
    pub fn filtered_entries(&self) -> Vec<&RemoteEntry> {
        let needle = self.filter.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return self.entries.iter().collect();
        }
        self.entries
            .iter()
            .filter(|e| e.name.to_ascii_lowercase().contains(&needle))
            .collect()
    }
}

pub struct PaneAction {
    pub go_path: Option<String>,
    pub drop: Option<DragPayload>,
    pub new_folder: bool,
    pub activated: bool,
    /// Selection or preview toggle changed — app should sync preview.
    pub preview_sync: bool,
    /// Batch-move selected items to the other pane (full remote paths).
    pub batch_move: Option<Vec<String>>,
}

pub fn is_image_filename(name: &str) -> bool {
    let Some(ext) = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
    else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
    )
}

pub fn show_pane(ui: &mut egui::Ui, pane: &mut PaneState, allow_dnd: bool) -> PaneAction {
    let mut action = PaneAction {
        go_path: None,
        drop: None,
        new_folder: false,
        activated: false,
        preview_sync: false,
        batch_move: None,
    };

    // Fill the allocated dual-pane slot so the frame grows with maximize
    // instead of leaving the old content-sized outline behind.
    let avail = ui.available_size();
    ui.set_min_size(avail);
    // Clip children so toolbar/path never paint past the pane edge.
    ui.set_clip_rect(ui.max_rect());

    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    let stroke = if maximized {
        egui::Stroke::NONE
    } else {
        ui.visuals().widgets.noninteractive.bg_stroke
    };
    let frame = egui::Frame::none()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(stroke)
        .rounding(if maximized { 0.0 } else { 6.0 })
        .inner_margin(egui::Margin::same(8.0));

    frame.show(ui, |ui| {
        ui.set_min_size(ui.available_size());
        ui.set_max_width(ui.available_width());

        // Toolbar: wrap when the pane is narrow (esp. maximized dual-pane).
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !pane.loading,
                    egui::Button::new(RichText::new("⬆ Up").size(17.0))
                        .min_size(Vec2::new(64.0, 32.0)),
                )
                .clicked()
            {
                action.go_path = Some(pathutil::parent(&pane.cwd));
            }
            if ui
                .add_enabled(
                    !pane.loading,
                    egui::Button::new(RichText::new("⟳").size(17.0))
                        .min_size(Vec2::new(40.0, 32.0)),
                )
                .on_hover_text("Refresh")
                .clicked()
            {
                action.go_path = Some(pane.cwd.clone());
            }
            if ui
                .add_enabled(
                    !pane.loading && allow_dnd,
                    egui::Button::new(RichText::new("+ Folder").size(17.0))
                        .min_size(Vec2::new(96.0, 32.0)),
                )
                .on_hover_text("Create a new folder here")
                .clicked()
            {
                action.new_folder = true;
            }

            let label = if pane.preview_enabled {
                "Preview ON"
            } else {
                "Preview OFF"
            };
            let mut btn = egui::Button::new(RichText::new(label).size(15.0))
                .min_size(Vec2::new(112.0, 32.0));
            if pane.preview_enabled {
                btn = btn.fill(egui::Color32::from_rgb(40, 110, 180));
            }
            if ui
                .add(btn)
                .on_hover_text("Toggle image preview for this pane")
                .clicked()
            {
                pane.set_preview_enabled(!pane.preview_enabled);
                action.preview_sync = true;
            }

            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut pane.filter)
                    .desired_width(120.0)
                    .desired_rows(1)
                    .margin(egui::Margin::symmetric(6.0, 6.0))
                    .hint_text("filter…"),
            );
            if !pane.filter.is_empty()
                && ui
                    .add(
                        egui::Button::new(RichText::new("✕").size(15.0))
                            .min_size(Vec2::new(28.0, 28.0)),
                    )
                    .on_hover_text("Clear filter")
                    .clicked()
            {
                pane.filter.clear();
            }
        });

        ui.add_space(4.0);

        // Path row: leave room for Go; clamp edit width to pane.
        ui.horizontal(|ui| {
            let go_w = 56.0;
            let edit_w = (ui.available_width() - go_w - 8.0).max(48.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut pane.path_edit)
                    .desired_width(edit_w)
                    .desired_rows(1)
                    .margin(egui::Margin::symmetric(8.0, 6.0))
                    .hint_text("/remote/path"),
            );
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter
                || ui
                    .add(
                        egui::Button::new(RichText::new("Go").size(17.0))
                            .min_size(Vec2::new(go_w, 32.0)),
                    )
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
                ui.label(RichText::new("Loading…").size(17.0));
            });
        }
        if let Some(err) = &pane.error {
            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
        }

        let total = pane.entries.len();
        let shown = if pane.filter.trim().is_empty() {
            total
        } else {
            pane.filtered_entries().len()
        };
        let count_text = if pane.filter.trim().is_empty() {
            if total == 1 {
                "1 item".to_string()
            } else {
                format!("{total} items")
            }
        } else {
            format!("{shown} / {total}")
        };
        ui.label(RichText::new(count_text).size(12.0).weak());

        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);

        const SPLITTER_H: f32 = 8.0;
        const MIN_LIST: f32 = 100.0;
        const MIN_PREVIEW: f32 = 100.0;

        let avail_h = ui.available_height();
        let (list_h, preview_h) = if pane.preview_enabled {
            let usable = (avail_h - SPLITTER_H).max(MIN_LIST + MIN_PREVIEW);
            let ratio = pane.preview_ratio.clamp(0.20, 0.75);
            let mut preview_h = (usable * ratio).clamp(MIN_PREVIEW, usable - MIN_LIST);
            let mut list_h = usable - preview_h;
            if list_h < MIN_LIST {
                list_h = MIN_LIST;
                preview_h = usable - list_h;
            }
            (list_h, preview_h)
        } else {
            (avail_h.max(MIN_LIST), 0.0)
        };

        let list_id = Id::new(("pane_list", pane.side as u8));
        let list_rect = show_file_list(
            ui,
            pane,
            allow_dnd,
            list_id,
            list_h,
            &mut action,
        );

        // Drop target: detect by pointer-in-rect so row widgets cannot steal hover.
        if allow_dnd {
            if let Some(payload) = handle_list_drop(ui, pane.side, list_rect) {
                action.drop = Some(payload);
            }
        }

        if pane.preview_enabled {
            // Draggable splitter between list and preview.
            let splitter_id = Id::new(("preview_split", pane.side as u8));
            let splitter_resp = ui
                .scope_builder(
                    egui::UiBuilder::new().id_salt(splitter_id),
                    |ui| {
                        let (_, resp) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), SPLITTER_H),
                            Sense::click_and_drag(),
                        );
                        resp
                    },
                )
                .inner;
            // Paint handle
            let visuals = ui.style().visuals.clone();
            let fill = if splitter_resp.hovered() || splitter_resp.dragged() {
                visuals.widgets.hovered.bg_fill
            } else {
                visuals.widgets.noninteractive.bg_fill
            };
            ui.painter()
                .rect_filled(splitter_resp.rect, 2.0, fill);
            let mid_y = splitter_resp.rect.center().y;
            let mid_x = splitter_resp.rect.center().x;
            ui.painter().hline(
                mid_x - 18.0..=mid_x + 18.0,
                mid_y,
                egui::Stroke::new(2.0_f32, visuals.widgets.noninteractive.fg_stroke.color),
            );
            if splitter_resp.hovered() || splitter_resp.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
            }
            // Dragging down → larger list / smaller preview.
            if splitter_resp.dragged() {
                let usable = (list_h + preview_h).max(1.0);
                let new_preview = (preview_h - splitter_resp.drag_delta().y)
                    .clamp(MIN_PREVIEW, usable - MIN_LIST);
                pane.preview_ratio = new_preview / usable;
                ui.ctx().request_repaint();
            }

            show_preview_area(ui, pane, preview_h);
        }
    });

    action
}

fn show_file_list(
    ui: &mut egui::Ui,
    pane: &mut PaneState,
    allow_dnd: bool,
    list_id: Id,
    list_h: f32,
    action: &mut PaneAction,
) -> egui::Rect {
    let scroll_out = egui::ScrollArea::vertical()
        .id_salt(list_id)
        .max_height(list_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_height(list_h);
            ui.set_min_width(ui.available_width());

            let filtered = pane.filtered_entries();
            let total = pane.entries.len();
            let shown = filtered.len();

            if total == 0 && !pane.loading {
                ui.weak(RichText::new("(empty)").size(17.0));
            } else if shown == 0 && !pane.loading {
                ui.weak(RichText::new("(no matches)").size(17.0));
            }

            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);

            // Owned copies so we can mutate selection while iterating.
            let entries: Vec<RemoteEntry> = filtered.into_iter().cloned().collect();
            for entry in &entries {
                let full_path = pathutil::join(&pane.cwd, &entry.name);
                let label = if entry.is_dir {
                    format!("📁  {}", entry.name)
                } else if is_image_filename(&entry.name) {
                    format!("🖼  {}  ({})", entry.name, format_size(entry.size))
                } else {
                    format!("📄  {}  ({})", entry.name, format_size(entry.size))
                };

                let selected = pane.selected.contains(&entry.name);
                let resp = file_row(ui, &label, selected);

                if resp.clicked() {
                    if ctrl {
                        if pane.selected.contains(&entry.name) {
                            pane.selected.remove(&entry.name);
                            if pane.focus.as_deref() == Some(entry.name.as_str()) {
                                pane.focus = pane.selected.iter().next().cloned();
                            }
                        } else {
                            pane.selected.insert(entry.name.clone());
                            pane.focus = Some(entry.name.clone());
                        }
                    } else {
                        pane.selected.clear();
                        pane.selected.insert(entry.name.clone());
                        pane.focus = Some(entry.name.clone());
                    }
                    action.activated = true;
                    action.preview_sync = true;
                }

                if resp.secondary_clicked() {
                    // Right-click: ensure the row is part of the selection.
                    if !pane.selected.contains(&entry.name) {
                        if !ctrl {
                            pane.selected.clear();
                        }
                        pane.selected.insert(entry.name.clone());
                        pane.focus = Some(entry.name.clone());
                        action.preview_sync = true;
                    }
                    action.activated = true;
                }

                if resp.double_clicked() && entry.is_dir {
                    action.go_path = Some(full_path.clone());
                }

                // Context menu for batch move.
                let n_selected = pane.selected.len();
                resp.context_menu(|ui| {
                    ui.set_min_width(220.0);
                    if n_selected == 0 {
                        ui.label(RichText::new("No selection").size(15.0));
                        return;
                    }
                    let label = if n_selected == 1 {
                        "Move to other pane".to_string()
                    } else {
                        format!("Move {n_selected} items to other pane")
                    };
                    if ui
                        .add_enabled(
                            allow_dnd,
                            egui::Button::new(RichText::new(label).size(16.0)),
                        )
                        .on_hover_text("Move selected items into the other pane’s current folder")
                        .clicked()
                    {
                        action.batch_move = Some(pane.selected_paths());
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.label(
                        RichText::new("Tip: Ctrl+click to multi-select")
                            .size(13.0)
                            .weak(),
                    );
                });

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

    scroll_out.inner_rect
}

/// List row with click + drag sense (SelectableLabel alone is click-only → DnD never starts).
fn file_row(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let desired = Vec2::new(ui.available_width(), 34.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());

    let visuals = ui.style().interact(&resp);
    let bg = if selected {
        ui.visuals().selection.bg_fill
    } else if resp.hovered() || resp.dragged() {
        visuals.bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    if bg.a() > 0 {
        ui.painter()
            .rect_filled(rect, visuals.rounding, bg);
    }

    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(18.0),
        if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().widgets.noninteractive.fg_stroke.color
        },
    );
    let text_pos = egui::pos2(
        rect.min.x + 8.0,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, visuals.text_color());

    resp
}

fn handle_list_drop(ui: &mut egui::Ui, side: PaneSide, list_rect: egui::Rect) -> Option<DragPayload> {
    if !ui.rect_contains_pointer(list_rect) {
        return None;
    }

    let hovering = egui::DragAndDrop::payload::<DragPayload>(ui.ctx()).and_then(|p| {
        if p.side != side {
            Some(p)
        } else {
            None
        }
    });

    if hovering.is_some() {
        ui.painter().rect_stroke(
            list_rect,
            6.0,
            egui::Stroke::new(3.0_f32, egui::Color32::from_rgb(40, 110, 180)),
        );
    }

    let released = ui.input(|i| i.pointer.any_released());
    if released {
        if let Some(payload) = egui::DragAndDrop::take_payload::<DragPayload>(ui.ctx()) {
            if payload.side != side {
                return Some(Arc::unwrap_or_clone(payload));
            }
        }
    }
    None
}

fn show_preview_area(ui: &mut egui::Ui, pane: &PaneState, preview_h: f32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Image preview").size(15.0).strong());
        if pane.preview_loading {
            ui.spinner();
            ui.label(RichText::new("Loading…").size(14.0));
        }
    });

    if let Some(err) = &pane.preview_error {
        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
    }

    let header_used = 28.0;
    let body_h = (preview_h - header_used).max(60.0);

    let frame = egui::Frame::none()
        .fill(ui.visuals().faint_bg_color)
        .rounding(4.0)
        .inner_margin(egui::Margin::same(6.0));

    frame.show(ui, |ui| {
        ui.set_min_height(body_h);
        ui.set_max_height(body_h);
        ui.set_min_width(ui.available_width());

        if let Some(tex) = &pane.preview_texture {
            let avail = ui.available_size();
            let size = tex.size_vec2();
            let scale = (avail.x / size.x).min(avail.y / size.y).min(1.0);
            let drawn = size * scale;
            ui.centered_and_justified(|ui| {
                ui.image((tex.id(), drawn));
            });
        } else if !pane.preview_loading && pane.preview_error.is_none() {
            ui.centered_and_justified(|ui| {
                ui.weak(
                    RichText::new("Select an image (png / jpg / gif / webp / bmp)").size(15.0),
                );
            });
        }
    });
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
