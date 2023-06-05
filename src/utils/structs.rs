use std::sync::mpsc::{Receiver, Sender};

use anyhow::Error;
use egui_toast::{Toast, ToastKind};

use crate::utils::enums::{Direction, StepMode128};

pub struct FontAndButtonSize {
    pub font_table: f32,
    pub font_default: f32,
    pub font_large: f32,
    pub button_top_panel: egui::Vec2,
    pub button_default: egui::Vec2,
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

pub struct Message {
    pub kind: ToastKind,
    pub message: String,
    pub error: Option<Error>,
    pub origin: Option<String>,
    pub duration: u64,
    pub is_waiting: bool,
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
    pub fn new(kind: ToastKind, message: &str, error: Option<Error>, origin: Option<String>, duration: u64, is_waiting: bool) -> Self {
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
pub struct Channels {
    pub toast_tx: Option<Sender<Toast>>,
    pub toast_rx: Option<Receiver<Toast>>,
    pub message_tx: Option<Sender<Message>>,
    pub message_rx: Option<Receiver<Message>>,
}

#[derive(Default)]
pub struct WindowsState {
    pub is_about_open: bool,
    pub is_help_open: bool,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Rotation {
    pub rpm: u32,
    pub accel: u32,
    pub step_mode: StepMode128,
    pub duration_of_one_direction_cycle_ms: u64,
    pub steps_for_one_direction_cycle: u64,
    pub direction: Direction,
    pub pause_before_direction_change_ms: u64,
}

impl Rotation {
    pub fn new(rpm: u32, accel: u32, step_mode: StepMode128, duration_of_one_direction_cycle_ms: u64, steps_for_one_direction_cycle: u64, direction: Direction, pause_before_direction_change_ms: u64) -> Self {
        Self {
            rpm,
            accel,
            step_mode,
            duration_of_one_direction_cycle_ms,
            steps_for_one_direction_cycle,
            direction,
            pause_before_direction_change_ms,
        }
    }

    /// Rotation to bytes for serial communication
    pub fn to_bytes(&self) -> [u8; 34] {
        let mut bytes = [0u8; 34];
        bytes[0..4].copy_from_slice(&self.rpm.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.accel.to_le_bytes());
        bytes[8..9].copy_from_slice(&self.step_mode.to_byte().to_le_bytes());
        bytes[9..17].copy_from_slice(&self.duration_of_one_direction_cycle_ms.to_le_bytes());
        bytes[17..25].copy_from_slice(&self.steps_for_one_direction_cycle.to_le_bytes());
        bytes[25..26].copy_from_slice(&self.direction.to_byte().to_le_bytes());
        bytes[26..34].copy_from_slice(&self.pause_before_direction_change_ms.to_le_bytes());
        bytes
    }
}