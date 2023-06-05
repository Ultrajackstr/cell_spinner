use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Error};
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};

pub struct Serial {
    port_name: String,
    port: Mutex<Option<Box<dyn SerialPort>>>,
}

impl Default for Serial {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            port: Mutex::new(None),
        }
    }
}

impl Serial {
    pub fn new(port_name: &str, already_connected_ports: Arc<Mutex<Vec<String>>>) -> Result<Self, Error> {
        let port = Self::connect_to_serial_port(port_name)?;
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
}