use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use catppuccin_egui::{LATTE, Theme};
use chrono::Local;
use dashmap::DashMap;
use dirs::home_dir;
use egui::{Color32, FontFamily, FontId, RichText, Sense};
use egui::TextStyle::{Body, Button, Heading, Monospace, Small};
use egui_dock::{Style, Tree};
use egui_toast::{Toast, ToastKind, Toasts};
use parking_lot::Mutex;
use rfd::FileDialog;

use crate::tabs::Tabs;
use crate::utils::helpers::send_toast;
use crate::utils::motor::Motor;
use crate::utils::protocols::Protocol;
use crate::utils::structs::{Channels, Durations, FontAndButtonSize, Message, WindowsState};

pub const FONT_BUTTON_SIZE: FontAndButtonSize = FontAndButtonSize {
    font_table: 13.0,
    font_default: 14.0,
    font_large: 20.0,
    button_top_panel: egui::vec2(75.0, 20.0),
    button_default: egui::vec2(100.0, 20.0),
};

pub const THREAD_SLEEP: u64 = 10;
pub const MAX_ACCELERATION: u32 = 20_000;
pub const MAX_RPM: u32 = 5_000;
// 1 year in milliseconds
pub const MAX_DURATION_MS: u64 = 365 * 24 * 60 * 60 * 1000;
pub const MAX_POINTS_GRAPHS: usize = 250_000;
pub const BYTES: usize = 110;
pub const THEME: Theme = Theme {
    base: Color32::from_rgb(249, 251, 255),
    ..LATTE
};

// pub const SCHEME: &[u8] = include_bytes!("./resources/schematic/protocol.png");

pub struct CellSpinner {
    app_version: String,
    is_first_frame: bool,
    toast_position_x: f32,
    toast_position_y: f32,
    height: f32,
    width: f32,
    channels: Channels,
    windows_state: WindowsState,
    info_message: String,
    info_message_is_waiting: bool,
    error_log: Vec<String>,
    allowed_to_close: bool,
    // Promises
    promise_serial_connect: Arc<DashMap<usize, Option<()>>>,
    // Serial
    selected_port: DashMap<usize, String>,
    available_ports: Vec<String>,
    already_connected_ports: Arc<Mutex<Vec<String>>>,
    // Motor
    //Only to prevent loss of focus while changing the name of the motor...
    motor_name: DashMap<usize, String>,
    durations: DashMap<usize, Durations>,
    motor: Arc<DashMap<usize, Motor>>,
    // Tabs
    current_tab_counter: usize,
    tree: Tree<usize>,
    absolute_tab_counter: usize,
    added_tabs: Vec<usize>,
    can_tab_close: bool,
    path_config: PathBuf,

}

impl Default for CellSpinner {
    fn default() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            is_first_frame: true,
            toast_position_x: 0.0,
            toast_position_y: 0.0,
            height: 0.0,
            width: 0.0,
            channels: Channels::default(),
            windows_state: WindowsState::default(),
            info_message: "".to_string(),
            info_message_is_waiting: false,
            error_log: vec![],
            allowed_to_close: false,
            promise_serial_connect: Arc::new(Default::default()),
            selected_port: DashMap::new(),
            available_ports: vec![],
            already_connected_ports: Arc::new(Mutex::new(vec![])),
            current_tab_counter: 1,
            tree: Tree::new(vec![1]),
            absolute_tab_counter: 1,
            added_tabs: vec![],
            can_tab_close: false,
            motor: Arc::new(Default::default()),
            motor_name: Default::default(),
            path_config: home_dir().unwrap(),
            durations: Default::default(),
        }
    }
}

