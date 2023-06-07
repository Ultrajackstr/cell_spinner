use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;

#[derive(Debug, Default, Clone)]
pub struct Graph {
    rotation_points: Arc<Mutex<Vec<[f64; 2]>>>,
    rotation_thread_index: Arc<AtomicUsize>,
    agitation_points: Arc<Mutex<Vec<[f64; 2]>>>,
    agitation_thread_index: Arc<AtomicUsize>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            rotation_points: Arc::new(Mutex::new(Vec::new())),
            rotation_thread_index: Arc::new(Default::default()),
            agitation_points: Arc::new(Mutex::new(Vec::new())),
            agitation_thread_index: Arc::new(Default::default()),
        }
    }

    pub fn get_mutex_rotation_points(&self) -> Arc<Mutex<Vec<[f64; 2]>>> {
        self.rotation_points.clone()
    }

    pub fn get_mutex_agitation_points(&self) -> Arc<Mutex<Vec<[f64; 2]>>> {
        self.agitation_points.clone()
    }

    pub fn get_rotation_points(&self) -> Vec<[f64; 2]> {
        self.rotation_points.lock().unwrap().clone()
    }

    pub fn get_agitation_points(&self) -> Vec<[f64; 2]> {
        self.agitation_points.lock().unwrap().clone()
    }
    pub fn get_rotation_thread_index(&self) -> Arc<AtomicUsize> {
        self.rotation_thread_index.clone()
    }

    pub fn get_agitation_thread_index(&self) -> Arc<AtomicUsize> {
        self.agitation_thread_index.clone()
    }
}