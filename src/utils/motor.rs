use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Instant;

use anyhow::{anyhow, bail, Error};
use egui_toast::ToastKind;
use fugit::TimerInstantU64;
use parking_lot::Mutex;

use crate::app::{MAX_ACCELERATION, MAX_DURATION_MS, MAX_POINTS_GRAPHS};
use crate::utils::enums::StepperState;
use crate::utils::frame_history::FrameHistory;
use crate::utils::graph::Graph;
use crate::utils::protocols::Protocol;
use crate::utils::serial::Serial;
use crate::utils::structs::{Message, StepsCycle, TimersAndPhases};

pub struct Motor {
    pub name: String,
    pub is_running: Arc<AtomicBool>,
    pub protocol: Protocol,
    pub serial: Serial,
    pub graph: Graph,
    pub timers_and_phases: Arc<Mutex<TimersAndPhases>>,
    pub steps_per_cycle: StepsCycle,
    pub frame_hisory: FrameHistory,
    pub angle_rotation: f32,
    pub angle_agitation: f32,
}

impl Default for Motor {
    fn default() -> Self {
        Self {
            name: String::from(""),
            is_running: Arc::new(AtomicBool::new(false)),
            protocol: Protocol::default(),
            serial: Serial::default(),
            graph: Graph::default(),
            timers_and_phases: Arc::new(Mutex::new(TimersAndPhases::default())),
            steps_per_cycle: StepsCycle::default(),
            frame_hisory: FrameHistory::default(),
            angle_rotation: 0.0,
            angle_agitation: 0.0,
        }
    }
}

impl Motor {
    pub fn new(serial_port: String, motor_name: String, already_connected_ports: Arc<Mutex<Vec<String>>>) -> Result<Self, Error> {
        let serial = Serial::new(&serial_port, already_connected_ports)?;
        Ok(Self {
            name: motor_name,
            is_running: Arc::new(AtomicBool::new(false)),
            protocol: Protocol::default(),
            serial,
            graph: Graph::default(),
            timers_and_phases: Arc::new(Mutex::new(TimersAndPhases::default())),
            steps_per_cycle: StepsCycle::default(),
            frame_hisory: FrameHistory::default(),
            angle_rotation: 0.0,
            angle_agitation: 0.0,
        })
    }

    pub fn new_with_already_loaded_protocol(serial_port: String, motor_name: String, already_connected_ports: Arc<Mutex<Vec<String>>>, protocol: Protocol, graph: Graph, steps_per_cycle: StepsCycle) -> Result<Self, Error> {
        let mut motor = Self::new(serial_port, motor_name, already_connected_ports)?;
        motor.protocol = protocol;
        motor.graph = graph;
        motor.steps_per_cycle = steps_per_cycle;
        Ok(motor)
    }

    pub fn get_is_connected(&self) -> bool {
        self.serial.get_is_connected()
    }

    pub fn get_is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn disconnect(&mut self, message_tx: Option<Sender<Message>>) {
        if self.is_running.load(Ordering::SeqCst) {
            self.stop_motor(message_tx.clone());
        }
        self.serial.disconnect();
        let message = Message::new(ToastKind::Info, &format!("Disconnected from {}", self.serial.port_name), None, Some(self.name.clone()), 3, false);
        if let Some(message_tx) = message_tx {
            message_tx.send(message).unwrap();
        }
        self.serial = Serial::default();
    }

