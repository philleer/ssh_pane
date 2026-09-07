//! Application state machine.

use eframe::egui::{self, FontFamily, FontId, RichText, TextStyle};
use log::{info, warn};

use ssh_split_pane::ops;
use ssh_split_pane::ssh_worker::{
    OpMode, PaneSide, WorkerEvent, WorkerHandle, WorkerRequest,
};

use crate::ui::connect::ConnectForm;
use crate::ui::menu;
use crate::ui::pane::{self, is_image_filename, PaneState};
use crate::ui::toolbar;

#[derive(PartialEq, Eq)]
enum Screen {
    Connect,
    Browser,
}

struct OverwritePrompt {
    src: String,
    dst_dir: String,
    mode: OpMode,
    existing_path: String,
}

struct NewFolderPrompt {
    side: PaneSide,
    parent: String,
    name: String,
    error: Option<String>,
}

pub struct App {
    screen: Screen,
    connect: ConnectForm,
    worker: WorkerHandle,
    mode: OpMode,
    left: PaneState,
    right: PaneState,
    active_pane: PaneSide,
    host_label: String,
    status: String,
    busy: bool,
    overwrite: Option<OverwritePrompt>,
    new_folder: Option<NewFolderPrompt>,
    show_about: bool,
    /// Previous frame maximized flag — used to force a full redraw on toggle.
    was_maximized: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        ssh_split_pane::fonts::install_cjk_fonts(&cc.egui_ctx);
        apply_large_style(&cc.egui_ctx);
        Self {
            screen: Screen::Connect,
            connect: ConnectForm::new(),
            worker: WorkerHandle::spawn(),
            mode: OpMode::Copy,
            left: PaneState::new(PaneSide::Left),
            right: PaneState::new(PaneSide::Right),
            active_pane: PaneSide::Left,
            host_label: String::new(),
            status: "Ready".into(),
            busy: false,
            overwrite: None,
            new_folder: None,
            show_about: false,
            was_maximized: false,
        }
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        while let Some(ev) = self.worker.try_recv() {
            self.handle_event(ctx, ev);
        }
    }

    fn handle_event(&mut self, ctx: &egui::Context, ev: WorkerEvent) {
        match ev {
            WorkerEvent::Connected { home } => {
                self.connect.on_connected();
                self.screen = Screen::Browser;
                self.host_label = format!(
                    "{}@{}:{}",
                    self.connect.user.trim(),
                    self.connect.host.trim(),
                    self.connect.port.trim()
                );
                self.status = format!("Connected. Home: {home}");
                info!("UI connected to {} home={home}", self.host_label);
                self.busy = false;
                self.request_list(PaneSide::Left, home.clone());
                self.request_list(PaneSide::Right, home);
            }
            WorkerEvent::ConnectFailed(msg) => {
                warn!("UI connect failed: {msg}");
                self.connect.on_connect_failed(msg);
                self.busy = false;
            }
            WorkerEvent::Disconnected => {
                self.screen = Screen::Connect;
                self.status = "Disconnected".into();
                self.busy = false;
                self.left = PaneState::new(PaneSide::Left);
                self.right = PaneState::new(PaneSide::Right);
                self.new_folder = None;
            }
            WorkerEvent::Listed {
                side,
                path,
                entries,
            } => {
                let pane = self.pane_mut(side);
                pane.loading = false;
                pane.error = None;
                pane.set_path(path);
                pane.entries = entries;
            }
            WorkerEvent::ListFailed { side, path, error } => {
                let pane = self.pane_mut(side);
                pane.loading = false;
                pane.error = Some(error.clone());
                pane.path_edit = path;
                self.status = format!("List failed: {error}");
            }
            WorkerEvent::NeedsOverwriteConfirm {
                src,
                dst_dir,
                mode,
                existing_path,
            } => {
                self.busy = false;
                self.status = format!("Target exists: {existing_path}");
                self.overwrite = Some(OverwritePrompt {
                    src,
                    dst_dir,
                    mode,
                    existing_path,
                });
            }
            WorkerEvent::TransferOk { src, dst_dir, mode } => {
                self.busy = false;
                self.status = format!("{} ok: {} → {}", mode.label(), src, dst_dir);
                self.request_list(PaneSide::Left, self.left.cwd.clone());
                self.request_list(PaneSide::Right, self.right.cwd.clone());
            }
            WorkerEvent::TransferFailed { error } => {
                self.busy = false;
                self.status = format!("Transfer failed: {error}");
            }
            WorkerEvent::MkdirOk { side, path } => {
                self.busy = false;
                self.status = format!("Created folder: {path}");
                self.new_folder = None;
                let cwd = self.pane_mut(side).cwd.clone();
                self.request_list(side, cwd);
            }
            WorkerEvent::MkdirFailed { error } => {
                self.busy = false;
                if let Some(prompt) = &mut self.new_folder {
                    prompt.error = Some(error.clone());
                }
                self.status = format!("mkdir failed: {error}");
            }
            WorkerEvent::FileReadOk { side, path, bytes } => {
                let pane = self.pane_mut(side);
                if !pane.preview_enabled || pane.preview_path.as_deref() != Some(path.as_str()) {
                    return;
                }
                match decode_preview_texture(ctx, &path, &bytes.0) {
                    Ok(tex) => {
                        pane.preview_loading = false;
                        pane.preview_error = None;
                        pane.preview_texture = Some(tex);
                        self.status = format!("Preview: {path}");
                    }
                    Err(e) => {
                        pane.preview_loading = false;
                        pane.preview_texture = None;
                        pane.preview_error = Some(e.clone());
                        self.status = format!("Preview failed: {e}");
                    }
                }
            }
            WorkerEvent::FileReadFailed { side, path, error } => {
                let pane = self.pane_mut(side);
                if pane.preview_path.as_deref() == Some(path.as_str()) {
                    pane.preview_loading = false;
                    pane.preview_texture = None;
                    pane.preview_error = Some(error.clone());
                }
                self.status = format!("Preview failed: {error}");
            }
            WorkerEvent::Busy(msg) => {
                self.busy = true;
                self.status = msg;
            }
        }
    }

    fn sync_preview(&mut self, side: PaneSide) {
        const MAX_PREVIEW_BYTES: u64 = 12 * 1024 * 1024;

        let (enabled, selected, cwd, entries, current_path, loading, has_tex) = {
            let pane = self.pane_mut(side);
            (
                pane.preview_enabled,
                pane.selected.clone(),
                pane.cwd.clone(),
                pane.entries.clone(),
                pane.preview_path.clone(),
                pane.preview_loading,
                pane.preview_texture.is_some(),
            )
        };

        if !enabled {
            self.pane_mut(side).clear_preview_content();
            return;
        }

        let Some(name) = selected else {
            let pane = self.pane_mut(side);
            pane.preview_loading = false;
            pane.preview_error = None;
            pane.preview_path = None;
            pane.preview_texture = None;
            return;
        };

        let Some(entry) = entries.iter().find(|e| e.name == name) else {
            return;
        };

        if entry.is_dir || !is_image_filename(&name) {
            let pane = self.pane_mut(side);
            pane.preview_loading = false;
            pane.preview_path = None;
            pane.preview_texture = None;
            pane.preview_error = Some("Select an image file to preview".into());
            return;
        }

        let path = ssh_split_pane::pathutil::join(&cwd, &name);
        if current_path.as_deref() == Some(path.as_str()) && (loading || has_tex) {
            return;
        }

        let pane = self.pane_mut(side);
        pane.preview_loading = true;
        pane.preview_error = None;
        pane.preview_texture = None;
        pane.preview_path = Some(path.clone());
        info!("preview request side={side:?} path={path}");
        self.worker.send(WorkerRequest::ReadFile {
            side,
            path,
            max_bytes: MAX_PREVIEW_BYTES,
        });
    }

    fn pane_mut(&mut self, side: PaneSide) -> &mut PaneState {
        match side {
            PaneSide::Left => &mut self.left,
            PaneSide::Right => &mut self.right,
        }
    }

    fn request_list(&mut self, side: PaneSide, path: String) {
        let pane = self.pane_mut(side);
        pane.loading = true;
        pane.error = None;
        pane.path_edit = path.clone();
        self.worker.send(WorkerRequest::ListDir { side, path });
    }

    fn start_transfer(&mut self, src_side: PaneSide, src_path: String) {
        if self.busy {
            return;
        }
        let dst_dir = match src_side {
            PaneSide::Left => self.right.cwd.clone(),
            PaneSide::Right => self.left.cwd.clone(),
        };
        if ops::is_same_directory_transfer(&src_path, &dst_dir) {
            self.status = ops::OpsError::SameDirectory.to_string();
            warn!("{}", self.status);
            return;
        }
        self.busy = true;
        self.status = format!("{} {} → {}", self.mode.label(), src_path, dst_dir);
        info!("start transfer: {}", self.status);
        self.worker.send(WorkerRequest::CheckExists {
            src: src_path,
            dst_dir,
            mode: self.mode,
            overwrite: false,
        });
    }

    fn open_new_folder_dialog(&mut self, side: PaneSide) {
        if self.busy {
            return;
        }
        let parent = self.pane_mut(side).cwd.clone();
        self.active_pane = side;
        info!("open new-folder dialog side={side:?} parent={parent}");
        self.new_folder = Some(NewFolderPrompt {
            side,
            parent,
            name: String::new(),
            error: None,
        });
    }

    fn submit_new_folder(&mut self) {
        let Some(prompt) = &self.new_folder else {
            return;
        };
        if let Err(e) = ops::validate_folder_name(&prompt.name) {
            if let Some(p) = &mut self.new_folder {
                p.error = Some(e.to_string());
            }
            return;
        }
        let name = prompt.name.trim().to_string();
        let side = prompt.side;
        let parent = prompt.parent.clone();
        self.busy = true;
        info!("mkdir request side={side:?} parent={parent} name={name}");
        self.worker.send(WorkerRequest::Mkdir {
            side,
            parent,
            name,
        });
    }

    fn show_overwrite_modal(&mut self, ctx: &egui::Context) {
        let Some(prompt) = &self.overwrite else {
            return;
        };
        let existing = prompt.existing_path.clone();
        let mut overwrite = false;
        let mut cancel = false;

        egui::Window::new("Overwrite?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label(RichText::new(format!(
                    "Target already exists:\n{existing}\n\nOverwrite?"
                )).size(18.0));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Overwrite").size(18.0))
                                .min_size(egui::vec2(120.0, 38.0)),
                        )
                        .clicked()
                    {
                        overwrite = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Cancel").size(18.0))
                                .min_size(egui::vec2(110.0, 38.0)),
                        )
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });

        if overwrite {
            if let Some(p) = self.overwrite.take() {
                self.busy = true;
                self.worker.send(WorkerRequest::Transfer {
                    src: p.src,
                    dst_dir: p.dst_dir,
                    mode: p.mode,
                    overwrite: true,
                });
            }
        } else if cancel {
            self.overwrite = None;
            self.status = "Cancelled".into();
        }
    }

    fn show_new_folder_modal(&mut self, ctx: &egui::Context) {
        let Some(prompt) = &self.new_folder else {
            return;
        };
        let parent = prompt.parent.clone();
        let side_label = match prompt.side {
            PaneSide::Left => "Left",
            PaneSide::Right => "Right",
        };
        let mut create = false;
        let mut cancel = false;

        egui::Window::new(format!("New Folder ({side_label})"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                ui.label(RichText::new(format!("Parent: {parent}")).size(17.0));
                ui.add_space(8.0);
                ui.label(RichText::new("Folder name").size(18.0));
                let resp = ui.add(
                    egui::TextEdit::singleline(
                        &mut self.new_folder.as_mut().unwrap().name,
                    )
                    .desired_width(380.0)
                    .margin(egui::Margin::symmetric(10.0, 8.0))
                    .hint_text("new_folder"),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    create = true;
                }
                if let Some(err) = &self.new_folder.as_ref().unwrap().error {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.busy,
                            egui::Button::new(RichText::new("Create").size(18.0))
                                .min_size(egui::vec2(110.0, 38.0)),
                        )
                        .clicked()
                    {
                        create = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Cancel").size(18.0))
                                .min_size(egui::vec2(110.0, 38.0)),
                        )
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });

        if create {
            self.submit_new_folder();
        } else if cancel {
            self.new_folder = None;
        }
    }

    fn show_about_modal(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        egui::Window::new("About SSH Split Pane")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                ui.heading("SSH Split Pane");
                ui.label(RichText::new("v0.1.0").size(17.0));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "Remote dual-pane file manager.\nDrag between panes to CP/MV on the same SSH host.",
                    )
                    .size(18.0),
                );
                ui.add_space(12.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("OK").size(18.0))
                            .min_size(egui::vec2(88.0, 38.0)),
                    )
                    .clicked()
                {
                    self.show_about = false;
                }
            });
    }

    fn apply_menu_actions(&mut self, actions: menu::MenuActions, ctx: &egui::Context) {
        if let Some(mode) = actions.set_mode {
            self.mode = mode;
        }
        if actions.refresh_left {
            self.request_list(PaneSide::Left, self.left.cwd.clone());
        }
        if actions.refresh_right {
            self.request_list(PaneSide::Right, self.right.cwd.clone());
        }
        if actions.refresh_both {
            self.request_list(PaneSide::Left, self.left.cwd.clone());
            self.request_list(PaneSide::Right, self.right.cwd.clone());
        }
        if let Some(side) = actions.new_folder {
            self.open_new_folder_dialog(side);
        }
        if actions.disconnect {
            self.worker.send(WorkerRequest::Disconnect);
        }
        if actions.show_about {
            self.show_about = true;
        }
        if actions.exit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

fn decode_preview_texture(
    ctx: &egui::Context,
    path: &str,
    bytes: &[u8],
) -> Result<egui::TextureHandle, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("decode image: {e}"))?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    if size[0] == 0 || size[1] == 0 {
        return Err("empty image".into());
    }
    let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Ok(ctx.load_texture(
        format!("preview:{path}"),
        color,
        egui::TextureOptions::LINEAR,
    ))
}