impl CellSpinner {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Font setup.
        let mut style = (*cc.egui_ctx.style()).clone(); // Get current context style
        style.text_styles = [
            (Heading, FontId::new(FONT_BUTTON_SIZE.font_large, FontFamily::Proportional)),
            (Body, FontId::new(FONT_BUTTON_SIZE.font_default, FontFamily::Proportional)),
            (Monospace, FontId::new(FONT_BUTTON_SIZE.font_default, FontFamily::Monospace)),
            (Button, FontId::new(FONT_BUTTON_SIZE.font_default, FontFamily::Proportional)),
            (Small, FontId::new(FONT_BUTTON_SIZE.font_table, FontFamily::Proportional)),
        ].into();
        cc.egui_ctx.set_style(style);
        catppuccin_egui::set_theme(&cc.egui_ctx, THEME);
        Default::default()
    }

    /// Function executing on first frame.
    fn startup(&mut self, _ctx: &egui::Context) {
        if !self.is_first_frame {
            return;
        }
        // Setup channels for toast notifications.
        let (toast_tx, toast_rx) = std::sync::mpsc::channel();
        self.channels.toast_tx = Some(toast_tx);
        self.channels.toast_rx = Some(toast_rx);
        let message: Message = Message::new(ToastKind::Info, &format!("Cell Spinner v.{}", self.app_version), None, None, 3, false);
        self.message_handler(message);
        // Setup channels for Message.
        let (message_tx, message_rx) = std::sync::mpsc::channel();
        self.channels.message_tx = Some(message_tx);
        self.channels.message_rx = Some(message_rx);
        self.init_tab(1);
        self.is_first_frame = false;
    }

    /// Message handler.
    fn message_handler(&mut self, message: Message) {
        match message.kind {
            ToastKind::Error => {
                if message.error.is_none() {
                    panic!("Error message without error");
                }
                let text = if let Some(origin) = message.origin {
                    format!("{} 💠 {}: {} - {:?}", Local::now().format("%d-%m-%Y %H:%M:%S"), origin, message.message, message.error.unwrap())
                } else {
                    format!("{} 💠 {} - {:?}", Local::now().format("%d-%m-%Y %H:%M:%S"), message.message, message.error.unwrap())
                };
                tracing::error!(text);
                self.error_log.insert(0, text.clone());
                self.info_message_is_waiting = false;
                send_toast(&self.channels.toast_tx, ToastKind::Error, text, message.duration);
            }
            _ => {
                self.info_message_is_waiting = message.is_waiting;
                let text = if let Some(origin) = message.origin {
                    format!("{}: {}", origin, message.message)
                } else {
                    message.message.to_string()
                };
                if !message.is_waiting {
                    send_toast(&self.channels.toast_tx, message.kind, text, message.duration);
                } else {
                    self.info_message = text;
                }
            }
        }
    }

    /// Init tab
    fn init_tab(&mut self, tab: usize) {
        self.added_tabs.push(tab);
        self.motor.insert(tab, Motor::default());
        self.durations.insert(tab, Durations::default());
        self.motor.get_mut(&tab).unwrap().name = format!("Motor {}", tab);
        self.motor_name.insert(tab, format!("Motor {}", tab));
        let available_ports = match serialport::available_ports() {
            Ok(ports) => {
                let available_ports: Vec<String> = ports.iter().map(|port| port.port_name.clone())
                    .filter(|port| !self.already_connected_ports.lock().contains(port)).collect();
                available_ports
            }
            Err(err) => {
                let error = anyhow!(err);
                self.message_handler(Message::new(ToastKind::Error, "Error while listing serial ports", Some(error), Some(format!("Motor {}", tab)), 3, false));
                vec!["".to_string()]
            }
        };
        self.selected_port.insert(tab, available_ports[0].clone());
        self.available_ports = available_ports;
        self.promise_serial_connect.insert(tab, None);
    }

    /// Error log window.
    fn window_error_log(&mut self, ctx: &egui::Context) {
        if !self.windows_state.is_error_log_open {
            return;
        }
        egui::Window::new("Error Log")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("OK")
                        .color(Color32::WHITE)).fill(THEME.blue))
                        .clicked() {
                        self.windows_state.is_error_log_open = false;
                    }
                    ui.separator();
                    if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("Open log folder")
                        .color(Color32::WHITE)).fill(THEME.sapphire))
                        .clicked() {
                        if let Some(mut path) = home_dir() {
                            path.push("cell_spinner");
                            match Command::new("explorer")
                                .arg(path)
                                .spawn() {
                                Ok(_) => { self.windows_state.is_error_log_open = false; }
                                Err(err) => {
                                    self.message_handler(Message::new(ToastKind::Error, "Error while opening the log folder", Some(anyhow!(err)), None, 3, false));
                                }
                            }
                        }
                    }
                });

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for error in &self.error_log {
                            ui.separator();
                            ui.label(error);
                        }
                    });
            });
    }

    /// Exit confirmation.
    fn window_exit_confirmation(&mut self, ctx: &egui::Context) {
        if !self.windows_state.is_confirmation_dialog_open {
            return;
        }
        egui::Window::new("Disconnect before exit")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Disconnect the board before closing the window.\n ⚠️ You should also stop the motor before disconnecting.");
                ui.separator();
                ui.horizontal(|ui| {
                    // Disconnect button.
                    if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("DISCONNECT ALL").color(Color32::WHITE)).fill(THEME.red)).clicked() {
                        self.motor.iter_mut().for_each(|mut motor| motor.disconnect());
                        self.allowed_to_close = true;
                    }
                    ui.separator();
                    // Cancel button.
                    if ui.add_sized(FONT_BUTTON_SIZE.button_default, egui::Button::new(RichText::new("CANCEL").color(Color32::WHITE)).fill(THEME.blue)).clicked() {
                        self.windows_state.is_confirmation_dialog_open = false;
                    }
                });
            });
    }

    // Export the configuration HashMap to a JSON file.
    fn export_configuration(&mut self, tab: &usize) {
        let mut fn_export = || -> Result<(), Error> {
            self.path_config = FileDialog::new()
                .add_filter("json", &["json"])
                .save_file()
                .unwrap_or_default();
            let mut file = File::create(&self.path_config)?;
            let protocol = self.motor.get(tab).unwrap().protocol;
            let json = serde_json::to_string_pretty(&protocol).unwrap();
            file.write_all(json.as_bytes()).unwrap();
            let current_motor = self.motor.get(tab).unwrap().name.to_string();
            let message: Message = Message::new(ToastKind::Info, "Configuration exported!", None, Some(current_motor), 3, false);
            self.message_handler(message);
            Ok(())
        };
        if let Err(err) = fn_export() {
            let current_motor = self.motor.get(tab).unwrap().name.to_string();
            let message: Message = Message::new(ToastKind::Error, "Error while exporting the configuration", Some(err), Some(current_motor), 3, false);
            self.message_handler(message);
        }
    }

    // Import the configuration from a JSON file.
    fn import_configuration(&mut self, tab: &usize, import_for_all: bool) {
        if self.motor.get(tab).unwrap().get_is_running() {
            return;
        }
        let mut fn_import = || -> Result<(), Error> {
            self.path_config = FileDialog::new()
                .add_filter("json", &["json"])
                .pick_file()
                .unwrap_or_default();
            let file = File::open(&self.path_config)?;
            let reader = BufReader::new(file);
            let protocol: Protocol = serde_json::from_reader(reader)?;
            if import_for_all {
                let mut errors_import: Vec<(String, Error)> = vec![];
                self.motor.iter_mut().for_each(|mut motor| match motor.import_protocol(protocol) {
                    Ok(_) => {}
                    Err(err) => {
                        errors_import.push((motor.name.to_string(), err));
                    }
                });
                if !errors_import.is_empty() {
                    for (motor_name, err) in errors_import.into_iter() {
                        let message: Message = Message::new(ToastKind::Error, "Error while importing the configuration", Some(err), Some(motor_name), 3, false);
                        self.message_handler(message);
                    }
                } else {
                    let message: Message = Message::new(ToastKind::Info, "Configuration imported for all stopped motors!", None, None, 3, false);
                    self.message_handler(message);
                }
                self.durations.iter_mut().for_each(|mut durations| {
                    let key = *durations.key();
                    durations.duration_of_one_direction_cycle_rotation.self_from_milliseconds(self.motor.get(&key).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms);
                    durations.pause_before_direction_change_rotation.self_from_milliseconds(self.motor.get(&key).unwrap().protocol.rotation.pause_before_direction_change_ms);
                    durations.duration_of_one_direction_cycle_agitation.self_from_milliseconds(self.motor.get(&key).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms);
                    durations.pause_before_direction_change_agitation.self_from_milliseconds(self.motor.get(&key).unwrap().protocol.agitation.pause_before_direction_change_ms);
                    let rotation_duration = self.motor.get(&key).unwrap().protocol.rotation_duration_ms;
                    let agitation_duration = self.motor.get(&key).unwrap().protocol.agitation_duration_ms;
                    durations.rotation_duration.self_from_milliseconds(rotation_duration);
                    durations.agitation_duration.self_from_milliseconds(agitation_duration);
                    let pause_pre_agitation = self.motor.get(&key).unwrap().protocol.pause_pre_agitation_ms;
                    let pause_post_agitation = self.motor.get(&key).unwrap().protocol.pause_post_agitation_ms;
                    durations.pause_pre_agitation.self_from_milliseconds(pause_pre_agitation);
                    durations.pause_post_agitation.self_from_milliseconds(pause_post_agitation);
                    durations.global_duration.self_from_milliseconds(self.motor.get(&key).unwrap().protocol.global_duration_ms);
                });
                self.motor.iter().for_each(|motor| {
                    motor.generate_graph_rotation();
                    motor.generate_graph_agitation();
                });
            } else {
                self.motor.get_mut(tab).unwrap().import_protocol(protocol)?;
                let current_motor = self.motor.get(tab).unwrap().name.to_string();
                let message: Message = Message::new(ToastKind::Info, "Configuration imported!", None, Some(current_motor), 3, false);
                self.message_handler(message);
                self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_rotation.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.rotation.duration_of_one_direction_cycle_ms);
                self.durations.get_mut(tab).unwrap().pause_before_direction_change_rotation.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.rotation.pause_before_direction_change_ms);
                self.durations.get_mut(tab).unwrap().duration_of_one_direction_cycle_agitation.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.agitation.duration_of_one_direction_cycle_ms);
                self.durations.get_mut(tab).unwrap().pause_before_direction_change_agitation.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.agitation.pause_before_direction_change_ms);
                let rotation_duration = self.motor.get(tab).unwrap().protocol.rotation_duration_ms;
                let agitation_duration = self.motor.get(tab).unwrap().protocol.agitation_duration_ms;
                self.durations.get_mut(tab).unwrap().rotation_duration.self_from_milliseconds(rotation_duration);
                self.durations.get_mut(tab).unwrap().agitation_duration.self_from_milliseconds(agitation_duration);
                let pause_pre_agitation = self.motor.get(tab).unwrap().protocol.pause_pre_agitation_ms;
                let pause_post_agitation = self.motor.get(tab).unwrap().protocol.pause_post_agitation_ms;
                self.durations.get_mut(tab).unwrap().pause_pre_agitation.self_from_milliseconds(pause_pre_agitation);
                self.durations.get_mut(tab).unwrap().pause_post_agitation.self_from_milliseconds(pause_post_agitation);
                self.durations.get_mut(tab).unwrap().global_duration.self_from_milliseconds(self.motor.get(tab).unwrap().protocol.global_duration_ms);
                self.motor.get(tab).unwrap().generate_graph_rotation();
                self.motor.get(tab).unwrap().generate_graph_agitation();
            }
            Ok(())
        };
        if let Err(err) = fn_import() {
            let current_motor = self.motor.get(tab).unwrap().name.to_string();
            let message: Message = Message::new(ToastKind::Error, "Error while importing the configuration", Some(err), Some(current_motor), 3, false);
            self.message_handler(message);
        }
    }
}

