use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Error};
use egui_toast::ToastKind;
use parking_lot::Mutex;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};

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
        let mut buf: [u8; 3];
        // First send the "bye!" command to be sure the RP-Pico is initialized
        system_port_unwrapped.write_all(b"bye!")?;
        thread::sleep(Duration::from_millis(200));
        system_port_unwrapped.clear(ClearBuffer::All)?;
        buf = [0u8; 3];
        let mut counter = 0;
        // Write "hi?" to serial port
        system_port_unwrapped.write_all(b"hi?")?;
        loop {
            system_port_unwrapped.read_exact(&mut buf)?;
            if buf == [b'o', b'k', b'!'] {
                system_port_unwrapped.clear(ClearBuffer::All)?;
                break;
            } else {
                counter += 1;
                if counter >= 10 {
                    system_port_unwrapped.clear(ClearBuffer::All)?;
                    bail!("Raspberry connection failed after 10 retries");
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
        thread::spawn(move || {
            while is_running.load(Ordering::Relaxed) {
                let mut buf: [u8; 3];
                // Check if there is a byte to read
                let is_byte = match port.lock().as_mut().unwrap().bytes_to_read() {
                    Ok(n) => n,
                    Err(err) => {
                        let error = Some(anyhow!(err));
                        let message: Message = Message::new(ToastKind::Error, "Error while reading serial port", error, Some(motor_name.clone()), 5, false);
                        message_tx.as_ref().unwrap().send(message).unwrap();
                        return;
                    }
                };
                if is_byte != 0 {
                    buf = [0u8; 3];
                    match port.lock().as_mut().unwrap().read_exact(&mut buf) {
                        Ok(_) => {
                            let state: StepperState = StepperState::from(&buf);
                            let origin = Some(motor_name.clone());
                            let message = state.to_string();
                            match state {
                                StepperState::Invalid => {
                                    port.lock().as_mut().unwrap().clear(ClearBuffer::All).ok();
                                }
                                StepperState::CommandReceived => {}
                                StepperState::StepgenAgitationError | StepperState::StepgenRotationError | StepperState::EmergencyStop | StepperState::OpenLoad
                                | StepperState::OverHeat | StepperState::OverCurrent => {
                                    let error = Some(anyhow!("Received: \"{}\" {:?}", String::from_utf8(buf.to_vec()).unwrap(), &buf));
                                    timers_and_phases.lock().set_stop_time_motor_stopped();
                                    timers_and_phases.lock().phase = state;
                                    timers_and_phases.lock().phase_start_time = None;
                                    timers_and_phases.lock().global_phase = state;
                                    timers_and_phases.lock().global_phase_start_time = None;
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, Ordering::Relaxed);
                                }
                                StepperState::Finished => {
                                    timers_and_phases.lock().set_stop_time_motor_stopped();
                                    timers_and_phases.lock().phase = state;
                                    timers_and_phases.lock().phase_start_time = None;
                                    timers_and_phases.lock().global_phase = state;
                                    timers_and_phases.lock().global_phase_start_time = None;
                                    let message: Message = Message::new(ToastKind::Success, &message, None, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, Ordering::Relaxed);
                                }
                                StepperState::StartRotation | StepperState::StartAgitation => {
                                    timers_and_phases.lock().global_phase = StepperState::StartRotation;
                                    timers_and_phases.lock().global_phase_start_time = Some(Instant::now());
                                }
                                _ => {
                                    timers_and_phases.lock().phase = state;
                                    timers_and_phases.lock().phase_start_time = Some(Instant::now());
                                }
                            }
                        }
                        Err(err) => {
                            let error = Some(anyhow!(err));
                            let message: Message = Message::new(ToastKind::Error, "Error while reading serial port", error, Some(format!("Port: {}", motor_name)), 5, false);
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
        if let Some(port) = self.port.lock().as_mut() {
            port.clear(ClearBuffer::All).ok();
            port.write_all(bytes).ok();
        }
    }
}