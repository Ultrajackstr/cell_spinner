use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Error;
use egui_toast::{Toast, ToastKind};

use crate::app::MAX_DURATION_MS;
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

    pub fn set_origin(&mut self, origin: &str) {
        self.origin = Some(origin.into());
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

    pub fn get_mut_days(&mut self) -> &mut u64 {
        &mut self.days
    }

    pub fn get_mut_hours(&mut self) -> &mut u64 {
        &mut self.hours
    }

    pub fn get_mut_minutes(&mut self) -> &mut u64 {
        &mut self.minutes
    }

    pub fn get_mut_seconds(&mut self) -> &mut u64 {
        &mut self.seconds
    }

    pub fn get_mut_milliseconds(&mut self) -> &mut u64 {
        &mut self.milliseconds
    }

    pub fn check_if_duration_is_greater_than_max_duration(&self) -> bool {
        self.convert_to_milliseconds() > MAX_DURATION_MS
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

impl Durations {
    pub fn get_mut_rotation_cycle_duration(&mut self) -> &mut DurationHelper {
        &mut self.duration_of_one_direction_cycle_rotation
    }

    pub fn get_mut_pause_between_rotation(&mut self) -> &mut DurationHelper {
        &mut self.pause_before_direction_change_rotation
    }

    pub fn get_mut_rotation_global_duration(&mut self) -> &mut DurationHelper {
        &mut self.rotation_duration
    }

    pub fn get_mut_pause_pre_agitation(&mut self) -> &mut DurationHelper {
        &mut self.pause_pre_agitation
    }

    pub fn get_mut_agitation_cycle_duration(&mut self) -> &mut DurationHelper {
        &mut self.duration_of_one_direction_cycle_agitation
    }

    pub fn get_mut_pause_between_agitation(&mut self) -> &mut DurationHelper {
        &mut self.pause_before_direction_change_agitation
    }

    pub fn get_mut_agitation_global_duration(&mut self) -> &mut DurationHelper {
        &mut self.agitation_duration
    }

    pub fn get_mut_pause_post_agitation(&mut self) -> &mut DurationHelper {
        &mut self.pause_post_agitation
    }

    pub fn get_mut_global_duration(&mut self) -> &mut DurationHelper {
        &mut self.global_duration
    }

    pub fn get_rotation_cycle_duration(&self) -> &DurationHelper {
        &self.duration_of_one_direction_cycle_rotation
    }

    pub fn get_pause_between_rotation(&self) -> &DurationHelper {
        &self.pause_before_direction_change_rotation
    }

    pub fn get_rotation_global_duration(&self) -> &DurationHelper {
        &self.rotation_duration
    }

    pub fn get_pause_pre_agitation(&self) -> &DurationHelper {
        &self.pause_pre_agitation
    }

    pub fn get_agitation_cycle_duration(&self) -> &DurationHelper {
        &self.duration_of_one_direction_cycle_agitation
    }

    pub fn get_pause_between_agitation(&self) -> &DurationHelper {
        &self.pause_before_direction_change_agitation
    }

    pub fn get_pause_post_agitation(&self) -> &DurationHelper {
        &self.pause_post_agitation
    }

    pub fn get_agitation_global_duration(&self) -> &DurationHelper {
        &self.agitation_duration
    }

    pub fn get_global_duration(&self) -> &DurationHelper {
        &self.global_duration
    }
}

#[derive(Default)]
pub struct TimersAndPhases {
    start_time: Option<Instant>,
    stop_time_ms: Option<u64>,
    phase: StepperState,
    phase_start_time: Option<Instant>,
    global_phase: StepperState,
    global_phase_start_time: Option<Instant>,
}

impl TimersAndPhases {
    pub fn set_start_time(&mut self, instant: Instant) {
        self.start_time = Some(instant);
    }

    pub fn set_stop_time_ms(&mut self, stop_time: Option<u64>) {
        self.stop_time_ms = stop_time;
    }

    pub fn set_phase(&mut self, phase: StepperState) {
        self.phase = phase;
    }

    pub fn set_phase_start_time(&mut self, instant: Option<Instant>) {
        self.phase_start_time = instant;
    }

    pub fn set_global_phase(&mut self, phase: StepperState) {
        self.global_phase = phase;
    }

    pub fn set_global_phase_start_time(&mut self, instant: Option<Instant>) {
        self.global_phase_start_time = instant;
    }

    pub fn get_elapsed_time_since_motor_start_as_millis(&self) -> u64 {
        match self.start_time {
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

    pub fn get_global_phase_string(&self) -> String {
        self.global_phase.to_string()
    }

    pub fn get_phase_string(&self) -> String {
        self.phase.to_string()
    }

    pub fn get_stop_time_ms(&self) -> Option<u64> {
        self.stop_time_ms
    }

    pub fn set_stop_time_motor_stopped(&mut self) {
        self.stop_time_ms = Some(self.get_elapsed_time_since_motor_start_as_millis());
    }
}