use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicUsize;

#[derive(Debug, Default, Clone)]
pub struct Graph {
    pub rotation_points: Arc<Mutex<Vec<[f64; 2]>>>,
    pub rotation_thread_index: Arc<AtomicUsize>,
    pub agitation_points: Arc<Mutex<Vec<[f64; 2]>>>,
    pub agitation_thread_index: Arc<AtomicUsize>,
}