use eframe::emath::{Rot2, Vec2};
use egui::{FontId, Pos2, RichText, TextStyle, Widget};
use egui::Direction::TopDown;
use crate::app::FONT_BUTTON_SIZE;

use crate::utils::enums::Direction;


pub struct RotatingTube {
    diameter: f32,
    pub direction: Direction,
    orientation: Rot2,
    color: egui::Color32,
    pub rpm: u32,
}

impl RotatingTube {
    pub fn new(diameter: f32, color: egui::Color32) -> Self {
        Self {
            diameter,
            direction: Direction::Forward,
            orientation: Rot2::from_angle(0.0),
            color,
            rpm: 0,
        }
    }
}

// A circle to start
impl Widget for RotatingTube {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = Vec2::splat(self.diameter);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let visuals = ui.style().interact(&response);
        if ui.is_rect_visible(rect) {
            let center = rect.center();
            let radius = rect.width() / 2.0;
            let font_size = self.diameter * 0.18;
            let stroke_width = self.diameter * 0.05;
            let mut stroke = visuals.fg_stroke;
            stroke.width = stroke_width;
            ui.painter().circle(center, radius, self.color, stroke);
            // let line_1_start_position = Pos2::new(
            // let line_1_end_position = Pos2::new(center.x + radius, center.y);
            // let line_2_start_position = Pos2::new(center.x, center.y - radius);
            // let line_2_end_position = Pos2::new(center.x, center.y + radius);
            // Add a black cross the size of the circle
            // The start and end position should rotate with the orientation
            let line_1_start_position = center + self.orientation * Vec2::new(-radius, 0.0); //TODO: check if this is correct and try to synchronize it with stepgen
            let line_1_end_position = center + self.orientation * Vec2::new(radius, 0.0);
            let line_2_start_position = center + self.orientation * Vec2::new(0.0, -radius);
            let line_2_end_position = center + self.orientation * Vec2::new(0.0, radius);
            ui.painter().line_segment([line_1_start_position, line_1_end_position], (stroke_width, visuals.fg_stroke.color));
            ui.painter().line_segment([line_2_start_position, line_2_end_position], (stroke_width, visuals.fg_stroke.color));
            // Write the RPM in the middle in white
            let text = format!("{} RPM", self.rpm);
            let center_rect = egui::Rect::from_center_size(center, Vec2::splat(self.diameter));
            ui.allocate_ui_at_rect(center_rect, |ui| {
                // ui.painter().text(center, egui::Align2::CENTER_CENTER, text, TextStyle::Body.resolve(ui.style()), egui::Color32::WHITE);
                ui.allocate_ui_with_layout(center_rect.size(),egui::Layout::centered_and_justified(TopDown), |ui| {
                    ui.label(RichText::new(text).color(egui::Color32::WHITE).size(font_size).strong());
                });
                // ui.label(RichText::new(text).color(egui::Color32::WHITE).size(FONT_BUTTON_SIZE.font_default));
            });
        }
        response
    }
}