impl eframe::App for CellSpinner {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        /////////////////////////////////////////////
        // Function executing only on first frame. //
        /////////////////////////////////////////////
        self.startup(ctx);

        /////////////////////////////////////
        // Functions executing each frame. //
        /////////////////////////////////////
        // Toasts
        self.height = frame.info().window_info.size.y;
        self.width = frame.info().window_info.size.x;
        self.toast_position_x = self.width - 10.0;
        self.toast_position_y = self.height - 10.0;
        let mut toasts = Toasts::new()
            .anchor((self.toast_position_x, self.toast_position_y))
            .direction(egui::Direction::BottomUp)
            .align_to_end(true)
            .progress_bar(THEME.mauve, 3.0, THEME.crust);

        // Check if new toasts have been sent.
        if let Some(toast_rx) = &self.channels.toast_rx {
            if let Ok(msg) = toast_rx.try_recv() {
                toasts.add(Toast { text: msg.text, kind: msg.kind, options: msg.options });
            }
        }

        // Check if new messages have been sent.
        if let Some(message_rx) = &self.channels.message_rx {
            if let Ok(msg) = message_rx.try_recv() {
                self.message_handler(msg);
            }
        }

        // Display toasts
        toasts.show(ctx);

        self.window_error_log(ctx);
        self.window_exit_confirmation(ctx);

