use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{bail, Error};

use crate::app::{MAX_ACCELERATION, MAX_DURATION_MS, THREAD_SLEEP};
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
    graph: Arc<Mutex<Graph>>,
}

impl Default for Motor {
    fn default() -> Self {
        Self {
            name: String::from(""),
            is_running: Arc::new(AtomicBool::new(false)),
            run_time_ms: Arc::new(Mutex::new(Duration::from_millis(0))),
            protocol: Protocol::default(),
            serial: Serial::default(),
            graph: Arc::new(Mutex::new(Graph::default())),
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
            graph: Arc::new(Mutex::new(Graph::default())),
        })
    }

    pub fn new_with_protocol(serial_port: String, motor_name: String, already_connected_ports: Arc<Mutex<Vec<String>>>, protocol: Protocol) -> Result<Self, Error> {
        let mut motor = Self::new(serial_port, motor_name, already_connected_ports)?;
        motor.set_protocol(protocol);
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
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_run_time_ms(&self) -> Duration {
        *self.run_time_ms.lock().unwrap()
    }

    pub fn get_graph(&self) -> Arc<Mutex<Graph>> {
        self.graph.clone()
    }

    pub fn set_graph(&mut self, graph: Graph) {
        self.graph = Arc::new(Mutex::new(graph));
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
}