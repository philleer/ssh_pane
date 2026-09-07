//! Connection dialog UI.

use egui::{Align, Layout, RichText};

use ssh_split_pane::ssh_worker::{ConnectParams, WorkerHandle, WorkerRequest};

#[derive(Default)]
pub struct ConnectForm {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub private_key: String,
    pub connecting: bool,
    pub error: Option<String>,
}

impl ConnectForm {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: "22".into(),
            user: String::new(),
            password: String::new(),
            private_key: String::new(),
            connecting: false,
            error: None,
        }
    }

    /// Returns true if connect was requested this frame.
    pub fn show(&mut self, ctx: &egui::Context, worker: &WorkerHandle) -> bool {
        let mut requested = false;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(ctx.style().visuals.panel_fill)
                    .stroke(egui::Stroke::NONE),
            )
            .show(ctx, |ui| {
                requested = self.show_in(ui, worker);
            });
        requested
    }

    /// Draw the connect form into an existing `Ui`.
    pub fn show_in(&mut self, ui: &mut egui::Ui, worker: &WorkerHandle) -> bool {
        let mut requested = false;

        ui.vertical_centered(|ui| {
            ui.add_space(48.0);
            ui.heading(RichText::new("SSH Split Pane").size(34.0));
            ui.label(
                RichText::new(
                    "Connect to a remote host, then drag files between panes to CP/MV.",
                )
                .size(18.0),
            );
            ui.add_space(28.0);
        });

        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() * 0.18);
            ui.vertical(|ui| {
                ui.set_max_width(480.0);
                egui::Grid::new("connect_grid")
                    .num_columns(2)
                    .spacing([16.0, 12.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Host").size(18.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.host)
                                .desired_width(320.0)
                                .margin(egui::Margin::symmetric(10.0, 8.0))
                                .hint_text("hostname or IP"),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Port").size(18.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.port)
                                .desired_width(100.0)
                                .margin(egui::Margin::symmetric(10.0, 8.0))
                                .hint_text("22"),
                        );
                        ui.end_row();

                        ui.label(RichText::new("User").size(18.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.user)
                                .desired_width(320.0)
                                .margin(egui::Margin::symmetric(10.0, 8.0)),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Password").size(18.0));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.password)
                                .password(true)
                                .desired_width(320.0)
                                .margin(egui::Margin::symmetric(10.0, 8.0)),
                        );
                        ui.end_row();

                        ui.label(RichText::new("Private key").size(18.0));
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.private_key)
                                    .desired_width(240.0)
                                    .margin(egui::Margin::symmetric(10.0, 8.0))
                                    .hint_text("optional path"),
                            );
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Browse…").size(18.0))
                                        .min_size(egui::vec2(104.0, 38.0)),
                                )
                                .clicked()
                            {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    self.private_key = path.display().to_string();
                                }
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(20.0);

                if let Some(err) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                    ui.add_space(8.0);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let can_connect = !self.connecting
                        && !self.host.trim().is_empty()
                        && !self.user.trim().is_empty();
                    if ui
                        .add_enabled(
                            can_connect,
                            egui::Button::new(RichText::new("Connect").size(20.0))
                                .min_size(egui::vec2(128.0, 44.0)),
                        )
                        .clicked()
                    {
                        match self.build_params() {
                            Ok(params) => {
                                self.connecting = true;
                                self.error = None;
                                worker.send(WorkerRequest::Connect(params));
                                requested = true;
                            }
                            Err(e) => {
                                self.error = Some(e);
                            }
                        }
                    }
                    if self.connecting {
                        ui.spinner();
                        ui.label(RichText::new("Connecting…").size(18.0));
                    }
                });
            });
        });

        requested
    }

    fn build_params(&self) -> Result<ConnectParams, String> {
        let port: u16 = self
            .port
            .trim()
            .parse()
            .map_err(|_| "Port must be a number".to_string())?;
        let key = self.private_key.trim();
        let private_key = if key.is_empty() {
            None
        } else {
            let p = std::path::PathBuf::from(key);
            if !p.is_file() {
                return Err(format!("Private key not found: {key}"));
            }
            Some(p)
        };
        Ok(ConnectParams {
            host: self.host.trim().to_string(),
            port,
            user: self.user.trim().to_string(),
            password: self.password.clone(),
            private_key,
        })
    }

    pub fn on_connect_failed(&mut self, msg: String) {
        self.connecting = false;
        self.error = Some(msg);
    }

    pub fn on_connected(&mut self) {
        self.connecting = false;
        self.error = None;
    }
}

/// Highlighted mode button.
pub fn mode_button(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let text = if selected {
        RichText::new(label).strong().size(22.0)
    } else {
        RichText::new(label).size(20.0)
    };
    let mut btn = egui::Button::new(text).min_size(egui::vec2(72.0, 40.0));
    if selected {
        btn = btn.fill(egui::Color32::from_rgb(40, 110, 180));
    }
    ui.add(btn)
}
