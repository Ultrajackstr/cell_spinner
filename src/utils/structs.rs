use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use anyhow::Error;
use egui_toast::{Toast, ToastKind};

use crate::utils::enums::StepperState;

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
    pub is_confirmation_dialog_open: bool,
    pub is_error_log_open: bool,
}

#[derive(Default)]
pub struct DurationHelper {
    pub days: u64,
    pub hours: u64,
    pub minutes: u64,
    pub seconds: u64,
    pub milliseconds: u64,
}

impl DurationHelper {
    pub fn convert_to_milliseconds(&self) -> u64 {
        self.days * 24 * 60 * 60 * 1000 + self.hours * 60 * 60 * 1000 + self.minutes * 60 * 1000 + self.seconds * 1000 + self.milliseconds
    }

    pub fn convert_from_milliseconds(&mut self, milliseconds: u64) {
        self.days = milliseconds / (24 * 60 * 60 * 1000);
        self.hours = (milliseconds - self.days * 24 * 60 * 60 * 1000) / (60 * 60 * 1000);
        self.minutes = (milliseconds - self.days * 24 * 60 * 60 * 1000 - self.hours * 60 * 60 * 1000) / (60 * 1000);
        self.seconds = (milliseconds - self.days * 24 * 60 * 60 * 1000 - self.hours * 60 * 60 * 1000 - self.minutes * 60 * 1000) / 1000;
        self.milliseconds = milliseconds - self.days * 24 * 60 * 60 * 1000 - self.hours * 60 * 60 * 1000 - self.minutes * 60 * 1000 - self.seconds * 1000;
    }
}

#[derive(Default)]
pub struct Durations {
    pub duration_of_one_direction_cycle_rotation: DurationHelper,
    pub pause_before_direction_change_rotation: DurationHelper,
    pub rotation_duration: DurationHelper,
    pub pause_pre_agitation: DurationHelper,
    pub duration_of_one_direction_cycle_agitation: DurationHelper,
    pub pause_before_direction_change_agitation: DurationHelper,
    pub agitation_duration: DurationHelper,
    pub pause_post_agitation: DurationHelper,
    pub global_duration: DurationHelper,
}


#[derive(Default)]
pub struct TimersAndPhases {
    pub motor_start_time: Option<Instant>,
    pub motor_stop_time_ms: Option<u64>,
    pub phase: StepperState,
    pub phase_start_time: Option<Instant>,
    pub global_phase: StepperState,
    pub global_phase_start_time: Option<Instant>,
}

impl TimersAndPhases {
    pub fn get_elapsed_time_since_motor_start_as_millis(&self) -> u64 {
        match self.motor_start_time {
            Some(start_time) => start_time.elapsed().as_millis() as u64,
            None => 0,
        }
    }

    pub fn get_elapsed_time_since_global_phase_start_as_millis(&self) -> u64 {
        match self.global_phase_start_time {
            Some(start_time) => start_time.elapsed().as_millis() as u64,
            None => 0,
        }
    }

    pub fn get_elapsed_time_since_phase_start_as_millis(&self) -> u64 {
        match self.phase_start_time {
            Some(start_time) => start_time.elapsed().as_millis() as u64,
            None => 0,
        }
    }

    pub fn set_stop_time_motor_stopped(&mut self) {
        self.motor_stop_time_ms = Some(self.get_elapsed_time_since_motor_start_as_millis());
    }
}

#[derive(Default)]
pub struct StepsCycle {
    pub steps_per_direction_cycle_rotation: Arc<AtomicU64>,
    pub steps_per_direction_cycle_agitation: Arc<AtomicU64>,
}