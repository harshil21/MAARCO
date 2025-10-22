use serialport;
use serialport::Error;
use serialport::TTYPort;
use std::io::{self, BufRead, BufReader, Write};
use std::time::Duration;


#[derive(Debug, Clone)]
pub struct SensorData {
    pub time_ms: u32,
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    // pub acc_x: f32,
    // pub acc_y: f32,
    // pub acc_z: f32,
    pub sonar_mm: f32,
    pub tof1_mm: f32,
    pub tof2_mm: f32,
}


#[derive(Debug)]
pub struct ArduinoSerialPort {
    reader: BufReader<TTYPort>,  // Buffered reader for line-based reads
}


pub fn open_port() -> Result<ArduinoSerialPort, Error> {
    const PORT_NAME: &str = "/dev/ttyACM0";
    const BAUD_RATE: u32 = 9600;

    match serialport::new(PORT_NAME, BAUD_RATE)
        .timeout(Duration::from_secs(1))  // 1s timeout for reads
        .open_native()
    {
        Ok(port) => {
            println!("Successfully opened port {} at {} baud.", PORT_NAME, BAUD_RATE);
            let reader = BufReader::new(port);  // Wrap for buffered line reads
            Ok(ArduinoSerialPort { reader })
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", PORT_NAME, e);
            Err(e)
        }
    }
}


impl ArduinoSerialPort {
    pub fn read_line(&mut self) -> Result<Option<SensorData>, Error> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Ok(None),  // EOF (unlikely on serial)
            Ok(bytes_read) if bytes_read > 0 => {
                // Remove trailing newline chars
                let trimmed = line.trim_end_matches(|c| c == '\r' || c == '\n');
                if trimmed.is_empty() {
                    return Ok(None);
                }
                // println!("Read line: {} ({} bytes)", trimmed, bytes_read);

                // Parse the line into SensorData
                let parts: Vec<&str> = trimmed.split(",").map(|s| s.trim()).collect();
                if parts.len() != 7 {
                    return Err(Error::new(
                        serialport::ErrorKind::Io(std::io::ErrorKind::InvalidData),
                        format!("Expected 7 parts, got {}", parts.len()),
                    ));
                }

                let time_ms: u32 = parts[0].parse().unwrap();
                let roll: f32 = parts[1].parse().unwrap();
                let pitch: f32 = parts[2].parse().unwrap();
                let yaw: f32 = parts[3].parse().unwrap();
                let sonar_mm: f32 = parts[4].parse().unwrap();
                let tof1_mm: f32 = parts[5].parse().unwrap();
                let tof2_mm: f32 = parts[6].parse().unwrap();
                // let acc_x: f32 = parts[7].parse().unwrap();
                // let acc_y: f32 = parts[8].parse().unwrap();
                // let acc_z: f32 = parts[9].parse().unwrap();

                let data = SensorData {
                    time_ms,
                    sonar_mm,
                    tof1_mm,
                    tof2_mm,
                    roll,
                    pitch,
                    yaw,
                    // acc_x,
                    // acc_y,
                    // acc_z,
                };

                // println!("Parsed data: {:?}", data);
                Ok(Some(data))
            }
            Ok(_) => Ok(None),  // Should not happen
            Err(e) => {
                eprintln!("Error reading line from Arduino serial port: {}", e);
                Err(Error::from(e))
            }
        }
    }

    pub fn get_port_mut(&mut self) -> &mut TTYPort {
        self.reader.get_mut()
    }
}


impl Write for ArduinoSerialPort {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_port_mut().write(buf)
    }

    fn flush(&mut self, ) -> io::Result<()> {
        self.get_port_mut().flush()
    }
}

