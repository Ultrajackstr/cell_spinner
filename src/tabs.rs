use std::sync::Arc;
use std::thread;

use dashmap::DashMap;
use egui::{Color32, RichText, Ui, WidgetText};
use egui::plot::{Corner, Legend, Line};
use egui_dock::{NodeIndex, TabViewer};
use egui_toast::ToastKind;
use parking_lot::Mutex;

use crate::app::{FONT_BUTTON_SIZE, MAX_ACCELERATION, MAX_POINTS_GRAPHS, THEME};
use crate::utils::enums::{Direction, StepperState};
use crate::utils::motor::Motor;
use crate::utils::structs::{Channels, DurationHelper, Durations, Message};
use crate::utils::widget_rotating_tube::RotatingTube;

pub struct Tabs<'a> {
    pub channels: &'a mut Channels,
    pub main_context: egui::Context,
    // pub frame: &'a mut Frame,
    pub available_ports: &'a mut Vec<String>,
    pub already_connected_ports: &'a mut Arc<Mutex<Vec<String>>>,
    pub selected_port: &'a mut DashMap<usize, String>,
    pub motor_name: &'a mut DashMap<usize, String>,
    pub motor: &'a mut Arc<DashMap<usize, Motor>>,
    pub durations: &'a mut DashMap<usize, Durations>,
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
        self.durations.insert(tab, Durations::default());
        self.motor.get_mut(&tab).unwrap().name = format!("Motor {}", tab);
        self.motor_name.insert(tab, format!("Motor {}", tab));
        self.added_tabs.push(tab);
        self.refresh_available_serial_ports(tab);
        self.selected_port.insert(tab, self.available_ports[0].clone());
    }

    fn remove_tab(&mut self, tab: usize) {
        self.already_connected_ports.lock().retain(|x| *x != self.motor.get(&tab).unwrap().serial.port_name);
        self.motor.get_mut(&tab).unwrap().disconnect();
        self.selected_port.remove(&tab);
        self.promise_serial_connect.remove(&tab);
        self.motor_name.remove(&tab);
        self.motor.remove(&tab);
        self.durations.remove(&tab);
        self.added_tabs.retain(|x| x != &tab);
    }

    fn thread_spawn_new_motor(&mut self, tab: usize, serial_port: String, motor_name: String) {
        self.promise_serial_connect.insert(tab, Some(()));
        let promise = self.promise_serial_connect.clone();
        let motors = self.motor.clone();
        let channels = self.channels.message_tx.clone();
        let already_connected_ports = self.already_connected_ports.clone();
        let protocol = self.motor.get(&tab).unwrap().protocol;
        let graph = self.motor.get(&tab).unwrap().graph.clone();
        let steps_per_cycle = self.motor.get(&tab).unwrap().steps_per_cycle.clone();
        thread::spawn(move || {
            let motor = match Motor::new_with_already_loaded_protocol(serial_port, motor_name, already_connected_ports, protocol, graph, steps_per_cycle) {
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
                    .filter(|port| !self.already_connected_ports.lock().contains(port)).collect();
                available_ports
            }
            Err(err) => {
                let error = anyhow::anyhow!(err);
                self.channels.message_tx.as_ref().unwrap().send(Message::new(ToastKind::Error, "Error while listing serial ports", Some(error), Some(format!("Motor {}", tab)), 3, false)).ok();
                vec!["".to_string()]
            }
        };
        *self.available_ports = available_ports;
        self.selected_port.insert(tab, self.available_ports[0].clone());
    }

    fn disconnect(&mut self, tab: usize) {
        self.already_connected_ports.lock().retain(|x| *x != self.motor.get(&tab).unwrap().serial.port_name);
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
        self.motor.get_mut(tab).unwrap().frame_hisory.on_new_frame(self.main_context.input(|i| i.time), None);
        let frame_time_sec = 1.0 / self.motor.get(tab).unwrap().frame_hisory.fps();
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
                                self.motor.get_mut(tab).unwrap().name = self.motor_name.get(tab).unwrap().to_string();
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
                    });
                ////////////////////////////
                ui.separator();
                ui.horizontal_centered(|ui| {
                    // Button to send the parameters to the motor and run it. Focus is check to prevent the button from being pressed when the user is typing in the text field.
                    ui.add_enabled_ui(is_connected && !is_running && self.main_context.memory(|mem| mem.focus().is_none() && self.motor.get(tab).unwrap().protocol.global_duration_ms != 0), |ui| {
                        let run_response = ui.add_sized(egui::vec2(FONT_BUTTON_SIZE.button_default.x, FONT_BUTTON_SIZE.button_default.y * 2.0), egui::Button::new(RichText::new("Run")
                            .color(Color32::WHITE)).fill(THEME.green))
                            .on_hover_text("Right click to start all motors");
                        if run_response.clicked() {
                            self.motor.get_mut(tab).unwrap().start_motor(self.channels.message_tx.clone());
                            self.durations.get_mut(tab).unwrap().rotation_duration.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.rotation_duration_ms);
                            self.durations.get_mut(tab).unwrap().agitation_duration.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.agitation_duration_ms);
                            self.durations.get_mut(tab).unwrap().pause_pre_agitation.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.pause_pre_agitation_ms);
                            self.durations.get_mut(tab).unwrap().pause_post_agitation.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.pause_post_agitation_ms);
                            self.durations.get_mut(tab).unwrap().global_duration.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.global_duration_ms);
                        } else if run_response.secondary_clicked() {
                            // Start all the connected motors that are not running
                            self.motor.iter_mut().for_each(|mut motor| {
                                if motor.get_is_connected() && !motor.get_is_running() {
                                    motor.start_motor(self.channels.message_tx.clone());
                                    let tab = *motor.key();
                                    self.durations.get_mut(&tab).unwrap().rotation_duration.self_from_milliseconds(motor.protocol.rotation_duration_ms);
                                    self.durations.get_mut(&tab).unwrap().agitation_duration.self_from_milliseconds(motor.protocol.agitation_duration_ms);
                                    self.durations.get_mut(&tab).unwrap().pause_pre_agitation.self_from_milliseconds(motor.protocol.pause_pre_agitation_ms);
                                    self.durations.get_mut(&tab).unwrap().pause_post_agitation.self_from_milliseconds(motor.protocol.pause_post_agitation_ms);
                                    self.durations.get_mut(&tab).unwrap().global_duration.self_from_milliseconds(motor.protocol.global_duration_ms);
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
                ui.separator();
                // Display run time
                // Convert the run time to days, hours, minutes, seconds and milliseconds.
                let run_time_ms = self.motor.get(tab).unwrap().timers_and_phases.lock().get_elapsed_time_since_global_start_as_millis();
                let is_stop_time = self.motor.get(tab).unwrap().timers_and_phases.lock().global_stop_time_ms;
                if run_time_ms != 0 && is_stop_time.is_none() {
                    let duration = DurationHelper::new_from_milliseconds(run_time_ms);
                    // Run time text.
                    ui.label(RichText::new("Current run time ➡️").size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} d", duration.days)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} h", duration.hours)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} min", duration.minutes)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} s", duration.seconds)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} ms", duration.milliseconds)).size(FONT_BUTTON_SIZE.font_large));
                } else if is_stop_time.is_some() {
                    let stop_time_ms = is_stop_time.unwrap();
                    let duration = DurationHelper::new_from_milliseconds(stop_time_ms);
                    // Run time text.
                    ui.label(RichText::new("Last session duration ➡️").size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} d", duration.days)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} h", duration.hours)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} min", duration.minutes)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} s", duration.seconds)).size(FONT_BUTTON_SIZE.font_large));
                    ui.label(RichText::new(format!("{} ms", duration.milliseconds)).size(FONT_BUTTON_SIZE.font_large));
                } else {
                    ui.label(RichText::new("Run time ➡️ None").size(FONT_BUTTON_SIZE.font_large));
                }
            });
        });
        ui.separator();
        ////// SETUP //////
        egui::ScrollArea::horizontal().id_source("setup").show(ui, |ui| {
            ui.horizontal(|ui| {
                // Setup rotation phase
                let mut rotation_graph_needs_update = false;
                let current_main_phase = self.motor.get(tab).unwrap().timers_and_phases.lock().main_phase;
                ui.allocate_ui(egui::vec2(440.0, 280.0), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Rotation ⬇️").color(THEME.sapphire).size(FONT_BUTTON_SIZE.font_large));
                            ui.separator();
                            // Rotation progress bar
                            let rotation_duration_ms = self.motor.get(tab).unwrap().protocol.rotation_duration_ms;
                            let current_rotation_duration_ms = if let Some(duration) = self.motor.get(tab).unwrap().timers_and_phases.lock().main_phase_start_time {
                                if current_main_phase == StepperState::StartRotation {
                                    duration.elapsed().as_millis() as u64
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            let progress = current_rotation_duration_ms as f32 / rotation_duration_ms as f32;
                            ui.add(egui::ProgressBar::new(progress).show_percentage())
                                .on_hover_text("Rotation progress");
                        });
                        ui.separator();
                        ui.add_enabled_ui(!is_running, |ui| {
                            egui::Grid::new("rotation_grid")
                                .show(ui, |ui| {
                                    // Slider for RPM
                                    ui.label("RPM:");
                                    let max_rpm = self.motor.get(tab).unwrap().protocol.rotation.max_rpm_for_stepmode();
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().protocol.rotation.rpm, 1..=max_rpm)).changed() {
                                        rotation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // Slider for acceleration
                                    ui.label("Acceleration:");
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().protocol.rotation.acceleration, 1..=MAX_ACCELERATION)).changed() {
                                        rotation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // List for stepmode
                                    let modes = self.motor.get(tab).unwrap().protocol.rotation.step_mode.get_modes();
                                    let selected_mode = self.motor.get(tab).unwrap().protocol.rotation.step_mode;
                                    ui.label("Step mode:");
                                    ui.horizontal(|ui| {
                                        egui::ComboBox::from_id_source("step_mode_rotation")
                                            .selected_text(selected_mode.to_string())
                                            .show_ui(ui, |ui| {
                                                for mode in modes {
                                                    if ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().protocol.rotation.step_mode, mode, mode.to_string()).changed() {
                                                        rotation_graph_needs_update = true;
                                                    }
                                                }
                                            });
                                        ui.separator();
                                        ui.label(format!("Rev: {:.2}", self.motor.get(tab).unwrap().get_revolutions_per_rotation_cycle()))
                                            .on_hover_text("Number of revolutions per rotation cycle.");
                                    });

                                    ui.end_row();
                                    // Duration for 1 direction cycle
                                    ui.label("Cycle duration:").on_hover_text("Duration of a cycle of rotations in one direction.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_rotation.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_rotation.to_milliseconds();
                                            rotation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_rotation.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_rotation.to_milliseconds();
                                            rotation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_rotation.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_rotation.to_milliseconds();
                                            rotation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_rotation.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_rotation.to_milliseconds();
                                            rotation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_rotation.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_rotation.to_milliseconds();
                                            rotation_graph_needs_update = true;
                                        }
                                    });
                                    ui.end_row();
                                    // Direction
                                    let directions: [Direction; 2] = [Direction::Forward, Direction::Backward];
                                    let selected_direction = self.motor.get(tab).unwrap().protocol.rotation.direction;
                                    ui.label("Initial direction:");
                                    egui::ComboBox::from_id_source("direction_rotation")
                                        .selected_text(selected_direction.to_string())
                                        .show_ui(ui, |ui| {
                                            for direction in directions {
                                                ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().protocol.rotation.direction, direction, direction.to_string());
                                            }
                                        });
                                    ui.end_row();
                                    // Pause before direction change
                                    ui.label("Pause:").on_hover_text("Pause before changing the direction of rotation.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_rotation.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_rotation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_rotation.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_rotation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_rotation.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_rotation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_rotation.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_rotation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_rotation.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_rotation.to_milliseconds();
                                        }
                                    });
                                    ui.end_row();
                                    // Slider for rotation duration
                                    ui.label("Rotation duration:").on_hover_text("Duration of the rotation phase.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().rotation_duration.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation_duration_ms = self.durations.get(tab).unwrap().rotation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().rotation_duration.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation_duration_ms = self.durations.get(tab).unwrap().rotation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().rotation_duration.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation_duration_ms = self.durations.get(tab).unwrap().rotation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().rotation_duration.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation_duration_ms = self.durations.get(tab).unwrap().rotation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().rotation_duration.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.rotation_duration_ms = self.durations.get(tab).unwrap().rotation_duration.to_milliseconds();
                                        }
                                    });
                                    ui.end_row();
                                    // Slider for pause before agitation
                                    ui.label("Pause pre-agitation:").on_hover_text("Pause before the agitation phase.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_pre_agitation.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_pre_agitation_ms = self.durations.get(tab).unwrap().pause_pre_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_pre_agitation.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_pre_agitation_ms = self.durations.get(tab).unwrap().pause_pre_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_pre_agitation.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_pre_agitation_ms = self.durations.get(tab).unwrap().pause_pre_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_pre_agitation.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_pre_agitation_ms = self.durations.get(tab).unwrap().pause_pre_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_pre_agitation.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_pre_agitation_ms = self.durations.get(tab).unwrap().pause_pre_agitation.to_milliseconds();
                                        }
                                    });
                                });
                            if rotation_graph_needs_update {
                                let max_rpm_rotation = self.motor.get(tab).unwrap().protocol.rotation.max_rpm_for_stepmode();
                                let current_rpm_rotation = self.motor.get(tab).unwrap().protocol.rotation.rpm;
                                if current_rpm_rotation > max_rpm_rotation {
                                    self.motor.get_mut(tab).unwrap().protocol.rotation.rpm = max_rpm_rotation;
                                }
                                self.motor.get(tab).unwrap().generate_graph_rotation();
                                rotation_graph_needs_update = false;
                            }
                        });
                    });
                });
                ui.separator();
                // Setup agitation phase
                let mut agitation_graph_needs_update = false;
                ui.allocate_ui(egui::vec2(440.0, 280.0), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Agitation ⬇️").color(THEME.blue).size(FONT_BUTTON_SIZE.font_large));
                            ui.separator();
                            // Agitation progress bar
                            let agitation_duration_ms = self.motor.get(tab).unwrap().protocol.agitation_duration_ms;
                            let current_agitation_duration_ms = if let Some(duration) = self.motor.get(tab).unwrap().timers_and_phases.lock().main_phase_start_time {
                                if current_main_phase == StepperState::StartAgitation {
                                    duration.elapsed().as_millis() as u64
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            let progress = current_agitation_duration_ms as f32 / agitation_duration_ms as f32;
                            ui.add(egui::ProgressBar::new(progress).show_percentage())
                                .on_hover_text("Agitation progress");
                        });
                        ui.separator();
                        ui.add_enabled_ui(!is_running, |ui| {
                            egui::Grid::new("agitation_grid")
                                .show(ui, |ui| {
                                    // Slider for RPM
                                    ui.label("RPM:");
                                    let max_rpm = self.motor.get(tab).unwrap().protocol.agitation.max_rpm_for_stepmode();
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().protocol.agitation.rpm, 1..=max_rpm)).changed() {
                                        agitation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // Slider for acceleration
                                    ui.label("Acceleration:");
                                    if ui.add(egui::Slider::new(&mut self.motor.get_mut(tab).unwrap().protocol.agitation.acceleration, 1..=MAX_ACCELERATION)).changed() {
                                        agitation_graph_needs_update = true;
                                    }
                                    ui.end_row();
                                    // List for stepmode
                                    let modes = self.motor.get(tab).unwrap().protocol.agitation.step_mode.get_modes();
                                    let selected_mode = self.motor.get(tab).unwrap().protocol.agitation.step_mode;
                                    ui.label("Step mode:");
                                    ui.horizontal(|ui| {
                                        egui::ComboBox::from_id_source("step_mode_agitation")
                                            .selected_text(selected_mode.to_string())
                                            .show_ui(ui, |ui| {
                                                for mode in modes {
                                                    if ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().protocol.agitation.step_mode, mode, mode.to_string()).changed() {
                                                        agitation_graph_needs_update = true;
                                                    }
                                                }
                                            });
                                        ui.separator();
                                        ui.label(format!("Rev: {:.2}", self.motor.get(tab).unwrap().get_revolutions_per_agitation_cycle()))
                                            .on_hover_text("Number of revolutions per agitation cycle.");
                                    });

                                    ui.end_row();
                                    // Duration for 1 direction cycle
                                    ui.label("Cycle duration:").on_hover_text("Duration of a cycle of agitations in one direction.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_agitation.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_agitation.to_milliseconds();
                                            agitation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_agitation.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_agitation.to_milliseconds();
                                            agitation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_agitation.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_agitation.to_milliseconds();
                                            agitation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_agitation.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_agitation.to_milliseconds();
                                            agitation_graph_needs_update = true;
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_agitation.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms = self.durations.get(tab).unwrap().duration_of_one_direction_cycle_agitation.to_milliseconds();
                                            agitation_graph_needs_update = true;
                                        }
                                    });
                                    ui.end_row();
                                    // Direction
                                    let directions: [Direction; 2] = [Direction::Forward, Direction::Backward];
                                    let selected_direction = self.motor.get(tab).unwrap().protocol.agitation.direction;
                                    ui.label("Initial direction:");
                                    egui::ComboBox::from_id_source("direction_agitation")
                                        .selected_text(selected_direction.to_string())
                                        .show_ui(ui, |ui| {
                                            for direction in directions {
                                                ui.selectable_value(&mut self.motor.get_mut(tab).unwrap().protocol.agitation.direction, direction, direction.to_string());
                                            }
                                        });
                                    ui.end_row();
                                    // Pause before direction change
                                    ui.label("Pause:").on_hover_text("Pause before changing the direction of agitation.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_agitation.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_agitation.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_agitation.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_agitation.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_before_direction_change_agitation.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation.pause_before_direction_change_ms = self.durations.get(tab).unwrap().pause_before_direction_change_agitation.to_milliseconds();
                                        }
                                    });
                                    ui.end_row();
                                    // Slider for agitation duration
                                    ui.label("Agitation duration:").on_hover_text("Duration of the agitation phase.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().agitation_duration.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation_duration_ms = self.durations.get(tab).unwrap().agitation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().agitation_duration.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation_duration_ms = self.durations.get(tab).unwrap().agitation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().agitation_duration.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation_duration_ms = self.durations.get(tab).unwrap().agitation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().agitation_duration.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation_duration_ms = self.durations.get(tab).unwrap().agitation_duration.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().agitation_duration.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.agitation_duration_ms = self.durations.get(tab).unwrap().agitation_duration.to_milliseconds();
                                        }
                                    });
                                    ui.end_row();
                                    // Slider for pause after agitation
                                    ui.label("Pause post-agitation:").on_hover_text("Pause after the agitation phase.");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_post_agitation.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_post_agitation_ms = self.durations.get(tab).unwrap().pause_post_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_post_agitation.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_post_agitation_ms = self.durations.get(tab).unwrap().pause_post_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_post_agitation.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_post_agitation_ms = self.durations.get(tab).unwrap().pause_post_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_post_agitation.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_post_agitation_ms = self.durations.get(tab).unwrap().pause_post_agitation.to_milliseconds();
                                        }
                                        if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().pause_post_agitation.milliseconds).suffix(" ms").speed(3.0).speed(3.0).clamp_range(0..=999)).changed() {
                                            self.motor.get_mut(tab).unwrap().protocol.pause_post_agitation_ms = self.durations.get(tab).unwrap().pause_post_agitation.to_milliseconds();
                                        }
                                    });
                                });
                        });
                        if agitation_graph_needs_update {
                            let max_rpm_agitation = self.motor.get(tab).unwrap().protocol.agitation.max_rpm_for_stepmode();
                            let current_rpm_agitation = self.motor.get(tab).unwrap().protocol.agitation.rpm;
                            if current_rpm_agitation > max_rpm_agitation {
                                self.motor.get_mut(tab).unwrap().protocol.agitation.rpm = max_rpm_agitation;
                            }
                            self.motor.get(tab).unwrap().generate_graph_agitation();
                            agitation_graph_needs_update = false;
                        }
                    });
                });
                ui.separator();
                // Setup durations
                ui.allocate_ui(egui::vec2(440.0, 280.0), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Global Duration ⬇️").color(THEME.lavender).size(FONT_BUTTON_SIZE.font_large));
                            ui.separator();
                            // Global progress
                            let global_duration_ms = self.motor.get(tab).unwrap().protocol.global_duration_ms;
                            let current_global_duration_ms = if let Some(duration) = self.motor.get(tab).unwrap().timers_and_phases.lock().global_start_time {
                                if is_running {
                                    duration.elapsed().as_millis() as u64
                                } else {
                                    0
                                }
                            } else {
                                0
                            };
                            let progress = current_global_duration_ms as f32 / global_duration_ms as f32;
                            ui.add(egui::ProgressBar::new(progress).show_percentage())
                                .on_hover_text("Global progress");
                        });
                        ui.separator();
                        ui.add_enabled_ui(!is_running, |ui| {
                            // Global duration of the protocol
                            ui.horizontal(|ui| {
                                let color = if self.motor.get(tab).unwrap().protocol.global_duration_ms == 0 { THEME.red } else { THEME.text };
                                ui.label(RichText::new("Global duration:").color(color).size(15.0)).on_hover_text("Global duration of the protocol.");
                                ui.horizontal(|ui| {
                                    if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().global_duration.days).suffix(" d").speed(2.0).clamp_range(0..=364)).changed() {
                                        self.motor.get_mut(tab).unwrap().protocol.global_duration_ms = self.durations.get(tab).unwrap().global_duration.to_milliseconds();
                                    }
                                    if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().global_duration.hours).suffix(" h").clamp_range(0..=23)).changed() {
                                        self.motor.get_mut(tab).unwrap().protocol.global_duration_ms = self.durations.get(tab).unwrap().global_duration.to_milliseconds();
                                    }
                                    if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().global_duration.minutes).suffix(" min").clamp_range(0..=59)).changed() {
                                        self.motor.get_mut(tab).unwrap().protocol.global_duration_ms = self.durations.get(tab).unwrap().global_duration.to_milliseconds();
                                    }
                                    if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().global_duration.seconds).suffix(" s").clamp_range(0..=59)).changed() {
                                        self.motor.get_mut(tab).unwrap().protocol.global_duration_ms = self.durations.get(tab).unwrap().global_duration.to_milliseconds();
                                    }
                                    if ui.add(egui::DragValue::new(&mut self.durations.get_mut(tab).unwrap().global_duration.milliseconds).suffix(" ms").speed(3.0).clamp_range(0..=999)).changed() {
                                        self.motor.get_mut(tab).unwrap().protocol.global_duration_ms = self.durations.get(tab).unwrap().global_duration.to_milliseconds();
                                    }
                                });
                            });
                        });
                        ui.separator();
                        ui.label(RichText::new("Current phase ⬇️").color(THEME.mauve).size(FONT_BUTTON_SIZE.font_large));
                        ui.vertical(|ui| {
                            let current_main_phase = self.motor.get(tab).unwrap().timers_and_phases.lock().main_phase;
                            let run_time_current_main_phase_ms = self.motor.get(tab).unwrap().timers_and_phases.lock().get_elapsed_time_since_main_phase_start_as_millis();
                            let current_sub_phase = self.motor.get(tab).unwrap().timers_and_phases.lock().sub_phase;
                            let run_time_current_sub_phase_ms = self.motor.get(tab).unwrap().timers_and_phases.lock().get_elapsed_time_since_sub_phase_start_as_millis();
                            egui::Grid::new("phases")
                                .min_col_width(140.0)
                                .show(ui, |ui| {
                                    if run_time_current_main_phase_ms != 0 {
                                        ui.label(RichText::new(current_main_phase.to_string()).size(FONT_BUTTON_SIZE.font_large));
                                        let duration = DurationHelper::new_from_milliseconds(run_time_current_main_phase_ms);
                                        let run_time_global_current_phase = format!("{} d {} h {} min {} s {} ms", duration.days, duration.hours, duration.minutes, duration.seconds, duration.milliseconds);
                                        ui.label(RichText::new(run_time_global_current_phase).size(FONT_BUTTON_SIZE.font_large));
                                        ui.end_row();
                                        if run_time_current_sub_phase_ms != 0 {
                                            ui.label(current_sub_phase.to_string());
                                            let duration = DurationHelper::new_from_milliseconds(run_time_current_sub_phase_ms);
                                            ui.label(RichText::new(format!("{} d {} h {} min {} s {} ms", duration.days, duration.hours, duration.minutes, duration.seconds, duration.milliseconds)));
                                        } else {
                                            ui.label("");
                                        }
                                    } else {
                                        ui.label(RichText::new(current_sub_phase.to_string()).size(FONT_BUTTON_SIZE.font_large));
                                        ui.end_row();
                                        ui.label("");
                                    }
                                });
                            //// Rotation & Agitation widgets
                            ui.horizontal(|ui| {
                                let diameter = 65.0;
                                // Rotation
                                let mut rotation_widget = RotatingTube::new(diameter, THEME.sapphire);
                                if is_running && current_main_phase == StepperState::StartRotation && current_sub_phase != StepperState::StartPausePreAgitation {
                                    let mut rpm = 0;
                                    self.motor.get(tab).unwrap().graph.rotation_points_sec_rpm.lock().iter().any(|point| {
                                        if point[0] * 1000.0 >= run_time_current_sub_phase_ms as f64 {
                                            rpm = point[1].round() as u32;
                                            true
                                        } else { false }
                                    });
                                    rotation_widget.rpm = rpm;
                                    let direction = self.motor.get(tab).unwrap().timers_and_phases.lock().rotation_direction;
                                    if direction == Direction::Forward {
                                        self.motor.get_mut(tab).unwrap().angle_rotation += rpm as f32 * 6.0 * frame_time_sec;
                                    } else { self.motor.get_mut(tab).unwrap().angle_rotation -= rpm as f32 * 6.0 * frame_time_sec; }
                                    // Reduce to modulo 360 to avoid overflow/underflow
                                    if self.motor.get(tab).unwrap().angle_rotation >= 360.0 {
                                        self.motor.get_mut(tab).unwrap().angle_rotation -= 360.0;
                                    }
                                    if self.motor.get(tab).unwrap().angle_rotation <= -360.0 {
                                        self.motor.get_mut(tab).unwrap().angle_rotation += 360.0;
                                    }
                                    rotation_widget.angle_degrees = self.motor.get(tab).unwrap().angle_rotation;
                                }
                                ui.add(rotation_widget).on_hover_text("Rotation");
                                ui.add_space(140.0 - diameter);
                                // Agitation
                                let mut agitation_widget = RotatingTube::new(diameter, THEME.blue);
                                if is_running && current_main_phase == StepperState::StartAgitation && current_sub_phase != StepperState::StartPausePostAgitation {
                                    let mut rpm = 0;
                                    self.motor.get(tab).unwrap().graph.agitation_points_sec_rpm.lock().iter().any(|point| {
                                        if point[0] * 1000.0 >= run_time_current_sub_phase_ms as f64 {
                                            rpm = point[1].round() as u32;
                                            true
                                        } else { false }
                                    });
                                    agitation_widget.rpm = rpm;
                                    let direction = self.motor.get(tab).unwrap().timers_and_phases.lock().agitation_direction;
                                    if direction == Direction::Forward {
                                        self.motor.get_mut(tab).unwrap().angle_agitation += rpm as f32 * 6.0 * frame_time_sec;
                                    } else { self.motor.get_mut(tab).unwrap().angle_agitation -= rpm as f32 * 6.0 * frame_time_sec; }
                                    // Reduce to modulo 360 to avoid overflow/underflow
                                    if self.motor.get(tab).unwrap().angle_agitation >= 360.0 {
                                        self.motor.get_mut(tab).unwrap().angle_agitation -= 360.0;
                                    }
                                    if self.motor.get(tab).unwrap().angle_agitation <= -360.0 {
                                        self.motor.get_mut(tab).unwrap().angle_agitation += 360.0;
                                    }
                                    agitation_widget.angle_degrees = self.motor.get(tab).unwrap().angle_agitation;
                                }
                                ui.add(agitation_widget).on_hover_text("Agitation");
                            });
                        });
                        // Schematic of protocol
                        // ui.vertical_centered(|ui| {
                        //     ui.horizontal(|ui| {
                        //         ui.label(RichText::new("Rotation").color(THEME.sapphire).size(FONT_BUTTON_SIZE.font_large)).on_hover_text("Direction 1 for cycle duration ➡️ Pause\nDirection 2 for cycle duration ➡️ Pause\nRepeat for rotation duration");
                        //         ui.label(RichText::new("➡️").size(FONT_BUTTON_SIZE.font_large));
                        //         ui.label(RichText::new("Pause pre-agitation").size(FONT_BUTTON_SIZE.font_large));
                        //     });
                        //     ui.label(RichText::new("⬇️").size(FONT_BUTTON_SIZE.font_large));
                        //     ui.horizontal(|ui| {
                        //         ui.label(RichText::new("Agitation").color(THEME.blue).size(FONT_BUTTON_SIZE.font_large)).on_hover_text("Direction 1 for agitation duration ➡️ Pause\nDirection 2 for agitation duration ➡️ Pause\nRepeat for rotation duration");
                        //         ui.label(RichText::new("➡️").size(FONT_BUTTON_SIZE.font_large));
                        //         ui.label(RichText::new("Pause post-agitation").size(FONT_BUTTON_SIZE.font_large));
                        //     });
                        //     ui.label(RichText::new("⬇️").size(FONT_BUTTON_SIZE.font_large));
                        //     ui.label(RichText::new("Repeat for global duration").color(THEME.lavender).size(FONT_BUTTON_SIZE.font_large)).on_hover_text("This duration supersedes all other durations.");
                        // });
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
            let number_rotation_points = self.motor.get(tab).unwrap().graph.rotation_points_sec_rpm.lock().len();
            if number_rotation_points <= MAX_POINTS_GRAPHS {
                let line = Line::new(self.motor.get(tab).unwrap().graph.rotation_points_sec_rpm.lock().clone()).name("Rotation").color(THEME.sapphire);
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
            let number_agitation_points = self.motor.get(tab).unwrap().graph.agitation_points_sec_rpm.lock().len();
            if number_agitation_points <= MAX_POINTS_GRAPHS {
                let line = Line::new(self.motor.get(tab).unwrap().graph.agitation_points_sec_rpm.lock().clone()).name("Agitation").color(THEME.blue);
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
        let motor_name = self.motor.get(tab).unwrap().name.to_string();
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
                                                , None, Some(self.motor.get(tab).unwrap().name.to_string())
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