fn apply_large_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Small, FontId::new(15.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(28.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(17.0, FontFamily::Monospace)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.interact_size = egui::vec2(52.0, 30.0);
    style.spacing.window_margin = egui::Margin::same(14.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.spacing.indent = 22.0;
    // Keep expansion at 0 so widget strokes cannot paint outside the
    // allocated rect — that leftover stroke looks like a pre-maximize outline.
    style.visuals.widgets.noninteractive.expansion = 0.0;
    style.visuals.widgets.inactive.expansion = 0.0;
    style.visuals.widgets.hovered.expansion = 0.0;
    style.visuals.widgets.active.expansion = 0.0;
    style.visuals.window_stroke = egui::Stroke::NONE;
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
    ctx.set_style(style);
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        if maximized != self.was_maximized {
            self.was_maximized = maximized;
            ctx.request_repaint();
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        self.poll_worker(ctx);
        self.show_overwrite_modal(ctx);
        self.show_new_folder_modal(ctx);
        self.show_about_modal(ctx);

        let fill = ctx.style().visuals.panel_fill;
        let flat = egui::Frame::none()
            .fill(fill)
            .stroke(egui::Stroke::NONE)
            .rounding(0.0)
            .inner_margin(egui::Margin::ZERO)
            .outer_margin(egui::Margin::ZERO);

        // Custom chrome — OS decorations are disabled in main.rs so CSD/DWM
        // drop-shadows cannot linger as a pre-maximize outline.
        egui::TopBottomPanel::top("titlebar")
            .exact_height(36.0)
            .frame(flat)
            .show(ctx, |ui| {
                crate::ui::titlebar::show(ui, "SSH Split Pane", maximized);
            });

        egui::TopBottomPanel::top("menubar")
            .frame(
                egui::Frame::none()
                    .fill(fill)
                    .stroke(egui::Stroke::NONE)
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                let connected = self.screen == Screen::Browser;
                let actions = menu::show_menu_bar(
                    ui,
                    connected,
                    self.busy,
                    self.mode,
                    self.active_pane,
                );
                self.apply_menu_actions(actions, ctx);
            });

        match self.screen {
            Screen::Connect => {
                self.connect.show(ctx, &self.worker);
            }
            Screen::Browser => {
                egui::TopBottomPanel::top("toolbar")
                    .frame(
                        egui::Frame::none()
                            .fill(fill)
                            .stroke(egui::Stroke::NONE)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
                    )
                    .show(ctx, |ui| {
                        let r = toolbar::show_toolbar(
                            ui,
                            &mut self.mode,
                            &self.host_label,
                            self.busy,
                        );
                        if r.disconnect {
                            self.worker.send(WorkerRequest::Disconnect);
                        }
                    });

                egui::TopBottomPanel::bottom("status")
                    .exact_height(40.0)
                    .frame(
                        egui::Frame::none()
                            .fill(fill)
                            .stroke(egui::Stroke::NONE)
                            .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal_centered(|ui| {
                            if self.busy {
                                ui.spinner();
                            }
                            ui.label(RichText::new(&self.status).size(17.0));
                        });
                    });

                let central_margin = if maximized {
                    egui::Margin::same(2.0)
                } else {
                    egui::Margin::same(8.0)
                };
                let central_frame = if maximized {
                    egui::Frame::none()
                        .fill(fill)
                        .stroke(egui::Stroke::NONE)
                        .rounding(0.0)
                        .inner_margin(central_margin)
                } else {
                    egui::Frame::none()
                        .fill(fill)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            ctx.style().visuals.widgets.noninteractive.bg_stroke.color,
                        ))
                        .rounding(0.0)
                        .inner_margin(central_margin)
                };

                egui::CentralPanel::default()
                    .frame(central_frame)
                    .show(ctx, |ui| {
                        let full = ui.available_size();
                        ui.set_min_size(full);
                        let avail = ui.available_width();
                        let sep = 8.0;
                        // Two panes + separator must fit exactly; previous half*2 ignored sep → overflow.
                        let half = ((avail - sep) * 0.5).max(120.0);
                        let height = ui.available_height();

                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(half, height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_min_size(ui.available_size());
                                    ui.set_max_size(ui.available_size());
                                    let action =
                                        pane::show_pane(ui, &mut self.left, !self.busy);
                                    if action.activated {
                                        self.active_pane = PaneSide::Left;
                                    }
                                    if action.preview_sync {
                                        self.sync_preview(PaneSide::Left);
                                    }
                                    if let Some(path) = action.go_path {
                                        self.active_pane = PaneSide::Left;
                                        self.request_list(PaneSide::Left, path);
                                    }
                                    if action.new_folder {
                                        self.open_new_folder_dialog(PaneSide::Left);
                                    }
                                    if let Some(payload) = action.drop {
                                        self.start_transfer(
                                            payload.side,
                                            payload.remote_path,
                                        );
                                    }
                                },
                            );

                            let (sep_rect, _) = ui.allocate_exact_size(
                                egui::vec2(sep, height),
                                egui::Sense::hover(),
                            );
                            ui.painter().vline(
                                sep_rect.center().x,
                                sep_rect.y_range(),
                                egui::Stroke::new(
                                    1.0_f32,
                                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                                ),
                            );

                            let right_w = ui.available_width().max(120.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(right_w, height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_min_size(ui.available_size());
                                    ui.set_max_size(ui.available_size());
                                    let action =
                                        pane::show_pane(ui, &mut self.right, !self.busy);
                                    if action.activated {
                                        self.active_pane = PaneSide::Right;
                                    }
                                    if action.preview_sync {
                                        self.sync_preview(PaneSide::Right);
                                    }
                                    if let Some(path) = action.go_path {
                                        self.active_pane = PaneSide::Right;
                                        self.request_list(PaneSide::Right, path);
                                    }
                                    if action.new_folder {
                                        self.open_new_folder_dialog(PaneSide::Right);
                                    }
                                    if let Some(payload) = action.drop {
                                        self.start_transfer(
                                            payload.side,
                                            payload.remote_path,
                                        );
                                    }
                                },
                            );
                        });
                    });
            }
        }

        if self.busy
            || self.connect.connecting
            || self.left.loading
            || self.right.loading
            || self.left.preview_loading
            || self.right.preview_loading
            || egui::DragAndDrop::has_payload_of_type::<pane::DragPayload>(ctx)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.worker.send(WorkerRequest::Shutdown);
    }
}
