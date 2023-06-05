use std::sync::{Arc, Mutex};
use std::time::Duration;

use catppuccin_egui::{LATTE, Theme};
use chrono::Local;
use dashmap::DashMap;
use egui::{Color32, FontFamily, FontId, RichText, Sense};
use egui::TextStyle::{Body, Button, Heading, Monospace, Small};
use egui_dock::{Style, Tree};
use egui_toast::{Toast, ToastKind, Toasts};

use crate::utils::graph::Graph;
use crate::utils::helpers::send_toast;
use crate::utils::motor::Motor;
use crate::utils::protocols::Protocol;
use crate::utils::serial::Serial;
use crate::utils::structs::{Channels, FontAndButtonSize, Message, WindowsState};

pub const FONT_BUTTON_SIZE: FontAndButtonSize = FontAndButtonSize {
    font_table: 13.0,
    font_default: 14.0,
    font_large: 20.0,
    button_top_panel: egui::vec2(75.0, 20.0),
    button_default: egui::vec2(100.0, 20.0),
};

pub const THREAD_SLEEP: u64 = 10;
pub const MAX_ACCELERATION: u32 = 20_000;
pub const MIN_ACCELERATION: u32 = 1;
pub const MIN_RPM_FULL: u32 = 1;
pub const MAX_RPM: u32 = 5_000;
pub const MAX_STEPS: u32 = 4_000_000_000;
pub const MAX_POINTS_GRAPHS: usize = 250_000;
pub const BYTES: usize = 110;
pub const THEME: Theme = Theme {
    base: Color32::from_rgb(249, 251, 255),
    ..LATTE
};

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
    // Promises
    promise_serial_connect: Arc<DashMap<usize, Option<()>>>,
    // Serial
    already_connected_ports: Arc<Mutex<Vec<String>>>,
    // Motor
    motor: Arc<DashMap<usize, Motor>>,
    // Tabs
    current_tab_counter: usize,
    tree: Tree<usize>,
    absolute_tab_counter: usize,
    added_tabs: Vec<usize>,
    can_tab_close: bool,

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
            promise_serial_connect: Arc::new(Default::default()),
            already_connected_ports: Arc::new(Mutex::new(vec![])),
            current_tab_counter: 0,
            tree: Default::default(),
            absolute_tab_counter: 0,
            added_tabs: vec![],
            can_tab_close: false,
            motor: Arc::new(Default::default()),
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
        let message: Message = Message::new(ToastKind::Info, "Welcome to TemplateApp !!", None, None, 3, false);
        self.message_handler(message);
        // Setup channels for Message.
        let (message_tx, message_rx) = std::sync::mpsc::channel();
        self.channels.message_tx = Some(message_tx);
        self.channels.message_rx = Some(message_rx);
        self.init_tab(1);
        self.added_tabs.push(1);
        self.is_first_frame = false;
    }

    /// Message handler.
    fn message_handler(&mut self, message: Message) {
        match message.kind {
            ToastKind::Info => {
                self.info_message_is_waiting = message.is_waiting;
                let text = if let Some(origin) = message.origin {
                    format!("{}: {}", origin, message.message)
                } else {
                    message.message.to_string()
                };
                if !message.is_waiting {
                    send_toast(&self.channels.toast_tx, ToastKind::Info, text, message.duration);
                } else {
                    self.info_message = text;
                }
            }
            ToastKind::Error => {
                if message.error.is_none() {
                    panic!("Error message without error");
                }
                let text = if let Some(origin) = message.origin {
                    format!("{} 💠 {}: {} {:?}", Local::now().format("%d-%m-%Y %H:%M:%S"), origin, message.message, message.error.unwrap())
                } else {
                    format!("{} 💠 {} {:?}", Local::now().format("%d-%m-%Y %H:%M:%S"), message.message, message.error.unwrap())
                };
                tracing::error!(text);
                self.error_log.insert(0, text.clone());
                send_toast(&self.channels.toast_tx, ToastKind::Error, text, message.duration);
            }
            ToastKind::Warning => {
                send_toast(&self.channels.toast_tx, ToastKind::Warning, message.message, message.duration);
            }
            ToastKind::Success => {
                send_toast(&self.channels.toast_tx, ToastKind::Success, message.message, message.duration);
            }
            _ => {}
        }
    }

    /// Init tab
    fn init_tab(&mut self, tab: usize) {
        self.graph.insert(tab, Graph::default());
        self.serial.insert(tab, Serial::default());
        self.protocol.insert(tab, Protocol::default());
    }
}

