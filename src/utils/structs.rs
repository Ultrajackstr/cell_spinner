use std::sync::mpsc::{Receiver, Sender};

use anyhow::Error;
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
    pub(crate) error: Option<Error>,
    pub(crate) origin: Option<String>,
    pub(crate) duration: u64,
    pub(crate) is_waiting: bool,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            kind: ToastKind::Info,
            message: String::new(),
            error: None,
            origin: None,
            duration: 0,
            is_waiting: false,
        }
    }
}

impl Message {
    pub(crate) fn new(kind: ToastKind, message: &str, error: Option<Error>, origin: Option<String>, duration: u64, is_waiting: bool) -> Self {
        Self {
            kind,
            message: message.into(),
            error,
            origin,
            duration,
            is_waiting,
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