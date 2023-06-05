use catppuccin_egui::{LATTE, Theme};
use egui::{Color32, FontFamily, FontId, RichText, Sense};
use egui::TextStyle::{Body, Button, Heading, Monospace, Small};
use egui_toast::ToastKind;

use crate::utils::helpers::send_toast;
use crate::utils::structs::{Channels, FontAndButtonSize, Message, WindowsState};

pub const FONT_BUTTON_SIZE: FontAndButtonSize = FontAndButtonSize {
    font_table: 13.0,
    font_default: 14.0,
    font_large: 20.0,
    button_top_panel: egui::vec2(75.0, 20.0),
    button_default: egui::vec2(100.0, 20.0),
};

pub const THEME: Theme = Theme {
    base: Color32::from_rgb(249, 251, 255),
    ..LATTE
};

pub struct TemplateApp {
    app_version: String,
    is_first_frame: bool,
    toast_position_x: f32,
    toast_position_y: f32,
    message: Message,
    height: f32,
    width: f32,
    channels: Channels,
    windows_state: WindowsState,
    info_message_is_waiting: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            is_first_frame: true,
            toast_position_x: 0.0,
            toast_position_y: 0.0,
            message: Message::default(),
            height: 0.0,
            width: 0.0,
            channels: Channels::default(),
            windows_state: WindowsState::default(),
            info_message_is_waiting: false,
        }
    }
}

impl TemplateApp {
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
        // send_toast(&self.channels.toast_tx, ToastKind::Info, "Welcome to the TemplateApp", 5);
        // Setup channels for Message.
        let (message_tx, message_rx) = std::sync::mpsc::channel();
        self.channels.message_tx = Some(message_tx);
        self.channels.message_rx = Some(message_rx);
        self.is_first_frame = false;
    }

    /// Message handler.
    fn message_handler(&mut self, message: &Message) {
        match message.kind {
            ToastKind::Info => {
                self.info_message_is_waiting = message.is_waiting;
                if !message.is_waiting {
                    send_toast(&self.channels.toast_tx, ToastKind::Info, &message.message, message.duration);
                }
            }
            ToastKind::Error => {
                send_toast(&self.channels.toast_tx, ToastKind::Error, &message.message, message.duration);
            }
            ToastKind::Warning => {
                send_toast(&self.channels.toast_tx, ToastKind::Warning, &message.message, message.duration);
            }
            ToastKind::Success => {
                send_toast(&self.channels.toast_tx, ToastKind::Success, &message.message, message.duration);
            }
            _ => {}
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // Examples of how to create different panels and windows.
        // Pick whichever suits you.
        // Tip: a good default choice is to just keep the `CentralPanel`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("Side Panel");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(label);
            });

            ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                *value += 1.0;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("powered by ");
                    ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                    ui.label(" and ");
                    ui.hyperlink_to(
                        "eframe",
                        "https://github.com/emilk/egui/tree/master/crates/eframe",
                    );
                    ui.label(".");
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            ui.heading("eframe template");
            ui.hyperlink("https://github.com/emilk/eframe_template");
            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));
            egui::warn_if_debug_build(ui);
        });

        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally choose either panels OR windows.");
            });
        }
    }
}
