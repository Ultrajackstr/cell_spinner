use std::io::Read;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Error};
use egui_toast::ToastKind;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};

use crate::app::THREAD_SLEEP;
use crate::utils::enums::StepperState;
use crate::utils::structs::Message;

#[derive(Default)]
pub struct Serial {
    port_name: String,
    port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
}

impl Serial {
    pub fn new(port_name: &str, already_connected_ports: Arc<Mutex<Vec<String>>>) -> Result<Self, Error> {
        let port = Self::connect_to_serial_port(port_name)?;
        let port = Arc::new(port);
        already_connected_ports.lock().unwrap().push(port_name.into());
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
        self.port.lock().unwrap().is_some()
    }

    pub fn get_port_name(&self) -> &str {
        &self.port_name
    }

    pub fn disconnect(&self) {
        if let Some(mut port) = self.port.lock().unwrap().take() {
            port.write_all(b"bye!").ok();
        }
    }

    pub fn listen_to_serial_port(&self, is_running: &Arc<AtomicBool>, message_tx: Option<Sender<Message>>) {
        let port = self.port.clone();
        let is_running = is_running.clone();
        let port_name = self.port_name.clone();
        thread::spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                let mut buf: [u8; 3];
                // Check if there is a byte to read
                let is_byte = match port.lock().unwrap().as_mut().unwrap().bytes_to_read() {
                    Ok(n) => n,
                    Err(err) => {
                        let error = Some(Error::new(err));
                        let message: Message = Message::new(ToastKind::Error, "Error while reading serial port", error, Some(format!("Port: {}", port_name)), 5, false);
                        message_tx.as_ref().unwrap().send(message).unwrap();
                        return;
                    }
                };
                if is_byte == 3 {
                    buf = [0u8; 3];
                    match port.lock().unwrap().as_mut().unwrap().read_exact(&mut buf) {
                        Ok(_) => {
                            let state: StepperState = StepperState::from(&buf);
                            let origin = Some(format!("Port: {}", port_name));
                            let error = Some(anyhow!("Received: \"{}\" {:?}", String::from_utf8(buf.to_vec()).unwrap(), &buf));
                            let message = state.to_string();
                            match state {
                                StepperState::CommandReceived => {}
                                StepperState::Finished => { //TODO: check while "fin" is not received
                                    let message: Message = Message::new(ToastKind::Success, &message, None, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                                StepperState::EmergencyStop => {
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                                StepperState::OpenLoad => {
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                                StepperState::OverCurrent => {
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                                StepperState::OverHeat => {
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                                StepperState::ParseError => {
                                    let message: Message = Message::new(ToastKind::Error, &message, error, origin, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                                StepperState::OscillationRotation => {
                                    let message: Message = Message::new(ToastKind::Info, &message, None, origin, 2, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                }
                                StepperState::OscillationAgitation => {
                                    let message: Message = Message::new(ToastKind::Info, &message, None, origin, 2, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                }
                                StepperState::Invalid => {
                                    let message: Message = Message::new(ToastKind::Error, &message, error, None, 5, false);
                                    message_tx.as_ref().unwrap().send(message).unwrap();
                                    is_running.store(false, std::sync::atomic::Ordering::Relaxed);
                                }
                            }
                        }
                        Err(err) => {
                            let error = Some(Error::new(err));
                            let message: Message = Message::new(ToastKind::Error, "Error while reading serial port", error, Some(format!("Port: {}", port_name)), 5, false);
                            message_tx.as_ref().unwrap().send(message).unwrap();
                            return;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(THREAD_SLEEP));
            }
        });
    }

    pub fn send_bytes(&self, bytes: Vec<u8>) {
        let port = self.port.clone();
        thread::spawn(move || {
            if let Some(port) = port.lock().unwrap().as_mut() {
                port.clear(ClearBuffer::All).ok();
                port.write_all(&bytes).ok();
                // port.read_exact(&mut [0u8; 3]).ok();
                // port.clear(ClearBuffer::All).ok();
            }
        });
    }
}