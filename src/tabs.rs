use std::sync::Arc;

use dashmap::DashMap;
use egui::{Ui, WidgetText};
use egui_dock::{NodeIndex, TabViewer};

use crate::utils::motor::Motor;
use crate::utils::structs::Channels;

pub struct Tabs<'a> {
    pub channels: &'a mut Channels,
    pub main_context: egui::Context,
    pub motor: &'a mut Arc<DashMap<usize, Motor>>,
    pub added_nodes: &'a mut Vec<NodeIndex>,
    pub added_tabs: &'a mut Vec<usize>,
    pub current_tab_counter: &'a mut usize,
    pub can_tab_close: &'a mut bool
}

impl Tabs<'_> {
    fn init_tab(&mut self, tab: usize) {
        self.motor.insert(tab, Motor::default());
        self.added_tabs.push(tab);
    }
}

impl TabViewer for Tabs<'_> {
    type Tab = usize;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        //todo
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        let is_connected = self.motor.get(tab).unwrap().get_is_connected();
        let is_running = self.motor.get(tab).unwrap().get_is_running();
        let motor_name = self.motor.get(tab).unwrap().get_name();
        format!("Motor: {}-{}{}",
                motor_name,
                if is_connected { "🔗" } else { "🚫" },
                if is_running { "▶️" } else { "⏹️" },
        ).into()
    }

    fn on_add(&mut self, node: NodeIndex) {
        self.added_nodes.push(node);
        *self.current_tab_counter += 1;
        self.init_tab(*self.current_tab_counter);
    }
}