//! Connection dialog UI.

use egui::{Align, Layout, RichText};

use ssh_split_pane::settings::Settings;
use ssh_split_pane::ssh_worker::{ConnectParams, WorkerHandle, WorkerRequest};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum AuthMethod {
    #[default]
    Password,
    PrivateKey,
}

#[derive(Default)]
pub struct ConnectForm {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub private_key: String,
    auth: AuthMethod,
    pub remember_user: bool,
    pub remember_host: bool,
    pub connecting: bool,
    pub error: Option<String>,
}

impl ConnectForm {
    pub fn new() -> Self {
        let settings = Settings::load();
        Self {
            host: if settings.remember_host && !settings.host.is_empty() {
                settings.host
            } else {
                "127.0.0.1".into()
            },
            port: "22".into(),
            user: if settings.remember_user {
                settings.user
            } else {
                String::new()
            },
            password: String::new(),
            private_key: String::new(),
            auth: AuthMethod::Password,
            remember_user: settings.remember_user,
            remember_host: settings.remember_host,
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
        let mut submit = false;
        const FORM_WIDTH: f32 = 620.0;

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

        // Center the form block horizontally; keep fields left-aligned inside.
        ui.horizontal(|ui| {
            let pad = ((ui.available_width() - FORM_WIDTH) * 0.5).max(0.0);
            ui.add_space(pad);
            ui.allocate_ui_with_layout(
                egui::vec2(FORM_WIDTH, ui.available_height()),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.set_max_width(FORM_WIDTH);
                    egui::Grid::new("connect_grid")
                        .num_columns(2)
                        .spacing([16.0, 12.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Host").size(18.0));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.host)
                                        .desired_width(260.0)
                                        .margin(egui::Margin::symmetric(10.0, 8.0))
                                        .hint_text("hostname or IP"),
                                );
                                ui.add_space(8.0);
                                if ui
                                    .checkbox(
                                        &mut self.remember_host,
                                        RichText::new("Remember Me").size(15.0),
                                    )
                                    .on_hover_text("Remember host IP")
                                    .changed()
                                {
                                    self.persist_settings();
                                }
                            });
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
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.user)
                                        .desired_width(260.0)
                                        .margin(egui::Margin::symmetric(10.0, 8.0)),
                                );
                                ui.add_space(8.0);
                                if ui
                                    .checkbox(
                                        &mut self.remember_user,
                                        RichText::new("Remember Me").size(15.0),
                                    )
                                    .on_hover_text("Remember username")
                                    .changed()
                                {
                                    self.persist_settings();
                                }
                            });
                            ui.end_row();

                            ui.label(RichText::new("Auth").size(18.0));
                            ui.horizontal(|ui| {
                                if mode_button(
                                    ui,
                                    "Password",
                                    self.auth == AuthMethod::Password,
                                )
                                .clicked()
                                {
                                    self.auth = AuthMethod::Password;
                                }
                                if mode_button(
                                    ui,
                                    "Private key",
                                    self.auth == AuthMethod::PrivateKey,
                                )
                                .clicked()
                                {
                                    self.auth = AuthMethod::PrivateKey;
                                }
                            });
                            ui.end_row();

                            match self.auth {
                                AuthMethod::Password => {
                                    ui.label(RichText::new("Password").size(18.0));
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(&mut self.password)
                                            .password(true)
                                            .desired_width(320.0)
                                            .margin(egui::Margin::symmetric(10.0, 8.0)),
                                    );
                                    if resp.lost_focus()
                                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    {
                                        submit = true;
                                    }
                                    ui.end_row();
                                }
                                AuthMethod::PrivateKey => {
                                    ui.label(RichText::new("Private key").size(18.0));
                                    ui.horizontal(|ui| {
                                        let resp = ui.add(
                                            egui::TextEdit::singleline(&mut self.private_key)
                                                .desired_width(240.0)
                                                .margin(egui::Margin::symmetric(10.0, 8.0))
                                                .hint_text("path to key file"),
                                        );
                                        if resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            submit = true;
                                        }
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Browse…").size(18.0),
                                                )
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
                                }
                            }
                        });

                    ui.add_space(20.0);

                    if let Some(err) = &self.error {
                        ui.colored_label(egui::Color32::from_rgb(220, 80, 80), err);
                        ui.add_space(8.0);
                    }

                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        if self.connecting {
                            ui.spinner();
                            ui.label(RichText::new("Connecting…").size(18.0));
                            ui.add_space(12.0);
                        }
                        let can_connect = self.can_connect();
                        if ui
                            .add_enabled(
                                can_connect,
                                egui::Button::new(RichText::new("Connect").size(20.0))
                                    .min_size(egui::vec2(128.0, 44.0)),
                            )
                            .clicked()
                        {
                            submit = true;
                        }
                    });

                    if submit && self.try_connect(worker) {
                        requested = true;
                    }
                },
            );
        });

        requested
    }

    fn can_connect(&self) -> bool {
        !self.connecting && !self.host.trim().is_empty() && !self.user.trim().is_empty()
    }

    /// Validate and send a connect request. Returns true if a request was sent.
    fn try_connect(&mut self, worker: &WorkerHandle) -> bool {
        if !self.can_connect() {
            return false;
        }
        match self.build_params() {
            Ok(params) => {
                self.persist_settings();
                self.connecting = true;
                self.error = None;
                worker.send(WorkerRequest::Connect(params));
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }

    fn persist_settings(&self) {
        Settings {
            remember_user: self.remember_user,
            user: if self.remember_user {
                self.user.trim().to_string()
            } else {
                String::new()
            },
            remember_host: self.remember_host,
            host: if self.remember_host {
                self.host.trim().to_string()
            } else {
                String::new()
            },
        }
        .save();
    }

    fn build_params(&self) -> Result<ConnectParams, String> {
        let port: u16 = self
            .port
            .trim()
            .parse()
            .map_err(|_| "Port must be a number".to_string())?;
        let (password, private_key) = match self.auth {
            AuthMethod::Password => (self.password.clone(), None),
            AuthMethod::PrivateKey => {
                let key = self.private_key.trim();
                if key.is_empty() {
                    return Err("Private key path is required".into());
                }
                let p = std::path::PathBuf::from(key);
                if !p.is_file() {
                    return Err(format!("Private key not found: {key}"));
                }
                (String::new(), Some(p))
            }
        };
        Ok(ConnectParams {
            host: self.host.trim().to_string(),
            port,
            user: self.user.trim().to_string(),
            password,
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
        self.persist_settings();
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
