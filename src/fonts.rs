//! Load a system CJK font so Chinese UI text is not shown as tofu boxes.

use std::path::{Path, PathBuf};

use egui::{FontData, FontDefinitions, FontFamily};
use log::{info, warn};

/// Candidate font files (Linux / WSL / Windows).
fn cjk_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Linux packages
    for p in [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/arphic/uming.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    ] {
        paths.push(PathBuf::from(p));
    }

    // WSL: Windows host fonts
    for p in [
        "/mnt/c/Windows/Fonts/msyh.ttc",
        "/mnt/c/Windows/Fonts/msyhbd.ttc",
        "/mnt/c/Windows/Fonts/simhei.ttf",
        "/mnt/c/Windows/Fonts/simsun.ttc",
    ] {
        paths.push(PathBuf::from(p));
    }

    // Native Windows
    if let Some(windir) = std::env::var_os("WINDIR") {
        let fonts = PathBuf::from(windir).join("Fonts");
        for name in ["msyh.ttc", "msyhbd.ttc", "simhei.ttf", "simsun.ttc"] {
            paths.push(fonts.join(name));
        }
    }

    // macOS
    for p in [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ] {
        paths.push(PathBuf::from(p));
    }

    paths
}

fn read_font(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok().filter(|b| !b.is_empty())
}

/// Install a system CJK font as a fallback for Proportional / Monospace.
pub fn install_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut loaded = false;

    for path in cjk_candidates() {
        if !path.is_file() {
            continue;
        }
        let Some(bytes) = read_font(&path) else {
            continue;
        };
        let name = "cjk_fallback".to_owned();
        fonts
            .font_data
            .insert(name.clone(), FontData::from_owned(bytes));

        // Prefer CJK for missing glyphs: keep default first for Latin metrics,
        // append CJK as fallback so Chinese codepoints resolve.
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Proportional) {
            fam.push(name.clone());
        }
        if let Some(fam) = fonts.families.get_mut(&FontFamily::Monospace) {
            fam.push(name);
        }

        info!("loaded CJK font from {}", path.display());
        loaded = true;
        break;
    }

    if !loaded {
        warn!(
            "no CJK font found; Chinese UI text may show as boxes. \
             Install fonts-noto-cjk or use a system YaHei/SimHei font."
        );
    }

    ctx.set_fonts(fonts);
}
