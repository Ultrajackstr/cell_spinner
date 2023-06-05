use egui::{Ui, WidgetText};
use egui_dock::TabViewer;
use crate::utils::motor::Motor;
use crate::utils::structs::Channels;

pub struct Tabs<'a> {
    pub channels: &'a mut Channels,
    pub main_context: egui::Context,
    pub motor: &'a mut Motor,
}

impl Tabs<'_> {
    // todo
}

impl TabViewer for Tabs<'_> {
    type Tab = usize;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        //todo
    }

    fn title(&mut self, _tab: &mut Self::Tab) -> WidgetText {
        let is_connected = self.motor.get_is_connected();
        let is_running = self.motor.get_is_running();
        let motor_name = self.motor.get_name();
        format!("Motor: {}-{}{}",
                motor_name,
                if is_connected { "🔗" } else { "🚫" },
                if is_running { "▶️" } else { "⏹️" },
        ).into()
    }
}