    pub fn start_motor(&mut self, message_tx: Option<Sender<Message>>) {
        let min_rotation_duration = self.protocol.rotation.get_min_duration();
        let min_agitation_duration = self.protocol.agitation.get_min_duration();
        if min_rotation_duration == 0 {
            self.protocol.rotation_duration_ms = 0;
        }
        if min_agitation_duration == 0 {
            self.protocol.agitation_duration_ms = 0;
        }
        if self.protocol.rotation_duration_ms == 0 {
            self.protocol.pause_pre_agitation_ms = 0;
        }
        if self.protocol.agitation_duration_ms == 0 {
            self.protocol.pause_post_agitation_ms = 0;
        }
        let duration_without_pause = self.protocol.get_duration_without_pause();
        if duration_without_pause == 0 {
            self.protocol.global_duration_ms = 0;
            let message = Message::new(ToastKind::Error, "The duration of the protocol is 0. Please check the durations.", Some(anyhow!("0 duration")), Some(self.name.clone()), 3, false);
            if let Some(message_tx) = message_tx {
                message_tx.send(message).unwrap();
            }
            return;
        }
        self.is_running.store(true, Ordering::SeqCst);
        self.timers_and_phases.lock().global_start_time = Some(Instant::now());
        self.timers_and_phases.lock().global_stop_time_ms = None;
        self.timers_and_phases.lock().rotation_direction = self.protocol.rotation.direction;
        self.timers_and_phases.lock().agitation_direction = self.protocol.agitation.direction;
        self.angle_rotation = 0.0;
        self.angle_agitation = 0.0;
        self.serial.listen_to_serial_port(self.name.clone(), &self.is_running, &self.timers_and_phases, message_tx);
        self.serial.send_bytes(&self.protocol.protocol_as_bytes());
        tracing::info!("Motor {} started.", self.name);
        tracing::info!("{}", self.protocol);
    }

    pub fn stop_motor(&mut self, message_tx: Option<Sender<Message>>) {
        self.is_running.store(false, Ordering::SeqCst);
        self.serial.send_bytes(b"stop");
        self.timers_and_phases.lock().set_global_stop_time_stopped();
        self.timers_and_phases.lock().sub_phase_start_time = None;
        self.timers_and_phases.lock().main_phase_start_time = None;
        self.timers_and_phases.lock().sub_phase = StepperState::default();
        self.timers_and_phases.lock().main_phase = StepperState::default();
        let message = Message::new(ToastKind::Info, &format!("{} has been stopped.", self.name), None, None, 3, false);
        if let Some(message_tx) = message_tx {
            message_tx.send(message).unwrap();
        }
    }

    pub fn get_revolutions_per_rotation_cycle(&self) -> f64 {
        self.steps_per_cycle.steps_per_direction_cycle_rotation.load(Ordering::SeqCst) as f64 / (self.protocol.rotation.step_mode.get_multiplier() as f64 * 200.0)
    }

    pub fn get_revolutions_per_agitation_cycle(&self) -> f64 {
        self.steps_per_cycle.steps_per_direction_cycle_agitation.load(Ordering::SeqCst) as f64 / (self.protocol.agitation.step_mode.get_multiplier() as f64 * 200.0)
    }

    pub fn import_protocol(&mut self, protocol: Protocol) -> Result<(), Error> {
        // Check if the protocol is valid
        if protocol.rotation.acceleration == 0 || protocol.agitation.acceleration == 0 {
            bail!("The acceleration of the rotation or agitation is 0");
        }
        if protocol.rotation.acceleration > MAX_ACCELERATION || protocol.agitation.acceleration > MAX_ACCELERATION {
            bail!("The acceleration of the rotation or agitation is too high");
        }
        if protocol.rotation.rpm > protocol.rotation.max_rpm_for_stepmode() || protocol.agitation.rpm > protocol.agitation.max_rpm_for_stepmode() {
            bail!("The rpm of the rotation or agitation is higher than the max rpm");
        }
        if protocol.get_duration_without_pause() == 0 {
            self.protocol.global_duration_ms = 0;
        }
        if protocol.rotation.duration_of_one_direction_cycle_ms > MAX_DURATION_MS || protocol.agitation.duration_of_one_direction_cycle_ms > MAX_DURATION_MS
            || protocol.rotation.pause_before_direction_change_ms > MAX_DURATION_MS || protocol.agitation.pause_before_direction_change_ms > MAX_DURATION_MS
            || protocol.global_duration_ms > MAX_DURATION_MS || protocol.rotation_duration_ms > MAX_DURATION_MS || protocol.agitation_duration_ms > MAX_DURATION_MS
            || protocol.pause_pre_agitation_ms > MAX_DURATION_MS || protocol.pause_post_agitation_ms > MAX_DURATION_MS
        {
            bail!("Some duration is too high");
        }

        self.protocol = protocol;
        Ok(())
    }

