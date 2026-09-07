//! Persistent UI settings (remember username, etc.).

use std::fs;
use std::path::PathBuf;

use log::{debug, warn};

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub remember_user: bool,
    pub user: String,
}

impl Settings {
    pub fn load() -> Self {
        let path = settings_path();
        let Ok(text) = fs::read_to_string(&path) else {
            return Self::default();
        };
        let mut s = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "remember_user" => {
                        s.remember_user = matches!(
                            v.trim().to_ascii_lowercase().as_str(),
                            "1" | "true" | "yes" | "on"
                        );
                    }
                    "user" => s.user = v.trim().to_string(),
                    _ => {}
                }
            }
        }
        debug!("loaded settings from {}", path.display());
        s
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("cannot create settings dir {}: {e}", parent.display());
                return;
            }
        }
        let body = format!(
            "# ssh_split_pane settings\nremember_user={}\nuser={}\n",
            if self.remember_user { "true" } else { "false" },
            if self.remember_user {
                self.user.as_str()
            } else {
                ""
            }
        );
        if let Err(e) = fs::write(&path, body) {
            warn!("cannot write settings {}: {e}", path.display());
        } else {
            debug!("saved settings to {}", path.display());
        }
    }
}

fn settings_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ssh_split_pane")
            .join("settings");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("ssh_split_pane")
            .join("settings");
    }
    PathBuf::from("ssh_split_pane_settings")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_parse_defaults() {
        let s = Settings::default();
        assert!(!s.remember_user);
        assert!(s.user.is_empty());
    }
}
