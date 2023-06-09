use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Error};
use egui_toast::ToastKind;
use parking_lot::Mutex;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

use crate::app::THREAD_SLEEP;
use crate::utils::enums::StepperState;
use crate::utils::structs::{Message, TimersAndPhases};

#[derive(Default)]
pub struct Serial {
    pub port_name: String,
    pub port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
}

impl Serial {
    pub fn new(port_name: &str, already_connected_ports: Arc<Mutex<Vec<String>>>) -> Result<Self, Error> {
        let port = Self::connect_to_serial_port(port_name)?;
        let port = Arc::new(port);
        already_connected_ports.lock().push(port_name.into());
        Ok(Self {
            port_name: port_name.into(),
            port,
        })
    }

    fn connect_to_serial_port(port_name: &str) -> Result<Mutex<Option<Box<dyn SerialPort>>>, Error> {
        let mut system_port_unwrapped = serialport::new(port_name, 500000)
            .parity(Parity::None)
            .data_bits(DataBits::Eight)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(2000))
            .open()?;
        let mut buf = [0u8; 3];
        let mut counter = 0;
        // Write "helo" to serial port
        loop {
            system_port_unwrapped.write_all(b"helo")?;
            system_port_unwrapped.read_exact(&mut buf)?;
            if buf == [b'o', b'k', b'!'] {
                break;
            } else {
                counter += 1;
                tracing::info!("Raspberry connection failed, retrying... ({})", counter);
                if counter >= 15 {
                    bail!("Raspberry connection failed after 15 retries");
                }
                thread::sleep(Duration::from_millis(500));
            }
        }
        Ok(Mutex::new(Some(system_port_unwrapped)))
    }

    pub fn get_is_connected(&self) -> bool {
        self.port.lock().is_some()
    }

    pub fn disconnect(&self) {
        if let Some(mut port) = self.port.lock().take() {
            port.write_all(b"bye!").ok();
        }
    }

    pub fn listen_to_serial_port(&self, motor_name: String, is_running: &Arc<AtomicBool>, timers_and_phases: &Arc<Mutex<TimersAndPhases>>, message_tx: Option<Sender<Message>>) {
        let port = self.port.clone();
        let is_running = is_running.clone();
        let timers_and_phases = timers_and_phases.clone();
        let port_name = self.port_name.clone();
        thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                if port.lock().is_none() {
                    return;
                }
                let mut buf: [u8; 3];
                // Check if there is a byte to read
                let result = port.lock().as_ref().unwrap().bytes_to_read();
                let is_byte = match result {
                    Ok(n) => n,
                    Err(err) => {
                        is_running.store(false, Ordering::SeqCst);
                        {
                            let mut lock = timers_and_phases.lock();
                            lock.set_global_stop_time_stopped();
                            lock.sub_phase = StepperState::Invalid;
                            lock.sub_phase_start_time = None;
                            lock.main_phase = StepperState::Invalid;
                            lock.main_phase_start_time = None;
                        }
                        // port.lock().take();
                        let error = Some(anyhow!(err));
                        let message: Message = Message::new(ToastKind::Error, &format!("Error while reading serial port {} - ⚠️YOU SHOULD RECONNECT⚠️", port_name), error, Some(motor_name.clone()), 5, false);
                        message_tx.as_ref().unwrap().send(message).unwrap();
                        return;
                    }
                };
                if is_byte != 0 {
                    buf = [0u8; 3];
                    let read_exact_result = port.lock().as_mut().unwrap().read_exact(&mut buf);
                    match read_exact_result {
                        Ok(_) => {
                            let state: StepperState = StepperState::from(&buf);
                            let origin = Some(motor_name.clone());
                            let message = state.to_string();
                            match state {
                                StepperState::Invalid => {
                                    is_running.store(false, Ordering::SeqCst);
                                    {
                                        let mut lock = timers_and_phases.lock();
                                        lock.set_global_stop_time_stopped();
                                        lock.sub_phase = StepperState::Invalid;
                                        lock.sub_phase_start_time = None;
                                        lock.main_phase = StepperState::Invalid;
                                        lock.main_phase_start_time = None;
                                    }
                                    // port.lock().take();
                                    let error = Some(anyhow!("Invalid state received. Disconnecting..."));
                                    let message: Message = Message::new(ToastKind::Error, &format!("Error while reading serial port {} - ⚠️YOU SHOULD RECONNECT⚠️", port_name), error, Some(motor_name), 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    return;
                                }
                                StepperState::CommandReceived => {}
                                StepperState::StepgenAgitationError | StepperState::StepgenRotationError | StepperState::EmergencyStop | StepperState::OpenLoad
                                | StepperState::OverHeat | StepperState::OverCurrent => {
                                    is_running.store(false, Ordering::SeqCst);
                                    timers_and_phases.lock().set_global_stop_time_stopped();
                                    {
                                        let mut lock = timers_and_phases.lock();
                                        lock.sub_phase = state;
                                        lock.sub_phase_start_time = None;
                                        lock.main_phase = state;
                                        lock.main_phase_start_time = None;
                                    }
                                    let error = Some(anyhow!("Motor stopped !"));
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                }
                                StepperState::Finished => {
                                    is_running.store(false, Ordering::SeqCst);
                                    {
                                        let mut lock = timers_and_phases.lock();
                                        lock.set_global_stop_time_stopped();
                                        lock.sub_phase = state;
                                        lock.sub_phase_start_time = None;
                                        lock.main_phase = state;
                                        lock.main_phase_start_time = None;
                                    }
                                    let message: Message = Message::new(ToastKind::Success, &message, None, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                }
                                StepperState::StartRotation | StepperState::StartAgitation => {
                                    let mut lock = timers_and_phases.lock();
                                    lock.main_phase = state;
                                    lock.main_phase_start_time = Some(Instant::now());
                                }
                                StepperState::OscillationRotation => {
                                    let mut lock = timers_and_phases.lock();
                                    let direction = lock.rotation_direction.reverse();
                                    lock.rotation_direction = direction;
                                    lock.sub_phase = state;
                                    lock.sub_phase_start_time = Some(Instant::now());
                                }
                                StepperState::OscillationAgitation => {
                                    let mut lock = timers_and_phases.lock();
                                    let direction = lock.agitation_direction.reverse();
                                    lock.agitation_direction = direction;
                                    lock.sub_phase = state;
                                    lock.sub_phase_start_time = Some(Instant::now());
                                }
                                _ => {
                                    let mut lock = timers_and_phases.lock();
                                    lock.sub_phase = state;
                                    lock.sub_phase_start_time = Some(Instant::now());
                                }
                            }
                        }
                        Err(err) => {
                            is_running.store(false, Ordering::SeqCst);
                            {
                                let mut lock = timers_and_phases.lock();
                                lock.set_global_stop_time_stopped();
                                lock.sub_phase = StepperState::Invalid;
                                lock.sub_phase_start_time = None;
                                lock.main_phase = StepperState::Invalid;
                                lock.main_phase_start_time = None;
                            }
                            // port.lock().take();
                            let error = Some(anyhow!(err));
                            let message: Message = Message::new(ToastKind::Error, &format!("Error while reading serial port {} - ⚠️YOU SHOULD RECONNECT⚠️", port_name), error, Some(motor_name), 5, false);
                            message_tx.as_ref().unwrap().send(message).unwrap();
                            return;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(THREAD_SLEEP));
            }
        });
    }

    pub fn send_bytes(&self, bytes: &[u8]) {
        let now = Instant::now();
        let future = now + Duration::from_millis(THREAD_SLEEP + 5);
        if let Some(mut lock) = self.port.try_lock_until(future) {
            if let Some(port) = lock.as_mut() {
                port.write_all(bytes).ok();
            }
        }
    }
}