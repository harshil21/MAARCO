// src/logger.rs
use csv::Writer;
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::usb_serial::SensorData;


/// Represents different types of data that can be logged
#[derive(Debug, Clone)]
pub enum LogData {
    NmeaSentence {
        timestamp: u64,
        sentence: String,
    },
    RtcmData {
        timestamp: u64,
        message_type: Option<u16>,
        data_length: usize,
        data_hex: String, // Hex representation of RTCM data
    },
    Imu {
        timestamp: u64,
        accel_x: f64,
        accel_y: f64,
        accel_z: f64,
        gyro_x: f64,
        gyro_y: f64,
        gyro_z: f64,
    },
    Barometer {
        timestamp: u64,
        pressure_pa: f64,
        temperature_c: f64,
        altitude_m: Option<f64>,
    },
    RadioTelemetry {
        timestamp: u64,
        rssi: i16,
        snr: f64,
        packet_count: u32,
        data: String,
    },
    SensorData {
        timestamp: u64,
        time_ms: u32,
        roll: f32,
        pitch: f32,
        yaw: f32,
        sonar_mm: f32,
        tof1_mm: f32,
        tof2_mm: f32,
        // acc_x: f32,
        // acc_y: f32,
        // acc_z: f32,
    }
}

/// Logger handle that can be cloned and sent to other threads
#[derive(Clone)]
pub struct Logger {
    tx: Sender<LogData>,
}

impl Logger {
    /// Create a new logger that writes to the specified file
    pub fn new(log_file_path: PathBuf) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel::<LogData>();

        // Spawn logging thread
        thread::spawn(move || {
            if let Err(e) = run_logger(rx, log_file_path) {
                eprintln!("Logger error: {}", e);
            }
        });

        Ok(Logger { tx })
    }

    /// Log a NMEA sentence
    pub fn log_nmea(&self, sentence: &str) {
        let timestamp = get_timestamp_nanos();
        let _ = self.tx.send(LogData::NmeaSentence {
            timestamp,
            sentence: sentence.trim().to_string(),
        });
    }

    /// Log RTCM correction data
    pub fn log_rtcm(&self, data: &[u8]) {
        let timestamp = get_timestamp_nanos();
        let message_type = extract_rtcm_message_type(data);
        let data_hex = hex_encode(data);

        let _ = self.tx.send(LogData::RtcmData {
            timestamp,
            message_type,
            data_length: data.len(),
            data_hex,
        });
    }

    /// Log IMU data (accelerometer + gyroscope)
    pub fn log_imu(&self, accel: [f64; 3], gyro: [f64; 3]) {
        let timestamp = get_timestamp_nanos();
        let _ = self.tx.send(LogData::Imu {
            timestamp,
            accel_x: accel[0],
            accel_y: accel[1],
            accel_z: accel[2],
            gyro_x: gyro[0],
            gyro_y: gyro[1],
            gyro_z: gyro[2],
        });
    }

    /// Log barometer data
    pub fn log_barometer(&self, pressure_pa: f64, temperature_c: f64, altitude_m: Option<f64>) {
        let timestamp = get_timestamp_nanos();
        let _ = self.tx.send(LogData::Barometer {
            timestamp,
            pressure_pa,
            temperature_c,
            altitude_m,
        });
    }

    /// Log radio telemetry data
    pub fn log_radio(&self, rssi: i16, snr: f64, packet_count: u32, data: String) {
        let timestamp = get_timestamp_nanos();
        let _ = self.tx.send(LogData::RadioTelemetry {
            timestamp,
            rssi,
            snr,
            packet_count,
            data,
        });
    }

    pub fn log_sensor_data(&self, data: &SensorData) {
        let timestamp = get_timestamp_nanos();
        let _ = self.tx.send(LogData::SensorData {
            timestamp,
            time_ms: data.time_ms,
            roll: data.roll,
            pitch: data.pitch,
            yaw: data.yaw,
            sonar_mm: data.sonar_mm,
            tof1_mm: data.tof1_mm,
            tof2_mm: data.tof2_mm,
        });
    }
}

