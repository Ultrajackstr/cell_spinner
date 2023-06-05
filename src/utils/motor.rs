use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64};

use anyhow::Error;

use crate::utils::graph::Graph;
use crate::utils::protocols::Protocol;
use crate::utils::serial::Serial;

pub struct Motor {
    name: String,
    is_running: Arc<AtomicBool>,
    run_time_ms: Arc<AtomicU64>,
    protocol: Protocol,
    serial: Serial,
    graph: Arc<Mutex<Graph>>,
}

impl Default for Motor {
    fn default() -> Self {
        Self {
            name: String::from(""),
            is_running: Arc::new(AtomicBool::new(false)),
            run_time_ms: Arc::new(AtomicU64::new(0)),
            protocol: Protocol::default(),
            serial: Serial::default(),
            graph: Arc::new(Mutex::new(Graph::default())),
        }
    }
}

impl Motor {
    pub fn new(serial_port: &str, motor_name: &str) -> Result<Self, Error> {
        let serial = Serial::new(serial_port)?;
        Ok(Self {
            name: motor_name.into(),
            is_running: Arc::new(AtomicBool::new(false)),
            run_time_ms: Arc::new(AtomicU64::new(0)),
            protocol: Protocol::default(),
            serial,
            graph: Arc::new(Mutex::new(Graph::default())),
        })
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
    }

    pub fn get_protocol(&self) -> Protocol {
        self.protocol.clone()
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_is_running(&mut self, is_running: bool) {
        self.is_running.store(is_running, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_run_time_ms(&self) -> u64 {
        self.run_time_ms.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_run_time_ms(&mut self, run_time_ms: u64) {
        self.run_time_ms.store(run_time_ms, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_graph(&self) -> Arc<Mutex<Graph>> {
        self.graph.clone()
    }

    pub fn set_graph(&mut self, graph: Graph) {
        self.graph = Arc::new(Mutex::new(graph));
    }
}