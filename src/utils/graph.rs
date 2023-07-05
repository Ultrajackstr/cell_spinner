use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use parking_lot::Mutex;

#[derive(Debug, Default, Clone)]
pub struct Graph {
    pub rotation_points_sec_rpm: Arc<Mutex<Vec<[f64; 2]>>>,
    pub rotation_thread_index: Arc<AtomicUsize>,
    pub agitation_points_sec_rpm: Arc<Mutex<Vec<[f64; 2]>>>,
    pub agitation_thread_index: Arc<AtomicUsize>,
    pub is_generating_rotation_graph: Arc<AtomicBool>,
    pub is_generating_agitation_graph: Arc<AtomicBool>,
}