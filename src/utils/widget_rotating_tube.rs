use std::f32::consts::TAU;

use eframe::emath::{Rot2, Vec2};
use egui::Widget;
use strum::Display;

use crate::utils::enums::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Display)]
pub enum Orientation {
    #[strum(to_string = "Top")]
    Top,

    #[strum(to_string = "Bottom")]
    Bottom,

    #[strum(to_string = "Left")]
    Left,

    #[strum(to_string = "Right")]
    Right,

    Custom(f32),
}

impl Orientation {
    pub(crate) fn rot2(&self) -> Rot2 {
        match *self {
            Self::Right => Rot2::from_angle(TAU * 0.00),
            Self::Bottom => Rot2::from_angle(TAU * 0.25),
            Self::Left => Rot2::from_angle(TAU * 0.50),
            Self::Top => Rot2::from_angle(TAU * 0.75),
            Self::Custom(angle) => Rot2::from_angle(angle),
        }
    }
}


pub struct RotatingTube {
    pub diameter: f32,
    pub direction: Direction,
    pub orientation: Orientation,
    pub color: egui::Color32,
    pub rpm: u32,
}

impl RotatingTube {
    pub fn new(diameter: f32, color: egui::Color32) -> Self {
        Self {
            diameter,
            direction: Direction::Forward,
            orientation: Orientation::Top,
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
            ui.painter().circle(center, radius, self.color, visuals.fg_stroke);
        }
        response
    }
}