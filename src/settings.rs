//! Persistent UI settings (remember username / host, etc.).

use std::fs;
use std::path::PathBuf;

use log::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "light" | "white" => Theme::Light,
            _ => Theme::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }

    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub remember_user: bool,
    pub user: String,
    pub remember_host: bool,
    pub host: String,
    pub theme: Theme,
}

impl Settings {
    pub fn path() -> PathBuf {
        settings_path()
    }

    pub fn load() -> Self {
        let path = settings_path();
        let (text, source) = match fs::read_to_string(&path) {
            Ok(t) => (t, path.clone()),
            Err(e) => {
                debug!("no settings at {}: {e}", path.display());
                match legacy_settings_path() {
                    Some(legacy) => match fs::read_to_string(&legacy) {
                        Ok(t) => {
                            info!(
                                "loading legacy settings from {} (will migrate to {})",
                                legacy.display(),
                                path.display()
                            );
                            (t, legacy)
                        }
                        Err(_) => {
                            info!("no settings file; using defaults (theme=dark)");
                            return Self::default();
                        }
                    },
                    None => {
                        info!("no settings file; using defaults (theme=dark)");
                        return Self::default();
                    }
                }
            }
        };
        let mut s = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "remember_user" => s.remember_user = parse_bool(v),
                    "user" => s.user = v.trim().to_string(),
                    "remember_host" => s.remember_host = parse_bool(v),
                    "host" => s.host = v.trim().to_string(),
                    "theme" => s.theme = Theme::parse(v),
                    _ => {}
                }
            }
        }
        info!(
            "settings loaded from {} (theme={}, remember_host={}, remember_user={})",
            source.display(),
            s.theme.as_str(),
            s.remember_host,
            s.remember_user
        );
        // Migrate legacy → new path so theme and other keys persist under ssh_pane.
        if source != path {
            s.save();
        }
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
            "# ssh_pane settings\n\
remember_user={}\n\
user={}\n\
remember_host={}\n\
host={}\n\
theme={}\n",
            bool_str(self.remember_user),
            if self.remember_user {
                self.user.as_str()
            } else {
                ""
            },
            bool_str(self.remember_host),
            if self.remember_host {
                self.host.as_str()
            } else {
                ""
            },
            self.theme.as_str(),
        );
        if let Err(e) = fs::write(&path, body) {
            warn!("cannot write settings {}: {e}", path.display());
        } else {
            debug!(
                "saved settings to {} (theme={})",
                path.display(),
                self.theme.as_str()
            );
        }
    }
}

fn parse_bool(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn bool_str(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn settings_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("ssh_pane")
            .join("settings");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("ssh_pane")
            .join("settings");
    }
    PathBuf::from("ssh_pane_settings")
}

/// Pre-rename path (`ssh_split_pane`); read-only fallback for existing installs.
fn legacy_settings_path() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME") {
        return Some(
            PathBuf::from(home)
                .join(".config")
                .join("ssh_split_pane")
                .join("settings"),
        );
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(
            PathBuf::from(appdata)
                .join("ssh_split_pane")
                .join("settings"),
        );
    }
    Some(PathBuf::from("ssh_split_pane_settings"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_parse_defaults() {
        let s = Settings::default();
        assert!(!s.remember_user);
        assert!(s.user.is_empty());
        assert!(!s.remember_host);
        assert!(s.host.is_empty());
        assert_eq!(s.theme, Theme::Dark);
    }

    #[test]
    fn theme_parse_aliases() {
        assert_eq!(Theme::parse("light"), Theme::Light);
        assert_eq!(Theme::parse("white"), Theme::Light);
        assert_eq!(Theme::parse("dark"), Theme::Dark);
    }
}
