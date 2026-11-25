use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui) {
    let grip_rect = egui::Rect::from_min_size(
        ui.max_rect().right_bottom() - egui::vec2(20.0, 20.0),
        egui::vec2(20.0, 20.0),
    );
    let grip = ui.interact(
        grip_rect,
        ui.id().with("window-resize-grip"),
        egui::Sense::drag(),
    );
    if grip.drag_started() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::BeginResize(
                egui::ResizeDirection::SouthEast,
            ));
    }
    let color = if grip.hovered() {
        egui::Color32::from_rgb(194, 157, 101)
    } else {
        egui::Color32::from_rgb(87, 77, 96)
    };
    for offset in [5.0, 9.0, 13.0] {
        ui.painter().line_segment(
            [
                grip_rect.right_bottom() - egui::vec2(offset, 1.0),
                grip_rect.right_bottom() - egui::vec2(1.0, offset),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
}
