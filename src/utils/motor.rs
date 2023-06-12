use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Error};
use egui_toast::ToastKind;
use fugit::TimerInstantU64;

use crate::app::{MAX_ACCELERATION, MAX_DURATION_MS, MAX_POINTS_GRAPHS, THREAD_SLEEP};
use crate::utils::enums::StepperState;
use crate::utils::graph::Graph;
use crate::utils::protocols::Protocol;
use crate::utils::serial::Serial;
use crate::utils::structs::Message;

pub struct Motor {
    name: String,
    is_running: Arc<AtomicBool>,
    start_time: Option<Instant>,
    stop_time_ms: Option<u64>,
    protocol: Protocol,
    serial: Serial,
    graph: Graph,
    pub phase: Arc<Mutex<StepperState>>,
    pub phase_start_time: Arc<Mutex<Option<Instant>>>,
    pub global_phase: Arc<Mutex<StepperState>>,
    pub global_phase_start_time: Arc<Mutex<Option<Instant>>>,
}

impl Default for Motor {
    fn default() -> Self {
        Self {
            name: String::from(""),
            is_running: Arc::new(AtomicBool::new(false)),
            start_time: None,
            stop_time_ms: None,
            protocol: Protocol::default(),
            serial: Serial::default(),
            graph: Graph::default(),
            phase: Arc::new(Mutex::new(StepperState::default())),
            phase_start_time: Arc::new(Mutex::new(None)),
            global_phase: Arc::new(Mutex::new(Default::default())),
            global_phase_start_time: Arc::new(Mutex::new(None)),
        }
    }
}

impl Motor {
    pub fn new(serial_port: String, motor_name: String, already_connected_ports: Arc<Mutex<Vec<String>>>) -> Result<Self, Error> {
        let serial = Serial::new(&serial_port, already_connected_ports)?;
        Ok(Self {
            name: motor_name,
            is_running: Arc::new(AtomicBool::new(false)),
            start_time: None,
            stop_time_ms: None,
            protocol: Protocol::default(),
            serial,
            graph: Graph::default(),
            phase: Arc::new(Mutex::new(StepperState::default())),
            phase_start_time: Arc::new(Mutex::new(None)),
            global_phase: Arc::new(Mutex::new(Default::default())),
            global_phase_start_time: Arc::new(Mutex::new(None)),
        })
    }

    pub fn new_with_protocol_and_graph(serial_port: String, motor_name: String, already_connected_ports: Arc<Mutex<Vec<String>>>, protocol: Protocol, graph: Graph) -> Result<Self, Error> {
        let mut motor = Self::new(serial_port, motor_name, already_connected_ports)?;
        motor.set_protocol(protocol);
        motor.set_graph(graph);
        Ok(motor)
    }

    pub fn get_serial(&self) -> &Serial {
        &self.serial
    }

    pub fn get_is_connected(&self) -> bool {
        self.serial.get_is_connected()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn get_name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    pub fn set_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
    }

    pub fn get_protocol(&self) -> &Protocol {
        &self.protocol
    }

    pub fn get_protocol_mut(&mut self) -> &mut Protocol {
        &mut self.protocol
    }

    pub fn get_is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    pub fn get_graph(&self) -> &Graph {
        &self.graph
    }

    pub fn set_graph(&mut self, graph: Graph) {
        self.graph = graph;
    }

