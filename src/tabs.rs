use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use egui::{Ui, WidgetText};
use egui_dock::{NodeIndex, TabViewer};
use egui_toast::ToastKind;

use crate::utils::motor::Motor;
use crate::utils::structs::{Channels, Message};

pub struct Tabs<'a> {
    pub channels: &'a mut Channels,
    pub main_context: egui::Context,
    pub available_ports: &'a mut Vec<String>,
    pub already_connected_ports: &'a mut Arc<Mutex<Vec<String>>>,
    pub motor: &'a mut Arc<DashMap<usize, Motor>>,
    pub added_nodes: &'a mut Vec<NodeIndex>,
    pub added_tabs: &'a mut Vec<usize>,
    pub current_tab_counter: &'a mut usize,
    pub can_tab_close: &'a mut bool,
}

impl Tabs<'_> {
    fn init_tab(&mut self, tab: usize) {
        self.motor.insert(tab, Motor::default());
        self.added_tabs.push(tab);
        let available_ports = match serialport::available_ports() {
            Ok(ports) => {
                let available_ports: Vec<String> = ports.iter().map(|port| port.port_name.clone())
                    .filter(|port| !self.already_connected_ports.lock().unwrap().contains(port)).collect();
                available_ports
            }
            Err(err) => {
                let error = anyhow::Error::new(err);
                self.channels.message_tx.as_ref().unwrap().send(Message::new(ToastKind::Error, "Error while listing serial ports", Some(error), Some(format!("Tab {}", tab)), 3, false)).ok();
                vec![]
            }
        };
        *self.available_ports = available_ports;
    }

    fn remove_tab(&mut self, tab: usize) {
        self.already_connected_ports.lock().unwrap().retain(|x| *x != self.motor.get(&tab).unwrap().get_serial().get_port_name());
        self.motor.remove(&tab);
        self.added_tabs.retain(|x| *x != tab);
    }
}

impl TabViewer for Tabs<'_> {
    type Tab = usize;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        ui.label(format!("Hello, World from tab {}!", tab));
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        if self.motor.get(tab).is_none() { // Avoid panic while the tab is removed.
            return "Motor".into();
        }
        let is_connected = self.motor.get(tab).unwrap().get_is_connected();
        let is_running = self.motor.get(tab).unwrap().get_is_running();
        let motor_name = self.motor.get(tab).unwrap().get_name().to_string();
        format!("Motor: {}-{}{}",
                motor_name,
                if is_connected { "🔗" } else { "🚫" },
                if is_running { "▶️" } else { "⏹️" },
        ).into()
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        let is_running = self.motor.get(tab).unwrap().get_is_running();
        if is_running {
            let message: Message = Message::new(ToastKind::Warning,
                                                "Motor is running! Please stop the motor before closing the tab."
                                                , None, Some(format!("Motor: {}", self.motor.get(tab).unwrap().get_name()))
                                                , 3, false);
            self.channels.message_tx.as_ref().unwrap().send(message).ok();
            return false;
        }
        *self.current_tab_counter -= 1;
        // Remove from the added tabs.
        self.added_tabs.retain(|x| *x != *tab);
        self.remove_tab(*tab);
        *self.can_tab_close = true;
        true
    }

    fn on_add(&mut self, node: NodeIndex) {
        self.added_nodes.push(node);
        *self.current_tab_counter += 1;
        self.init_tab(*self.current_tab_counter);
    }
}