impl eframe::App for CellSpinner {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
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
        self.toast_position_x = 0.0;
        self.toast_position_y = self.height - 30.5;
        let mut toasts = Toasts::new()
            .anchor((self.toast_position_x, self.toast_position_y))
            .direction(egui::Direction::BottomUp)
            .align_to_end(false)
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

        ////////////////////////////////////////////////////////////////////////////////
        ////////////////////////////////////////////////////////////////////////////////
        ///////////////
        // Top Panel //
        ///////////////
        egui::TopBottomPanel::top("top_panel")
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::ScrollArea::horizontal().id_source("Top_scroll_area").show(ui, |ui| {
                        let mut tab = 1;
                        if let Some(active_tab) = self.tree.find_active_focused() {
                            tab = *active_tab.1;
                        };
                        let is_running = *self.is_motor_running.get(&tab).unwrap();
                        let is_any_running = self.is_motor_running.iter().any(|v| *v);
                        // Title
                        let response_heading = ui.add(egui::Label::new(RichText::new("EV Stepper Controller").heading())
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
                                self.import_configuration(&tab);
                            } else if import_response.secondary_clicked() {
                                self.import_for_all_motors = true;
                                self.import_configuration(&tab);
                            }
                        });
                        // Sync all motors
                        ui.separator();
                        ui.add_enabled_ui(!is_any_running, |ui| {
                            ui.add(IndicatorButton::new(&mut self.sync_conf_for_all_motors)
                                       .label("Sync config")
                                       .width(FONT_BUTTON_SIZE.button_top_panel.x)
                                       .height(FONT_BUTTON_SIZE.button_top_panel.y),
                            ).on_hover_text("Sync the configuration changes with all the motors.");
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
                .show_inside(ui, &mut TabsData {
                    channels: &mut self.channels,
                    main_context: ctx.clone(),
                    phases_window: &mut self.windows_state.phases,
                    position_phase_window: &mut self.windows_state.phases_position,
                    serial_port: &mut self.serial_port,
                    serial_port_name_for_selection: &mut self.serial_port_name,
                    serial_port_name_connected: &mut self.serial_port_name_connected,
                    available_ports: &mut self.available_ports,
                    promise_available_ports: &mut self.promise_available_ports,
                    promise_rp_pico_connect: &mut self.promise_rp_pico_connect,
                    promise_rp_pico_write: &mut self.promise_rp_pico_write,
                    is_new_repetition: &mut self.is_new_repetition,
                    is_motor_running: &mut self.is_motor_running,
                    run_time: &mut self.run_time,
                    phase_color: &mut self.phase_color,
                    phase_background_color: &mut self.phase_background_color,
                    path_config: &mut self.path_config,
                    index_thread_graph_point_time_control: &mut self.index_thread_graph_point_time_control,
                    added_nodes: &mut added_nodes,
                    added_tabs: &mut self.added_tabs,
                    current_tab_counter: &mut self.current_tab_counter,
                    absolute_tab_counter: &mut self.absolute_tab_counter,
                    can_tab_close: &mut self.can_tab_close,
                    position_control_setup: &mut self.position_control_setup,
                    index_thread_graph_point_position_control: &mut self.index_thread_graph_point_position_control,
                    graphs_data: &mut self.graphs_data,
                    time_control_setup: &mut self.time_control_setup,
                    control_mode: &mut self.control_mode,
                    phase_for_timer: &mut self.phase_for_timer,
                    sync_conf_for_all_motors: &mut self.sync_conf_for_all_motors,
                });
            added_nodes.drain(..).for_each(|node| {
                self.tree.set_focused_node(node);
                self.tree.push_to_focused_leaf(*self.added_tabs.last().unwrap());
            });
        });
    }
}