    pub fn disconnect(&mut self) {
        self.serial.disconnect();
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
        self.is_running.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());
        self.stop_time_ms = None;
        self.serial.listen_to_serial_port(self.name.clone(), &self.is_running, &self.global_phase, &self.global_phase_start_time, &self.phase, &self.phase_start_time, message_tx);
        self.serial.send_bytes(self.protocol.bytes_vec_to_send());
    }

    pub fn stop_motor(&mut self) {
        self.serial.send_bytes(vec![b'x']);
        self.is_running.store(false, Ordering::Relaxed);
        self.stop_time_ms = Some(self.get_elapsed_time_since_motor_start_as_millis());
        self.phase_start_time = Arc::new(Mutex::new(None));
        self.phase = Arc::new(Mutex::new(StepperState::default()));
        self.global_phase_start_time = Arc::new(Mutex::new(None));
        self.global_phase = Arc::new(Mutex::new(Default::default()));
    }

    pub fn get_stop_time_ms(&self) -> Option<u64> {
        self.stop_time_ms
    }

    pub fn get_elapsed_time_in_current_phase_as_millis(&self) -> u64 {
        if let Some(start_time) = *self.phase_start_time.lock().unwrap() {
            start_time.elapsed().as_millis() as u64
        } else {
            0
        }
    }

    pub fn get_current_phase(&self) -> String {
        self.phase.lock().unwrap().to_string()
    }

    pub fn get_elapsed_time_since_motor_start_as_millis(&self) -> u64 {
        if let Some(start_time) = self.start_time {
            start_time.elapsed().as_millis() as u64
        } else {
            0
        }
    }

    pub fn get_elapsed_time_since_global_phase_start_as_millis(&self) -> u64 {
        if let Some(start_time) = *self.global_phase_start_time.lock().unwrap() {
            start_time.elapsed().as_millis() as u64
        } else {
            0
        }
    }

    pub fn get_global_phase(&self) -> String {
        self.global_phase.lock().unwrap().to_string()
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
        let points_rotation = self.graph.get_mutex_rotation_points();
        let rotation = self.protocol.rotation;
        let index_thread = self.graph.get_rotation_thread_index();
        index_thread.fetch_add(1, Ordering::Relaxed);
        let index_thead_initial = index_thread.load(Ordering::Relaxed);
        // Rotation
        thread::spawn(move || {
            points_rotation.lock().unwrap().clear();
            let mut stepgen = rotation.create_stepgen();
            let mut delay_acc_us = 0;
            let mut rpm_for_graph = 0.0;
            let mut current_time = 0.0;
            let mut last_rpm = 0.0;
            let now = |prev_delay: u64| -> TimerInstantU64<1000> {
                TimerInstantU64::from_ticks((prev_delay as f64 * 0.001) as u64)
            };
            while let Some(delay) = stepgen.next_delay(Some(now(delay_acc_us))) {
                if points_rotation.lock().unwrap().len() > MAX_POINTS_GRAPHS {
                    return;
                }
                current_time = delay_acc_us as f64 * 0.001;
                rpm_for_graph = 300_000.0 / rotation.step_mode.get_multiplier() as f64 / (delay + 1) as f64;
                if index_thead_initial != index_thread.load(Ordering::Relaxed) {
                    return;
                }
                if rpm_for_graph != last_rpm {
                    points_rotation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                    last_rpm = rpm_for_graph;
                } else if (current_time as u64) % 100_000 == 0 {
                    points_rotation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                }
                delay_acc_us += delay;
            }
            points_rotation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
        });
    }

    pub fn generate_graph_agitation(&self) {
        let points_agitation = self.graph.get_mutex_agitation_points();
        let agitation = self.protocol.agitation;
        let index_thread = self.graph.get_agitation_thread_index();
        index_thread.fetch_add(1, Ordering::Relaxed);
        let index_thead_initial = index_thread.load(Ordering::Relaxed);
        // Agitation
        thread::spawn(move || {
            points_agitation.lock().unwrap().clear();
            let mut stepgen = agitation.create_stepgen();
            let mut delay_acc_us = 0;
            let mut rpm_for_graph = 0.0;
            let mut current_time = 0.0;
            let mut last_rpm = 0.0;
            let now = |prev_delay: u64| -> TimerInstantU64<1000> {
                TimerInstantU64::from_ticks((prev_delay as f64 * 0.001) as u64)
            };
            while let Some(delay) = stepgen.next_delay(Some(now(delay_acc_us))) {
                if points_agitation.lock().unwrap().len() > MAX_POINTS_GRAPHS {
                    return;
                }
                current_time = delay_acc_us as f64 * 0.001;
                rpm_for_graph = 300_000.0 / agitation.step_mode.get_multiplier() as f64 / (delay + 1) as f64;
                if index_thead_initial != index_thread.load(Ordering::Relaxed) {
                    return;
                }
                if rpm_for_graph != last_rpm {
                    points_agitation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                    last_rpm = rpm_for_graph;
                } else if (current_time as u64) % 100_000 == 0 {
                    points_agitation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                }
                delay_acc_us += delay;
            }
            points_agitation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
        });
    }
}