/// Main logging thread function
fn run_logger(rx: Receiver<LogData>, log_file_path: PathBuf) -> std::io::Result<()> {
    let file = File::create(&log_file_path)?;
    let mut writer = Writer::from_writer(file);

    // Write CSV header
    writer.write_record(&[
        "timestamp_us",
        "data_type",
        "nmea_sentence",
        "rtcm_msg_type",
        "rtcm_data_length",
        "rtcm_data_hex",
        "accel_x",
        "accel_y",
        "accel_z",
        "gyro_x",
        "gyro_y",
        "gyro_z",
        "pressure_pa",
        "temperature_c",
        "altitude_m",
        "rssi",
        "snr",
        "packet_count",
        "radio_data",
        "sonar_mm",
        "tof1_mm",
        "tof2_mm",
        "roll",
        "pitch",
        "yaw",
    ])?;

    writer.flush()?;

    loop {
        match rx.recv() {
            Ok(data) => {
                write_log_entry(&mut writer, data)?;
                writer.flush()?;
            }
            Err(_) => {
                // Channel closed, exit logging thread
                break;
            }
        }
    }

    Ok(())
}

/// Write a single log entry to the CSV file
fn write_log_entry(writer: &mut Writer<File>, data: LogData) -> csv::Result<()> {
    match data {
        LogData::NmeaSentence { timestamp, sentence } => {
            writer.write_record(&[
                timestamp.to_string(),
                "NMEA".to_string(),
                sentence,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ])?;
        }
        LogData::RtcmData {
            timestamp,
            message_type,
            data_length,
            data_hex,
        } => {
            writer.write_record(&[
                timestamp.to_string(),
                "RTCM".to_string(),
                String::new(),
                message_type.map_or(String::new(), |m| m.to_string()),
                data_length.to_string(),
                data_hex,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ])?;
        }
        LogData::Imu {
            timestamp,
            accel_x,
            accel_y,
            accel_z,
            gyro_x,
            gyro_y,
            gyro_z,
        } => {
            writer.write_record(&[
                timestamp.to_string(),
                "IMU".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                accel_x.to_string(),
                accel_y.to_string(),
                accel_z.to_string(),
                gyro_x.to_string(),
                gyro_y.to_string(),
                gyro_z.to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ])?;
        }
        LogData::Barometer {
            timestamp,
            pressure_pa,
            temperature_c,
            altitude_m,
        } => {
            writer.write_record(&[
                timestamp.to_string(),
                "BARO".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                pressure_pa.to_string(),
                temperature_c.to_string(),
                altitude_m.map_or(String::new(), |a| a.to_string()),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ])?;
        }
        LogData::RadioTelemetry {
            timestamp,
            rssi,
            snr,
            packet_count,
            data,
        } => {
            writer.write_record(&[
                timestamp.to_string(),
                "RADIO".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                rssi.to_string(),
                snr.to_string(),
                packet_count.to_string(),
                data,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ])?;
        }

        LogData::SensorData {
            timestamp,
            time_ms: _,
            sonar_mm,
            tof1_mm,
            tof2_mm,
            roll,
            pitch,
            yaw,
        } => {
            writer.write_record(&[
                timestamp.to_string(),
                "ARDUINO".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                sonar_mm.to_string(),
                tof1_mm.to_string(),
                tof2_mm.to_string(),
                roll.to_string(),
                pitch.to_string(),
                yaw.to_string(),
            ])?;
        }
    }
    Ok(())
}

/// Get current timestamp in nanoseconds since UNIX epoch
fn get_timestamp_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64
}

/// Extract RTCM message type from data
/// RTCM v3 format: first 12 bits after preamble contain message type
fn extract_rtcm_message_type(data: &[u8]) -> Option<u16> {
    if data.len() < 3 {
        return None;
    }

    // RTCM v3 starts with 0xD3 preamble
    if data[0] != 0xD3 {
        return None;
    }

    // Message type is in bits 12-23 of the header
    // Byte 1 bits 0-5 and byte 2 bits 0-5 contain the message type
    let msg_type = ((data[1] as u16 & 0x3F) << 6) | ((data[2] as u16) >> 2);
    Some(msg_type)
}

/// Convert bytes to hex string
fn hex_encode(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtcm_message_type_extraction() {
        // RTCM message 1005 example
        let data = vec![0xD3, 0x00, 0x13, 0x3E, 0xD0]; // Simplified
        let msg_type = extract_rtcm_message_type(&data);
        assert!(msg_type.is_some());
    }

    #[test]
    fn test_hex_encode() {
        let data = vec![0xD3, 0x00, 0x13];
        assert_eq!(hex_encode(&data), "d30013");
    }
}
