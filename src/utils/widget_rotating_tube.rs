use eframe::emath::{Rot2, Vec2};
use egui::{RichText, Widget};
use egui::Direction::TopDown;

use crate::app::THEME;
use crate::utils::enums::Direction;

pub struct RotatingTube {
    pub diameter: f32,
    pub direction: Direction,
    pub angle_degrees: f32,
    pub color: egui::Color32,
    pub rpm: u32,
}

impl Default for RotatingTube {
    fn default() -> Self {
        Self {
            diameter: 75.0,
            direction: Direction::Forward,
            angle_degrees: 0.0,
            color: egui::Color32::LIGHT_GRAY,
            rpm: 0,
        }
    }
}

impl RotatingTube {
    pub fn new(diameter: f32, color: egui::Color32) -> Self {
        Self {
            diameter,
            direction: Direction::Forward,
            angle_degrees: 0.0,
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
            let mut stroke_red = visuals.fg_stroke;
            stroke_red.width = stroke_width;
            stroke_red.color = THEME.red;
            ui.painter().circle(center, radius, self.color, stroke);
            // Add a black cross the size of the circle comprising of 4 lines
            // The start and end position should rotate with the orientation
            // One line is red for better visibility
            let rotation = Rot2::from_angle(self.angle_degrees.to_radians());
            let line_1_start_position = center + rotation * Vec2::new(0.0, 0.0);
            let line_1_end_position = center + rotation * Vec2::new(0.0, radius);
            let line_2_start_position = center + rotation * Vec2::new(0.0, 0.0);
            let line_2_end_position = center + rotation * Vec2::new(0.0, -radius - stroke_width);
            let line_3_start_position = center + rotation * Vec2::new(-radius, 0.0);
            let line_3_end_position = center + rotation * Vec2::new(radius, 0.0);
            ui.painter().line_segment([line_1_start_position, line_1_end_position], stroke);
            ui.painter().line_segment([line_2_start_position, line_2_end_position], stroke_red);
            ui.painter().line_segment([line_3_start_position, line_3_end_position], stroke);
            // Write the RPM in the middle in white
            let text = format!("{} RPM", self.rpm);
            let center_rect = egui::Rect::from_center_size(center, Vec2::splat(self.diameter));
            ui.allocate_ui_at_rect(center_rect, |ui| {
                ui.allocate_ui_with_layout(center_rect.size(), egui::Layout::centered_and_justified(TopDown), |ui| {
                    ui.label(RichText::new(text).color(egui::Color32::WHITE).size(font_size).strong());
                });
            });
        }
        response
    }
}