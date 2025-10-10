use serialport;
use serialport::Error;
use serialport::TTYPort;
use std::io::Read;
use std::io::{self, Write};


#[derive(Debug)]
pub struct SerialReader {
    port: TTYPort,
    buffer: String,
}

pub fn open_port() -> Result<SerialReader, Error> {
    const PORT_NAME: &str = "/dev/ttyUSB0";
    const BAUD_RATE: u32 = 115_200;

    match serialport::new(PORT_NAME, BAUD_RATE)
        .timeout(std::time::Duration::from_millis(10000))
        .open_native()
    {
        Ok(port) => {
            println!("Successfully opened port {}", PORT_NAME);
            Ok(SerialReader {
                port,
                buffer: String::new(),
            })
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {}", PORT_NAME, e);
            Err(e)
        }
    }
}

impl SerialReader {
    pub fn read_sentences(&mut self) -> Result<Vec<String>, Error> {
        let mut serial_buf: Vec<u8> = vec![0; 1024];
        let mut sentences = Vec::new();

        match self.port.read(serial_buf.as_mut_slice()) {
            Ok(t) if t > 0 => {
                let text = String::from_utf8_lossy(&serial_buf[..t]).into_owned();
                self.buffer.push_str(&text);

                loop {
                    if let Some(start) = self.buffer.find('$') {
                        if let Some(end_pos) = self.buffer[start..].find("\r\n") {
                            let end = start + end_pos + 2;
                            let sentence = self.buffer[start..end].to_string();
                            sentences.push(sentence.clone());

                            // Remove the processed sentence from the buffer
                            self.buffer = self.buffer[end..].to_string();
                        } else {
                            // Partial sentence starting with $, keep from $ onward
                            self.buffer = self.buffer[start..].to_string();
                            break;
                        }
                    } else {
                        // No sentence start found, discard garbage
                        self.buffer.clear();
                        break;
                    }
                }
                Ok(sentences)
            }
            Ok(_) => {
                // No data read
                Ok(sentences)
            }
            Err(e) => {
                eprintln!("Error reading from port: {:?}", e);
                Err(Error::from(e))
            }
        }
    }
}

impl Write for SerialReader {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.port.flush()
    }
}
