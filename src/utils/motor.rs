use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Error};
use fugit::TimerInstantU64;

use crate::app::{MAX_ACCELERATION, MAX_DURATION_MS, MAX_POINTS_GRAPHS, THREAD_SLEEP};
use crate::utils::enums::{Direction, StepMode128};
use crate::utils::graph::Graph;
use crate::utils::protocols::{Protocol, Rotation};
use crate::utils::serial::Serial;
use crate::utils::structs::Message;

pub struct Motor {
    name: String,
    is_running: Arc<AtomicBool>,
    run_time_ms: Arc<Mutex<Duration>>,
    protocol: Protocol,
    serial: Serial,
    graph: Graph,
}

impl Default for Motor {
    fn default() -> Self {
        Self {
            name: String::from(""),
            is_running: Arc::new(AtomicBool::new(false)),
            run_time_ms: Arc::new(Mutex::new(Duration::from_millis(0))),
            protocol: Protocol::default(),
            serial: Serial::default(),
            graph: Graph::default(),
        }
    }
}

impl Motor {
    pub fn new(serial_port: String, motor_name: String, already_connected_ports: Arc<Mutex<Vec<String>>>) -> Result<Self, Error> {
        let serial = Serial::new(&serial_port, already_connected_ports)?;
        Ok(Self {
            name: motor_name,
            is_running: Arc::new(AtomicBool::new(false)),
            run_time_ms: Arc::new(Mutex::new(Duration::from_millis(0))),
            protocol: Protocol::default(),
            serial,
            graph: Graph::default(),
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

    pub fn get_run_time_ms(&self) -> Duration {
        *self.run_time_ms.lock().unwrap()
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
        // Check the durations
        if self.protocol.rotation.pause_before_direction_change_ms != 0 && self.protocol.rotation.duration_of_one_direction_cycle_ms == 0 {
            self.protocol.rotation.pause_before_direction_change_ms = 0;
        }
        if self.protocol.agitation.pause_before_direction_change_ms != 0 && self.protocol.agitation.duration_of_one_direction_cycle_ms == 0 {
            self.protocol.agitation.pause_before_direction_change_ms = 0;
        }
        let min_rotation_duration = self.protocol.rotation.get_min_duration();
        let min_agitation_duration = self.protocol.agitation.get_min_duration();
        if self.protocol.rotation_duration_ms < min_rotation_duration {
            self.protocol.rotation_duration_ms = min_rotation_duration;
        }
        if self.protocol.agitation_duration_ms < min_agitation_duration {
            self.protocol.agitation_duration_ms = min_agitation_duration;
        }
        if min_rotation_duration == 0 {
            self.protocol.rotation_duration_ms = min_rotation_duration;
        }
        if min_agitation_duration == 0 {
            self.protocol.agitation_duration_ms = min_agitation_duration;
        }
        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        self.start_run_time();
        self.serial.listen_to_serial_port(&self.is_running, message_tx);
        self.serial.send_bytes(self.protocol.bytes_vec_to_send());
    }

    pub fn stop_motor(&mut self) {
        self.serial.send_bytes(vec![b'x']);
        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn start_run_time(&mut self) {
        let is_running = self.is_running.clone();
        let run_time_ms = self.run_time_ms.clone();
        std::thread::spawn(move || {
            let start_time = std::time::Instant::now();
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                let elapsed_time = start_time.elapsed();
                *run_time_ms.lock().unwrap() = elapsed_time;
                std::thread::sleep(Duration::from_millis(THREAD_SLEEP));
            }
        });
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
        if protocol.rotation.duration_of_one_direction_cycle_ms > MAX_DURATION_MS || protocol.agitation.duration_of_one_direction_cycle_ms > MAX_DURATION_MS
            || protocol.rotation.pause_before_direction_change_ms > MAX_DURATION_MS || protocol.agitation.pause_before_direction_change_ms > MAX_DURATION_MS
            || protocol.global_duration_ms > MAX_DURATION_MS || protocol.rotation_duration_ms > MAX_DURATION_MS || protocol.agitation_duration_ms > MAX_DURATION_MS
            || protocol.pause_before_agitation_ms > MAX_DURATION_MS || protocol.pause_after_agitation_ms > MAX_DURATION_MS
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
            let now = |prev_delay: u64| -> TimerInstantU64<1000> {
                TimerInstantU64::from_ticks((prev_delay as f64 * 0.001) as u64)
            };
            'stepgen: while let Some(delay) = stepgen.next_delay(Some(now(delay_acc_us))) {
                if points_rotation.lock().unwrap().len() >= MAX_POINTS_GRAPHS {
                    break 'stepgen;
                }
                current_time = delay_acc_us as f64 * 0.001;
                rpm_for_graph = 300_000.0 / rotation.step_mode.get_multiplier() as f64 / (delay + 1) as f64;
                if index_thead_initial != index_thread.load(Ordering::Relaxed) {
                    return;
                }
                if rpm_for_graph == points_rotation.lock().unwrap().last().unwrap_or(&[0.0f64; 2])[1] && current_time as u64 % 1000 == 0 {
                    points_rotation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                } else if current_time as u64 % 50 == 0 {
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
            let now = |prev_delay: u64| -> TimerInstantU64<1000> {
                TimerInstantU64::from_ticks((prev_delay as f64 * 0.001) as u64)
            };
            'stepgen: while let Some(delay) = stepgen.next_delay(Some(now(delay_acc_us))) {
                if points_agitation.lock().unwrap().len() >= MAX_POINTS_GRAPHS {
                    break 'stepgen;
                }
                current_time = delay_acc_us as f64 * 0.001;
                rpm_for_graph = 300_000.0 / agitation.step_mode.get_multiplier() as f64 / (delay + 1) as f64;
                if index_thead_initial != index_thread.load(Ordering::Relaxed) {
                    return;
                }
                if rpm_for_graph == points_agitation.lock().unwrap().last().unwrap_or(&[0.0f64; 2])[1] && current_time as u64 % 1000 == 0 {
                    points_agitation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                } else if current_time as u64 % 50 == 0 {
                    points_agitation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
                }
                delay_acc_us += delay;
            }
            points_agitation.lock().unwrap().push([current_time * 0.001, rpm_for_graph]);
        });
    }
}