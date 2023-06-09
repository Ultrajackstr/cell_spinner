use std::sync::{Arc, Mutex};
use std::thread;

use dashmap::DashMap;
use egui::{Color32, RichText, Ui, WidgetText};
use egui::plot::{Corner, Legend, Line};
use egui_dock::{NodeIndex, TabViewer};
use egui_toast::ToastKind;

use crate::app::{FONT_BUTTON_SIZE, MAX_ACCELERATION, MAX_DURATION_MS, MAX_POINTS_GRAPHS, THEME};
use crate::utils::enums::Direction;
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
        self.motor.get_mut(&tab).unwrap().set_name(&format!("Motor {}", tab));
        self.motor_name.insert(tab, format!("Motor {}", tab));
        self.added_tabs.push(tab);
        self.refresh_available_serial_ports(tab);
        self.selected_port.insert(tab, self.available_ports[0].clone());
    }

    fn remove_tab(&mut self, tab: usize) {
        self.already_connected_ports.lock().unwrap().retain(|x| *x != self.motor.get(&tab).unwrap().get_serial().get_port_name());
        self.motor.get_mut(&tab).unwrap().disconnect();
        self.selected_port.get_mut(&tab).unwrap().clear();
        self.promise_serial_connect.remove(&tab);
        self.motor_name.remove(&tab);
        self.motor.remove(&tab);
        self.added_tabs.retain(|x| x != &tab);
    }

    fn thread_spawn_new_motor(&mut self, tab: usize, serial_port: String, motor_name: String) {
        self.promise_serial_connect.insert(tab, Some(()));
        let promise = self.promise_serial_connect.clone();
        let motors = self.motor.clone();
        let channels = self.channels.message_tx.clone();
        let already_connected_ports = self.already_connected_ports.clone();
        let current_protocol = *self.motor.get(&tab).unwrap().get_protocol();
        let current_graph = self.motor.get(&tab).unwrap().get_graph().clone();
        thread::spawn(move || {
            let motor = match Motor::new_with_protocol_and_graph(serial_port, motor_name, already_connected_ports, current_protocol, current_graph) {
                Ok(motor) => motor,
                Err(err) => {
                    channels.as_ref().unwrap().send(Message::new(ToastKind::Error, "Error while connecting to serial port", Some(err), Some(format!("Motor {}", tab)), 3, false)).ok();
                    promise.insert(tab, None);
                    return;
                }
            };
            motors.insert(tab, motor);
            promise.insert(tab, None);
            channels.as_ref().unwrap().send(Message::new(ToastKind::Success, "Successfully connected to serial port", None, Some(format!("Motor {}", tab)), 3, false)).ok();
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
                self.channels.message_tx.as_ref().unwrap().send(Message::new(ToastKind::Error, "Error while listing serial ports", Some(error), Some(format!("Motor {}", tab)), 3, false)).ok();
                vec!["".to_string()]
            }
        };
        *self.available_ports = available_ports;
        self.selected_port.insert(tab, self.available_ports[0].clone());
    }

    fn disconnect(&mut self, tab: usize) {
        self.already_connected_ports.lock().unwrap().retain(|x| *x != self.motor.get(&tab).unwrap().get_serial().get_port_name());
        self.motor.get_mut(&tab).unwrap().disconnect();
        self.selected_port.get_mut(&tab).unwrap().clear();
        self.refresh_available_serial_ports(tab);
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
                egui::Grid::new("serial")
                    .show(ui, |ui| {
                        // Refresh COM ports button.
                        ui.add_enabled_ui(!is_connected && self.promise_serial_connect.get(tab).unwrap().is_none(), |ui| {
                            if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new("Refresh ➡")).clicked() {
                                self.refresh_available_serial_ports(*tab);
                            }
                        });
                        ui.add_enabled_ui(!is_connected && self.promise_serial_connect.get(tab).unwrap().is_none(), |ui| {
                            let selected_port = self.selected_port.get(tab).unwrap().value().clone();
                            egui::ComboBox::from_id_source("available_ports")
                                .selected_text(selected_port)
                                .show_ui(ui, |ui| {
                                    for port in self.available_ports.iter() {
                                        ui.selectable_value(self.selected_port.get_mut(tab).unwrap().value_mut(), port.to_string(), port.to_string());
                                    }
                                });
                        });
                        ui.add_enabled_ui(is_connected && !is_running, |ui| {
                            if ui.add_sized(egui::vec2(100.0, 20.0), egui::TextEdit::singleline(self.motor_name.get_mut(tab).unwrap().value_mut()))
                                .on_hover_text("Change the name of the motor")
                                .lost_focus() {
                                self.motor.get_mut(tab).unwrap().set_name(&self.motor_name.get(tab).unwrap());
                            }
                        });
                        ui.end_row();
                        // Disconnect button.
                        ui.add_enabled_ui(is_connected, |ui| {
                            if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("DISCONNECT").color(Color32::WHITE)).fill(THEME.red)).clicked() {
                                self.disconnect(*tab);
                            }
                        });
                        // Connect button.
                        ui.add_enabled_ui(!is_connected && self.promise_serial_connect.get(tab).unwrap().is_none() &&
                                              !self.available_ports.is_empty(), |ui| {
                            if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("Connect").color(Color32::WHITE)).fill(THEME.green)).clicked() {
                                self.channels.message_tx.as_ref().unwrap().send(Message::new(ToastKind::Info, "Connecting to serial port...", None, Some(format!("Motor {}", tab)), 0, true)).ok();
                                let selected_port = self.selected_port.get(tab).unwrap().value().to_string();
                                let motor_name = self.motor_name.get(tab).unwrap().clone();
                                self.thread_spawn_new_motor(*tab, selected_port, motor_name);
                            };
                        });
                        // Show the run time of the motor
                        let run_time = self.motor.get(tab).unwrap().get_run_time_ms().as_secs_f32();
                        ui.label(format!("⏱️: {:.2} s", run_time))
                            .on_hover_text(
                                format!("{} days, {} hours, {} minutes", run_time as u32 / 86400, run_time as u32 / 3600 % 24, run_time as u32 / 60 % 60)
                            );
                    });
                ////////////////////////////
                ui.separator();
                ui.horizontal_centered(|ui| {
                    // Button to send the parameters to the motor and run it. Focus is check to prevent the button from being pressed when the user is typing in the text field.
                    ui.add_enabled_ui(is_connected && !is_running && self.main_context.memory(|mem| mem.focus().is_none() && self.motor.get(tab).unwrap().get_protocol().global_duration_ms != 0), |ui| {
                        let run_response = ui.add_sized(egui::vec2(FONT_BUTTON_SIZE.button_default.x, FONT_BUTTON_SIZE.button_default.y * 2.0), egui::Button::new(RichText::new("Run")
                            .color(Color32::WHITE)).fill(THEME.green))
                            .on_hover_text("Right click to start all motors");
                        if run_response.clicked() {
                            self.motor.get_mut(tab).unwrap().start_motor(self.channels.message_tx.clone());
                        } else if run_response.secondary_clicked() {
                            // Start all the connected motors that are not running
                            self.motor.iter_mut().for_each(|mut motor| {
                                if motor.get_is_connected() && !motor.get_is_running() {
                                    motor.start_motor(self.channels.message_tx.clone());
                                }
                            });
                        }
                    });
                    ui.add_enabled_ui(is_connected && is_running, |ui| {
                        let stop_response = ui.add_sized(egui::vec2(FONT_BUTTON_SIZE.button_default.x, FONT_BUTTON_SIZE.button_default.y * 2.0), egui::Button::new(RichText::new("STOP MOTOR").color(Color32::WHITE)).fill(THEME.red))
                            .on_hover_text("Right click to stop all motors");
                        if stop_response.clicked() {
                            self.motor.get_mut(tab).unwrap().stop_motor();
                        } else if stop_response.secondary_clicked() {
                            // Stop all running motors
                            self.motor.iter_mut().for_each(|mut motor| {
                                if motor.get_is_running() {
                                    motor.stop_motor();
                                }
                            });
                        }
                    });
                });
                ui.separator();
                // Emergency stop button.
                if ui.add_sized(egui::vec2(FONT_BUTTON_SIZE.button_default.x, FONT_BUTTON_SIZE.button_default.y * 2.0), egui::Button::new(RichText::new("EMERGENCY\nSTOP").color(Color32::WHITE))
                    .fill(THEME.peach))
                    .on_hover_text("Stop all the motors and disconnect them.")
                    .clicked() {
                    self.motor.iter_mut().for_each(|mut motor| {
                        motor.stop_motor();
                        motor.disconnect();
                    });
                }
            });
        });
        ui.separator();
        ////// SETUP //////
        ui.add_enabled_ui(!is_running, |ui| {
            egui::ScrollArea::horizontal().id_source("connect").show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Setup rotation phase
                    let mut rotation_graph_needs_update = false;
                    ui.allocate_ui(egui::vec2(370.0, 280.0), |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Rotation ⬇️").color(THEME.sapphire).size(FONT_BUTTON_SIZE.font_large));
                            ui.separator();
                            egui::Grid::new("rotation_grid")
                                .show(ui, |ui| {
                                    // Slider for RPM
                                    ui.label("RPM:");
                                    let max_rpm = self.motor.get(tab).unwrap().get_protocol().rotation.max_rpm_for_stepmode();
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.rpm, 1..=max_rpm)).changed() {
                                        rotation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // Slider for acceleration
                                    ui.label("Acceleration:");
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.acceleration, 1..=MAX_ACCELERATION)).changed() {
                                        rotation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // List for stepmode
                                    let modes = self.motor.get(tab).unwrap().get_protocol().rotation.step_mode.get_modes();
                                    let selected_mode = self.motor.get(tab).unwrap().get_protocol().rotation.step_mode;
                                    ui.label("Step mode:");
                                    egui::ComboBox::from_id_source("step_mode_rotation")
                                        .selected_text(selected_mode.to_string())
                                        .show_ui(ui, |ui| {
                                            for mode in modes {
                                                if ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.step_mode, mode, mode.to_string()).changed() {
                                                    rotation_graph_needs_update = true;
                                                }
                                            }
                                        });
                                    ui.end_row();
                                    // Duration for 1 direction cycle
                                    ui.label("Cycle duration (ms):").on_hover_text("Duration of a cycle of rotations in one direction.");
                                    let current_duration = self.motor.get(tab).unwrap().get_protocol().rotation.duration_of_one_direction_cycle_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.duration_of_one_direction_cycle_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Rotation cycle duration")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_duration / 86400000, (current_duration % 86400000) / 3600000, (current_duration % 3600000) / 60000, (current_duration % 60000) / 1000));
                                            });
                                    }
                                    if response.changed() {
                                        rotation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // Direction
                                    let directions: [Direction; 2] = [Direction::Forward, Direction::Backward];
                                    let selected_direction = self.motor.get(tab).unwrap().get_protocol().rotation.direction;
                                    ui.label("Direction:");
                                    egui::ComboBox::from_id_source("direction_rotation")
                                        .selected_text(selected_direction.to_string())
                                        .show_ui(ui, |ui| {
                                            for direction in directions {
                                                ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.direction, direction, direction.to_string());
                                            }
                                        });
                                    ui.end_row();
                                    // Pause before direction change
                                    ui.label("Pause (ms):").on_hover_text("Pause before changing the direction of rotation.");
                                    let current_pause = self.motor.get(tab).unwrap().get_protocol().rotation.pause_before_direction_change_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.pause_before_direction_change_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Pause before rotation change")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_pause / 86400000, (current_pause % 86400000) / 3600000, (current_pause % 3600000) / 60000, (current_pause % 60000) / 1000));
                                            });
                                    }
                                    ui.end_row();
                                    // Slider for rotation duration
                                    ui.label("Rotation duration (ms):").on_hover_text("Duration of the rotation phase.");
                                    let current_duration = self.motor.get(tab).unwrap().get_protocol().rotation_duration_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation_duration_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Rotation duration")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_duration / 86400000, (current_duration % 86400000) / 3600000, (current_duration % 3600000) / 60000, (current_duration % 60000) / 1000));
                                            });
                                    }
                                    ui.end_row();
                                    // Slider for pause before agitation
                                    ui.label("Pause pre-agitation (ms):").on_hover_text("Pause before the agitation phase.");
                                    let current_pause = self.motor.get(tab).unwrap().get_protocol().pause_before_agitation_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().pause_before_agitation_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Pause pre-agitation")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_pause / 86400000, (current_pause % 86400000) / 3600000, (current_pause % 3600000) / 60000, (current_pause % 60000) / 1000));
                                            });
                                    }
                                });
                            if rotation_graph_needs_update {
                                let max_rpm_rotation = self.motor.get(tab).unwrap().get_protocol().rotation.max_rpm_for_stepmode();
                                let current_rpm_rotation = self.motor.get(tab).unwrap().get_protocol().rotation.rpm;
                                if current_rpm_rotation > max_rpm_rotation {
                                    self.motor.get_mut(tab).unwrap().get_protocol_mut().rotation.rpm = max_rpm_rotation;
                                }
                                self.motor.get(tab).unwrap().generate_graph_rotation();
                                rotation_graph_needs_update = false;
                            }
                        });
                    });
                    ui.separator();
                    // Setup agitation phase
                    let mut agitation_graph_needs_update = false;
                    ui.allocate_ui(egui::vec2(370.0, 280.0), |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Agitation ⬇️").color(THEME.blue).size(FONT_BUTTON_SIZE.font_large));
                            ui.separator();
                            egui::Grid::new("agitation_grid")
                                .show(ui, |ui| {
                                    // Slider for RPM
                                    ui.label("RPM:");
                                    let max_rpm = self.motor.get(tab).unwrap().get_protocol().agitation.max_rpm_for_stepmode();
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.rpm, 1..=max_rpm)).changed() {
                                        agitation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // Slider for acceleration
                                    ui.label("Acceleration:");
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.acceleration, 1..=MAX_ACCELERATION)).changed() {
                                        agitation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // List for stepmode
                                    let modes = self.motor.get(tab).unwrap().get_protocol().agitation.step_mode.get_modes();
                                    let selected_mode = self.motor.get(tab).unwrap().get_protocol().agitation.step_mode;
                                    ui.label("Step mode:");
                                    egui::ComboBox::from_id_source("step_mode_agitation")
                                        .selected_text(selected_mode.to_string())
                                        .show_ui(ui, |ui| {
                                            for mode in modes {
                                                if ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.step_mode, mode, mode.to_string()).changed() {
                                                    agitation_graph_needs_update = true;
                                                }
                                            }
                                        });
                                    ui.end_row();
                                    // Duration for 1 direction cycle
                                    ui.label("Cycle duration (ms):").on_hover_text("Duration of a cycle of agitations in one direction.");
                                    let current_duration = self.motor.get(tab).unwrap().get_protocol().agitation.duration_of_one_direction_cycle_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.duration_of_one_direction_cycle_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Agitation cycle duration")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_duration / 86400000, (current_duration % 86400000) / 3600000, (current_duration % 3600000) / 60000, (current_duration % 60000) / 1000));
                                            });
                                    }
                                    if response.changed() {
                                        agitation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // Direction
                                    let directions: [Direction; 2] = [Direction::Forward, Direction::Backward];
                                    let selected_direction = self.motor.get(tab).unwrap().get_protocol().agitation.direction;
                                    ui.label("Direction:");
                                    egui::ComboBox::from_id_source("direction_agitation")
                                        .selected_text(selected_direction.to_string())
                                        .show_ui(ui, |ui| {
                                            for direction in directions {
                                                ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.direction, direction, direction.to_string());
                                            }
                                        });
                                    ui.end_row();
                                    // Pause before direction change

                                    ui.label("Pause (ms):").on_hover_text("Pause before changing the direction of agitation.");
                                    let current_pause = self.motor.get(tab).unwrap().get_protocol().agitation.pause_before_direction_change_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.pause_before_direction_change_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Pause before agitation change")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_pause / 86400000, (current_pause % 86400000) / 3600000, (current_pause % 3600000) / 60000, (current_pause % 60000) / 1000));
                                            });
                                    }
                                    ui.end_row();
                                    // Slider for agitation duration

                                    ui.label("Agitation duration (ms):").on_hover_text("Duration of the agitation phase.");
                                    let current_duration = self.motor.get(tab).unwrap().get_protocol().agitation_duration_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation_duration_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Agitation duration")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_duration / 86400000, (current_duration % 86400000) / 3600000, (current_duration % 3600000) / 60000, (current_duration % 60000) / 1000));
                                            });
                                    }
                                    ui.end_row();
                                    // Slider for pause after agitation
                                    ui.label("Pause post-agitation (ms):").on_hover_text("Pause after the agitation phase.");
                                    let current_pause = self.motor.get(tab).unwrap().get_protocol().pause_after_agitation_ms;
                                    let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().pause_after_agitation_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                    if response.hovered() || response.has_focus() || response.dragged() {
                                        egui::Window::new("Pause post-agitation")
                                            .collapsible(false)
                                            .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                            .show(&self.main_context, |ui| {
                                                ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_pause / 86400000, (current_pause % 86400000) / 3600000, (current_pause % 3600000) / 60000, (current_pause % 60000) / 1000));
                                            });
                                    }
                                });
                        });
                        if agitation_graph_needs_update {
                            let max_rpm_agitation = self.motor.get(tab).unwrap().get_protocol().agitation.max_rpm_for_stepmode();
                            let current_rpm_agitation = self.motor.get(tab).unwrap().get_protocol().agitation.rpm;
                            if current_rpm_agitation > max_rpm_agitation {
                                self.motor.get_mut(tab).unwrap().get_protocol_mut().agitation.rpm = max_rpm_agitation;
                            }
                            self.motor.get(tab).unwrap().generate_graph_agitation();
                            agitation_graph_needs_update = false;
                        }
                    });
                    ui.separator();
                    // Setup durations
                    ui.allocate_ui(egui::vec2(370.0, 280.0), |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Global Duration ⬇️").color(THEME.lavender).size(FONT_BUTTON_SIZE.font_large));
                            ui.separator();
                            // Global duration of the protocol
                            ui.horizontal(|ui| {
                                let color = if self.motor.get(tab).unwrap().get_protocol().global_duration_ms == 0 { THEME.red } else { THEME.text };
                                ui.label(RichText::new("Global duration (ms):").color(color)).on_hover_text("Global duration of the protocol.");
                                let current_duration = self.motor.get(tab).unwrap().get_protocol().global_duration_ms;
                                let response = ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().get_protocol_mut().global_duration_ms, 0..=MAX_DURATION_MS).logarithmic(true));
                                if response.hovered() || response.has_focus() || response.dragged() {
                                    egui::Window::new("Global duration")
                                        .collapsible(false)
                                        .default_pos(response.rect.left_bottom() + egui::vec2(0.0, 20.0))
                                        .show(&self.main_context, |ui| {
                                            ui.label(format!("{} days\n{} hours\n{} minutes\n{} seconds", current_duration / 86400000, (current_duration % 86400000) / 3600000, (current_duration % 3600000) / 60000, (current_duration % 60000) / 1000));
                                        });
                                }
                            });
                            ui.separator();
                            // Schematic of protocol
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(50.0);
                                    ui.label("Rotation").on_hover_text("Direction 1 for cycle duration ➡️ Pause\nDirection 2 for cycle duration ➡️ Pause\nRepeat for rotation duration");
                                    ui.label("➡️");
                                    ui.label("Pause pre-agitation");
                                });
                                ui.label("⬇️️");
                                ui.horizontal(|ui| {
                                    ui.add_space(50.0);
                                    ui.label("Agitation").on_hover_text("Direction 1 for agitation duration ➡️ Pause\nDirection 2 for agitation duration ➡️ Pause\nRepeat for rotation duration");
                                    ui.label("➡️");
                                    ui.label("Pause post-agitation");
                                });
                                ui.label("⬇️");
                                ui.label("Repeat for global duration").on_hover_text("This duration supersedes all other durations.");
                            });
                        });
                    });
                });
            });
        });
        ui.separator();
        ///// Graphs /////
        let default_color = ui.visuals().extreme_bg_color;
        ui.visuals_mut().extreme_bg_color = THEME.base;
        // Graph Rotation
        egui::ScrollArea::horizontal().id_source("rotation_scroll").show(ui, |ui| {
            let number_rotation_points = self.motor.get(tab).unwrap().get_graph().get_rotation_points().len();
            if number_rotation_points <= MAX_POINTS_GRAPHS {
                let line = Line::new(self.motor.get(tab).unwrap().get_graph().get_rotation_points()).name("Rotation").color(THEME.sapphire);
                egui::plot::Plot::new("rotation_graph")
                    .legend(Legend { position: Corner::RightTop, ..Default::default() })
                    .auto_bounds_x()
                    .auto_bounds_y()
                    .show_background(true)
                    .height(200.0)
                    .label_formatter(move |_s, value| {
                        format!("Time (s): {:.2}\nRPM: {:.0}", value.x, value.y)
                    })
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });
            } else {
                ui.heading(RichText::new("Too many points to display rotation graph.").color(THEME.mauve));
            }
        });
        ui.separator();
        // Graph Agitation
        egui::ScrollArea::horizontal().id_source("agitation_scroll").show(ui, |ui| {
            let number_agitation_points = self.motor.get(tab).unwrap().get_graph().get_agitation_points().len();
            if number_agitation_points <= MAX_POINTS_GRAPHS {
                let line = Line::new(self.motor.get(tab).unwrap().get_graph().get_agitation_points()).name("Agitation").color(THEME.blue);
                egui::plot::Plot::new("agitation_graph")
                    .auto_bounds_x()
                    .auto_bounds_y()
                    .show_background(true)
                    .legend(Legend { position: Corner::RightTop, ..Default::default() })
                    .height(200.0)
                    .label_formatter(move |_s, value| {
                        format!("Time (s): {:.2}\nRPM: {:.0}", value.x, value.y)
                    })
                    .show(ui, |plot_ui| {
                        plot_ui.line(line);
                    });
            } else {
                ui.heading(RichText::new("Too many points to display agitation graph.").color(THEME.mauve));
            }
        });
        ui.visuals_mut().extreme_bg_color = default_color;
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        if self.motor.get(tab).is_none() { // Avoid panic while the tab is removed.
            return "Motor".into();
        }
        let is_connected = self.motor.get(tab).unwrap().get_is_connected();
        let is_running = self.motor.get(tab).unwrap().get_is_running();
        let motor_name = self.motor.get(tab).unwrap().get_name().to_string();
        format!("{}-{}{}",
                if !motor_name.is_empty() { motor_name } else { tab.to_string() },
                if is_connected { "🔗" } else { "🚫" },
                if is_running { "▶️" } else { "⏹️" },
        ).into()
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        let is_running = self.motor.get(tab).unwrap().get_is_running();
        if is_running {
            let message: Message = Message::new(ToastKind::Warning,
                                                "Motor is running! Please stop the motor before closing the tab."
                                                , None, Some(self.motor.get(tab).unwrap().get_name().to_string())
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