    pub fn generate_graph_rotation(&self) {
        let points_rotation = self.graph.rotation_points_sec_rpm.clone();
        let rotation = self.protocol.rotation;
        let index_thread = self.graph.rotation_thread_index.clone();
        index_thread.fetch_add(1, Ordering::SeqCst);
        let index_thead_initial = index_thread.load(Ordering::SeqCst);
        let steps_rotation = self.steps_per_cycle.steps_per_direction_cycle_rotation.clone();
        // Rotation
        thread::spawn(move || {
            points_rotation.lock().clear();
            let mut stepgen = rotation.create_stepgen();
            let duration_ms = rotation.duration_of_one_direction_cycle_ms;
            let point_threshold_us = duration_ms * 1000 / 100; // 100 points per cycle while rpm is constant
            let mut delay_acc_us = 0;
            let mut rpm_for_graph;
            let mut last_rpm = 0.0;
            let mut acc_us_for_points = 0;
            let now_ms = |prev_delay_us: u64| -> TimerInstantU64<1000> {
                TimerInstantU64::from_ticks((prev_delay_us as f64 * 0.001) as u64)
            };
            while let Some(delay) = stepgen.next_delay(Some(now_ms(delay_acc_us))) {
                let is_max_points = points_rotation.lock().len() > MAX_POINTS_GRAPHS;
                rpm_for_graph = 300_000.0 / rotation.step_mode.get_multiplier() as f64 / (delay + 1) as f64;
                if index_thead_initial != index_thread.load(Ordering::SeqCst) {
                    return;
                }
                if rpm_for_graph != last_rpm && !is_max_points {
                    points_rotation.lock().push([delay_acc_us as f64 * 0.000001, rpm_for_graph]);
                    last_rpm = rpm_for_graph;
                } else if acc_us_for_points >= point_threshold_us && !is_max_points {
                    points_rotation.lock().push([delay_acc_us as f64 * 0.000001, rpm_for_graph]);
                    acc_us_for_points = 0;
                }
                delay_acc_us += delay;
                acc_us_for_points += delay;
                steps_rotation.store(stepgen.get_current_step(), Ordering::SeqCst);
            }
        });
    }

    pub fn generate_graph_agitation(&self) {
        let points_agitation = self.graph.agitation_points_sec_rpm.clone();
        let agitation = self.protocol.agitation;
        let index_thread = self.graph.agitation_thread_index.clone();
        index_thread.fetch_add(1, Ordering::SeqCst);
        let index_thead_initial = index_thread.load(Ordering::SeqCst);
        let steps_agitation = self.steps_per_cycle.steps_per_direction_cycle_agitation.clone();
        // Agitation
        thread::spawn(move || {
            points_agitation.lock().clear();
            let mut stepgen = agitation.create_stepgen();
            let duration_ms = agitation.duration_of_one_direction_cycle_ms;
            let point_threshold_us = duration_ms * 1000 / 100; // 100 points per cycle while rpm is constant
            let mut delay_acc_us = 0;
            let mut rpm_for_graph;
            let mut last_rpm = 0.0;
            let mut acc_us_for_points = 0;
            let now_ms = |prev_delay_us: u64| -> TimerInstantU64<1000> {
                TimerInstantU64::from_ticks((prev_delay_us as f64 * 0.001) as u64)
            };
            while let Some(delay_us) = stepgen.next_delay(Some(now_ms(delay_acc_us))) {
                let is_max_points = points_agitation.lock().len() > MAX_POINTS_GRAPHS;
                rpm_for_graph = 300_000.0 / agitation.step_mode.get_multiplier() as f64 / (delay_us + 1) as f64;
                if index_thead_initial != index_thread.load(Ordering::SeqCst) {
                    return;
                }
                if rpm_for_graph != last_rpm && !is_max_points {
                    points_agitation.lock().push([delay_acc_us as f64 * 0.000001, rpm_for_graph]);
                    last_rpm = rpm_for_graph;
                } else if acc_us_for_points >= point_threshold_us && !is_max_points {
                    points_agitation.lock().push([delay_acc_us as f64 * 0.000001, rpm_for_graph]);
                    acc_us_for_points = 0;
                }
                delay_acc_us += delay_us;
                acc_us_for_points += delay_us;
                steps_agitation.store(stepgen.get_current_step(), Ordering::SeqCst);
            }
        });
    }
}