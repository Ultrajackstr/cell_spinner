use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::Error;

use crate::app::THREAD_SLEEP;
use crate::utils::graph::Graph;
use crate::utils::protocols::Protocol;
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
        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        self.start_run_time();
        self.serial.listen_to_serial_port(&self.is_running, message_tx);
        let test_protocol = Protocol::test_protocol();
        self.serial.send_bytes(test_protocol.bytes_vec_to_send());
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
}