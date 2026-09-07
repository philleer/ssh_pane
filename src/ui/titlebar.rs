//! Custom title bar (used with `ViewportBuilder::with_decorations(false)`).
//!
//! Native decorations (especially Wayland CSD / DWM shadows) can leave a
//! pre-maximize outline when the window is maximized. Drawing our own chrome
//! avoids that compositor shadow rim.

use egui::{Sense, Vec2, ViewportCommand};

pub struct TitleBarAction {
    pub drag: bool,
    pub double_clicked: bool,
}

/// Draw a draggable title bar with minimize / maximize / close.
pub fn show(ui: &mut egui::Ui, title: &str, maximized: bool) -> TitleBarAction {
    let mut action = TitleBarAction {
        drag: false,
        double_clicked: false,
    };

    let height = 36.0_f32;
    let (rect, resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        Sense::click_and_drag(),
    );

    // Background strip
    ui.painter()
        .rect_filled(rect, 0.0, ui.visuals().widgets.noninteractive.bg_fill);

    // Title text (left)
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(16.0),
        ui.visuals().widgets.noninteractive.fg_stroke.color,
    );

    // Window controls on the right
    let btn_w = 46.0_f32;
    let btn_h = height;
    let mut x = rect.right();

    // Close
    x -= btn_w;
    let close_rect = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(btn_w, btn_h));
    let close = ui.allocate_rect(close_rect, Sense::click());
    let close_bg = if close.hovered() {
        egui::Color32::from_rgb(200, 60, 60)
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(close_rect, 0.0, close_bg);
    ui.painter().text(
        close_rect.center(),
        egui::Align2::CENTER_CENTER,
        "X",
        egui::FontId::proportional(14.0),
        if close.hovered() {
            egui::Color32::WHITE
        } else {
            ui.visuals().text_color()
        },
    );
    if close.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }

    // Maximize / restore
    x -= btn_w;
    let max_rect = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(btn_w, btn_h));
    let max = ui.allocate_rect(max_rect, Sense::click());
    if max.hovered() {
        ui.painter()
            .rect_filled(max_rect, 0.0, ui.visuals().widgets.hovered.bg_fill);
    }
    ui.painter().text(
        max_rect.center(),
        egui::Align2::CENTER_CENTER,
        if maximized { "❐" } else { "□" },
        egui::FontId::proportional(16.0),
        ui.visuals().text_color(),
    );
    if max.clicked() {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }

    // Minimize
    x -= btn_w;
    let min_rect = egui::Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(btn_w, btn_h));
    let min = ui.allocate_rect(min_rect, Sense::click());
    if min.hovered() {
        ui.painter()
            .rect_filled(min_rect, 0.0, ui.visuals().widgets.hovered.bg_fill);
    }
    ui.painter().text(
        min_rect.center(),
        egui::Align2::CENTER_CENTER,
        "–",
        egui::FontId::proportional(18.0),
        ui.visuals().text_color(),
    );
    if min.clicked() {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }

    // Drag / double-click maximize on the remaining title area
    let drag_area = egui::Rect::from_min_max(rect.min, egui::pos2(x, rect.bottom()));
    let drag_resp = ui.interact(drag_area, ui.id().with("title_drag"), Sense::click_and_drag());
    if drag_resp.drag_started() {
        action.drag = true;
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag_resp.double_clicked() {
        action.double_clicked = true;
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }

    // Keep compiler happy if Sense on outer resp unused
    let _ = resp;

    action
}