        if self.allowed_to_close {
            frame.close();
        }

        // Always repaint the UI, even if no events occurred.
        ctx.request_repaint();

        ////////////////////////////////////////////////////////////////////////////////
        ////////////////////////////////////////////////////////////////////////////////
        ///////////////
        // Top Panel //
        ///////////////
        egui::TopBottomPanel::top("top_panel")
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::ScrollArea::horizontal().id_source("Top_scroll_area").show(ui, |ui| {
                        let tab;
                        if let Some(active_tab) = self.tree.find_active_focused() {
                            tab = *active_tab.1;
                        } else {
                            tab = self.added_tabs[0];
                        }
                        let is_running = self.motor.get(&tab).unwrap().get_is_running();
                        // Title
                        let response_heading = ui.add(egui::Label::new(RichText::new("Cell Spinner").heading())
                            .sense(Sense::click()))
                            .on_hover_text(format!("Version {} - Giacomo Gropplero - Copyright © 2023", self.app_version));
                        if response_heading.secondary_clicked() {
                            self.windows_state.is_error_log_open = !self.windows_state.is_error_log_open;
                        };
                        ui.separator();
                        // Buttons to save and load config.
                        if ui.add_sized(FONT_BUTTON_SIZE.button_top_panel, egui::Button::new("Save config").fill(THEME.surface0))
                            .clicked() {
                            self.export_configuration(&tab);
                        }
                        ui.separator();
                        ui.add_enabled_ui(!is_running, |ui| {
                            let import_response = ui.add_sized(FONT_BUTTON_SIZE.button_top_panel, egui::Button::new("Import config").fill(THEME.surface0))
                                .on_hover_text("Right click to import config for all the motors");
                            if import_response.clicked() {
                                self.import_configuration(&tab, false);
                            } else if import_response.secondary_clicked() {
                                self.import_configuration(&tab, true);
                            }
                        });
                        // Info message
                        ui.add_visible_ui(self.info_message_is_waiting, |ui| {
                            ui.separator();
                            ui.spinner();
                            ui.label(&self.info_message);
                        });
                    });
                });
            });

        ///////////////////
        // Central Panel //
        ///////////////////
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut added_nodes = vec![];
            let show_close = self.current_tab_counter != 1;
            let show_add = self.current_tab_counter < 5;
            egui_dock::DockArea::new(&mut self.tree)
                .style({
                    let mut style = Style::from_egui(ctx.style().as_ref());
                    style.tabs.fill_tab_bar = true;
                    style.buttons.add_tab_bg_fill = THEME.sky;
                    style.tabs.text_color_focused = THEME.blue;
                    style
                })
                .show_close_buttons(show_close)
                .show_add_buttons(show_add)
                .show_inside(ui, &mut Tabs {
                    channels: &mut self.channels,
                    main_context: ctx.clone(),
                    available_ports: &mut self.available_ports,
                    already_connected_ports: &mut self.already_connected_ports,
                    selected_port: &mut self.selected_port,
                    motor_name: &mut self.motor_name,
                    motor: &mut self.motor,
                    durations: &mut self.durations,
                    promise_serial_connect: &mut self.promise_serial_connect,
                    added_nodes: &mut added_nodes,
                    added_tabs: &mut self.added_tabs,
                    current_tab_counter: &mut self.current_tab_counter,
                    absolute_tab_counter: &mut self.absolute_tab_counter,
                    can_tab_close: &mut self.can_tab_close,
                });
            added_nodes.drain(..).for_each(|node| {
                self.tree.set_focused_node(node);
                self.tree.push_to_focused_leaf(*self.added_tabs.last().unwrap());
            });
        });
    }

    fn on_close_event(&mut self) -> bool {
        let any_connected = self.motor.iter().any(|motor| motor.get_is_connected());
        if any_connected {
            self.windows_state.is_confirmation_dialog_open = true;
        } else {
            self.allowed_to_close = true;
        }
        self.allowed_to_close
    }
}
