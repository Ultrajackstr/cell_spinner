use std::sync::mpsc::Sender;
use std::time::Duration;

use egui_toast::{Toast, ToastKind, ToastOptions};

/// Wrapper for toast notifications sender.
/// Send a toast notification with the given kind, text and duration.
pub fn send_toast(toast_tx: &Option<Sender<Toast>>, kind: ToastKind, text: String, duration: u64) {
    if let Some(toast_tx) = toast_tx {
        toast_tx.send(Toast { kind, text: text.into(), options: ToastOptions::with_duration(Duration::from_secs(duration)) }).ok();
    }
}