use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct Graph {
    rotation_points: Arc<Mutex<Vec<[f64; 2]>>>,
    agitation_points: Arc<Mutex<Vec<[f64; 2]>>>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            rotation_points: Arc::new(Mutex::new(Vec::new())),
            agitation_points: Arc::new(Mutex::new(Vec::new())),
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
}