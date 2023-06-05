use std::sync::mpsc::{Receiver, Sender};

use egui_toast::{Toast, ToastKind};

pub(crate) struct FontAndButtonSize {
    pub(crate) font_table: f32,
    pub(crate) font_default: f32,
    pub(crate) font_large: f32,
    pub(crate) button_top_panel: egui::Vec2,
    pub(crate) button_default: egui::Vec2,
}

impl Default for FontAndButtonSize {
    fn default() -> Self {
        Self {
            font_table: 14.0,
            font_default: 16.0,
            font_large: 20.0,
            button_top_panel: egui::Vec2::new(100.0, 30.0),
            button_default: egui::Vec2::new(100.0, 30.0),
        }
    }
}

pub(crate) struct Message {
    pub(crate) kind: ToastKind,
    pub(crate) message: String,
    pub(crate) origin: String,
    pub(crate) duration: u64,
    pub(crate) is_waiting: bool,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            kind: ToastKind::Info,
            message: String::new(),
            origin: String::new(),
            duration: 0,
            is_waiting: false,
        }
    }
}

impl Message {
    pub(crate) fn new_info(message: String, origin: String, duration: u64, is_waiting: bool) -> Self {
        Self {
            kind: ToastKind::Info,
            message,
            origin,
            duration,
            is_waiting,
        }
    }

    pub(crate) fn new_error(message: String, origin: String, duration: u64) -> Self {
        Self {
            kind: ToastKind::Error,
            message,
            origin,
            duration,
            is_waiting: false,
        }
    }

    pub(crate) fn new_warning(message: String, origin: String, duration: u64) -> Self {
        Self {
            kind: ToastKind::Warning,
            message,
            origin,
            duration,
            is_waiting: false,
        }
    }
}

#[derive(Default)]
pub(crate) struct Channels {
    pub(crate) toast_tx: Option<Sender<Toast>>,
    pub(crate) toast_rx: Option<Receiver<Toast>>,
    pub(crate) message_tx: Option<Sender<Message>>,
    pub(crate) message_rx: Option<Receiver<Message>>,
}

#[derive(Default)]
pub(crate) struct WindowsState {
    pub(crate) is_about_open: bool,
    pub(crate) is_help_open: bool,
}