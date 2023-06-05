use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use dashmap::DashMap;
use egui::{Color32, RichText, Ui, WidgetText};
use egui_dock::{NodeIndex, TabViewer};
use egui_toast::ToastKind;

use crate::app::{FONT_BUTTON_SIZE, THEME};
use crate::utils::motor::Motor;
use crate::utils::structs::{Channels, Message};

pub struct Tabs<'a> {
    pub channels: &'a mut Channels,
    pub main_context: egui::Context,
    pub available_ports: &'a mut Vec<String>,
    pub already_connected_ports: &'a mut Arc<Mutex<Vec<String>>>,
    pub selected_port: &'a mut DashMap<usize, String>,
    pub motor_name: &'a mut DashMap<usize, String>,
    pub motor: &'a mut Arc<DashMap<usize, Motor>>,
    pub promise_serial_connect: &'a mut Arc<DashMap<usize, Option<()>>>,
    pub added_nodes: &'a mut Vec<NodeIndex>,
    pub added_tabs: &'a mut Vec<usize>,
    pub current_tab_counter: &'a mut usize,
    pub absolute_tab_counter: &'a mut usize,
    pub can_tab_close: &'a mut bool,
}

impl Tabs<'_> {
    fn init_tab(&mut self, tab: usize) {
        self.promise_serial_connect.insert(tab, None);
        self.motor.insert(tab, Motor::default());
        self.motor_name.insert(tab, tab.to_string());
        self.added_tabs.push(tab);
        self.refresh_available_serial_ports(tab);
        self.selected_port.insert(tab, self.available_ports[0].clone());
    }

    fn remove_tab(&mut self, tab: usize) {
        self.promise_serial_connect.remove(&tab);
        self.already_connected_ports.lock().unwrap().retain(|x| *x != self.motor.get(&tab).unwrap().get_serial().get_port_name());
        self.motor_name.remove(&tab);
        self.motor.remove(&tab);
        self.added_tabs.retain(|x| *x != tab);
    }

    fn thread_spawn_new_motor(&mut self, tab: usize, serial_port: String) {
        self.promise_serial_connect.insert(tab, Some(()));
        let promise = self.promise_serial_connect.clone();
        let motors = self.motor.clone();
        let channels = self.channels.message_tx.clone();
        let already_connected_ports = self.already_connected_ports.clone();
        thread::spawn(move || {
            let motor = match Motor::new(serial_port, already_connected_ports) {
                Ok(motor) => motor,
                Err(err) => {
                    channels.as_ref().unwrap().send(Message::new(ToastKind::Error, "Error while connecting to serial port", Some(err), Some(format!("Tab {}", tab)), 3, false)).ok();
                    promise.insert(tab, None);
                    return;
                }
            };
            motors.insert(tab, motor);
            promise.insert(tab, None);
            channels.as_ref().unwrap().send(Message::new(ToastKind::Success, "Successfully connected to serial port", None, Some(format!("Tab {}", tab)), 3, false)).ok();
        });
    }

    fn refresh_available_serial_ports(&mut self, tab: usize) {
        let available_ports = match serialport::available_ports() {
            Ok(ports) => {
                let available_ports: Vec<String> = ports.iter().map(|port| port.port_name.clone())
                    .filter(|port| !self.already_connected_ports.lock().unwrap().contains(port)).collect();
                available_ports
            }
            Err(err) => {
                let error = anyhow::Error::new(err);
                self.channels.message_tx.as_ref().unwrap().send(Message::new(ToastKind::Error, "Error while listing serial ports", Some(error), Some(format!("Tab {}", tab)), 3, false)).ok();
                vec!["".to_string()]
            }
        };
        *self.available_ports = available_ports;
        self.selected_port.insert(tab, self.available_ports[0].clone());
    }

    fn disconnect(&mut self, tab: &mut usize) {
        self.already_connected_ports.lock().unwrap().retain(|x| *x != self.motor.get(tab).unwrap().get_serial().get_port_name());
        self.motor.get_mut(tab).unwrap().disconnect();
        self.selected_port.get_mut(tab).unwrap().clear();
        self.refresh_available_serial_ports(*tab);
    }
}

impl TabViewer for Tabs<'_> {
    type Tab = usize;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        if *self.can_tab_close {
            *self.can_tab_close = false;
            return;
        }
        let is_connected = self.motor.get(tab).unwrap().get_is_connected();
        let is_running = self.motor.get(tab).unwrap().get_is_running();
        egui::ScrollArea::horizontal().id_source("connect").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        // Refresh COM ports button.
                        ui.add_enabled_ui(!is_connected && self.promise_serial_connect.get(tab).unwrap().is_none(), |ui| {
                            if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new("Refresh ➡")).clicked() {
                                self.refresh_available_serial_ports(*tab);
                            }
                            let selected_port = self.selected_port.get(tab).unwrap().value().clone();
                            egui::ComboBox::from_id_source("available_ports")
                                .selected_text(selected_port)
                                .show_ui(ui, |ui| {
                                    for port in self.available_ports.iter() {
                                        ui.selectable_value(self.selected_port.get_mut(tab).unwrap().value_mut(), port.to_string(), port.to_string());
                                    }
                                });
                        });
                    });
                    ui.horizontal(|ui| {
                        // Disconnect button.
                        ui.add_enabled_ui(is_connected, |ui| {
                            if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("DISCONNECT").color(Color32::WHITE)).fill(THEME.red)).clicked() {
                                self.disconnect(tab);
                            }
                        });
                        // Connect button.
                        ui.add_enabled_ui(!is_connected && self.promise_serial_connect.get(tab).unwrap().is_none() &&
                                              !self.available_ports.is_empty(), |ui| {
                            if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("Connect").color(Color32::WHITE)).fill(THEME.green)).clicked() {
                                self.channels.message_tx.as_ref().unwrap().send(Message::new(ToastKind::Info, "Connecting to serial port...", None, Some(format!("Tab {}", tab)), 0, true)).ok();
                                let selected_port = self.selected_port.get(tab).unwrap().value().to_string();
                                self.thread_spawn_new_motor(*tab, selected_port);
                            };
                        });
                    });
                });

                // Show the run time of the motor
                let run_time = self.motor.get(tab).unwrap().get_run_time_ms().as_secs_f32();
                ui.label(format!("⏱️: {:.2} s", run_time))
                    .on_hover_text(
                        // Show, minutes, hours, days
                        format!(
                            "Total run time of the motor.\n{:.2} minutes\n{:.2} hours\n{:.2} days",
                            run_time / 60.0,
                            run_time / 3600.0,
                            run_time / 86400.0
                        )
                    );
            });
            ui.separator();
        });
        ui.add_enabled_ui(is_connected, |ui| {
            ui.label("Motor name:");
            if ui.add_sized(egui::vec2(50.0, 35.0), egui::TextEdit::singleline(self.motor_name.get_mut(tab).unwrap().value_mut())).lost_focus() {
                self.motor.get_mut(tab).unwrap().set_name(&self.motor_name.get(tab).unwrap());
            }
        });
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
        *self.absolute_tab_counter += 1;
        self.init_tab(*self.absolute_tab_counter);
    }
}
