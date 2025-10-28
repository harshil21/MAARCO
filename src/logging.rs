// src/logger.rs
use csv::Writer;
use serde::Serialize;
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::usb_serial::SensorData;


// Helper macro to create structs with field names
macro_rules! make_struct_with_fields {
    (
        $(#[$attr:meta])*
        pub struct $name:ident { $($fname:ident : $ftype:ty),* $(,)? }
    ) => {
        $(#[$attr])*
        pub struct $name {
            $($fname : $ftype),*
        }

        impl $name {
            fn field_names() -> Vec<String> {
                vec![$(stringify!($fname).to_string()),*]
            }
        }
    };
}

make_struct_with_fields! {
    #[derive(Debug, Clone, Serialize)]
    pub struct NmeaSentence {
        sentence: String,
    }
}
make_struct_with_fields! {
    #[derive(Debug, Clone, Serialize)]
    pub struct RtcmData {
        message_type: Option<u16>,
        data_length: usize,
        data_hex: String, // Hex representation of RTCM data
    }
}
make_struct_with_fields! {
    #[derive(Debug, Clone, Serialize)]
    pub struct ArduinoSensorData {
        motor_1_rpm: f32,
        motor_2_rpm: f32,
        motor_1_tot_rotations: f32,
        motor_2_tot_rotations: f32,
        time_ms: u32,
        // roll: f32,
        // pitch: f32,
        // yaw: f32,
        current_motor_1_ma: f32,
        current_motor_2_ma: f32,
        sonar_mm: f32,
        tof1_mm: f32,
        tof2_mm: f32,
        // acc_x: f32,
        // acc_y: f32,
        // acc_z: f32,
    }
}

// Wrapper structs for CSV serialization
#[derive(Serialize)]
struct NmeaLog {
    timestamp_us: u64,
    log_type: &'static str,
    data: NmeaSentence,
}

#[derive(Serialize)]
struct RtcmLog {
    timestamp_us: u64,
    log_type: &'static str,
    data: RtcmData,
}

#[derive(Serialize)]
struct SensorLog {
    timestamp_us: u64,
    log_type: &'static str,
    data: ArduinoSensorData,
}


/// Represents different types of data that can be logged
#[derive(Debug, Clone)]
pub enum LogData {
    NmeaSentence(NmeaSentence),
    RtcmData(RtcmData),
    SensorData(ArduinoSensorData),
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
        let data = NmeaSentence {
            sentence: sentence.trim().to_string(),
        };
        let _ = self.tx.send(LogData::NmeaSentence(data));
    }

    /// Log RTCM correction data
    pub fn log_rtcm(&self, data: &[u8]) {
        let message_type = extract_rtcm_message_type(data);
        let data_hex = hex_encode(data);
        let data = RtcmData {
            message_type,
            data_length: data.len(),
            data_hex: data_hex,
        };
        let _ = self.tx.send(LogData::RtcmData(data));
    }

    pub fn log_sensor_data(&self, data: &SensorData) {
        let data = ArduinoSensorData {
            motor_1_rpm: data.motor_1_rpm,
            motor_2_rpm: data.motor_2_rpm,
            motor_1_tot_rotations: data.motor_1_tot_rotations,
            motor_2_tot_rotations: data.motor_2_tot_rotations,
            time_ms: data.time_ms,
            // roll: data.roll,
            // pitch: data.pitch,
            // yaw: data.yaw,
            current_motor_1_ma: data.current_motor_1_ma,
            current_motor_2_ma: data.current_motor_2_ma,    
            sonar_mm: data.sonar_mm,
            tof1_mm: data.tof1_mm,
            tof2_mm: data.tof2_mm,
        };

        let _ = self.tx.send(LogData::SensorData(data));
    }
}

/// Main logging thread function
fn run_logger(rx: Receiver<LogData>, log_file_path: PathBuf) -> std::io::Result<()> {
    let file = File::create(&log_file_path)?;
    let mut writer = Writer::from_writer(file);

    // Write CSV header
    let sensor_data_fields = ArduinoSensorData::field_names();
    let nmea_fields = NmeaSentence::field_names();
    let rtcm_fields = RtcmData::field_names();

    let mut fields = vec![
        "timestamp_ns".to_string(),
        "log_type".to_string(),
    ];
    fields.extend(nmea_fields);
    fields.extend(rtcm_fields);
    fields.extend(sensor_data_fields);

    writer.write_record(&fields)?;

    writer.flush()?;

    loop {
        match rx.recv() {
            Ok(data) => {
                println!("Logging data: {:?}", data);
                write_log_entry(&mut writer, data)?;
                writer.flush()?;
            }
            Err(err) => {

                // Channel closed, exit logging thread
                eprintln!("Logging thread error: {}", err);
                break;
            }
        }
    }

    Ok(())
}

/// Write a single log entry to the CSV file
fn write_log_entry(writer: &mut Writer<File>, data: LogData) -> csv::Result<()> {
    let timestamp = get_timestamp_nanos();
    match data {
        LogData::NmeaSentence(sentence) => {
            let record = vec![
                timestamp.to_string(),
                "NMEA".to_string(),
                sentence.sentence,
            ];
            writer.write_record(record)?;
        }
        LogData::RtcmData(rtcm) => {
            let record = vec![
                timestamp.to_string(),
                "RTCM".to_string(),
                rtcm.message_type.map_or(String::new(), |m| m.to_string()),
                rtcm.data_length.to_string(),
                rtcm.data_hex,
            ];
            writer.write_record(record)?;
        }
        LogData::SensorData(sensor) => {
            let record = vec![
                timestamp.to_string(),
                "ARDUINO".to_string(),
                sensor.motor_1_rpm.to_string(),
                sensor.motor_2_rpm.to_string(),
                sensor.motor_1_tot_rotations.to_string(),
                sensor.motor_2_tot_rotations.to_string(),
                sensor.time_ms.to_string(),
                sensor.current_motor_1_ma.to_string(),
                sensor.current_motor_2_ma.to_string(),
                sensor.sonar_mm.to_string(),
                sensor.tof1_mm.to_string(),
                sensor.tof2_mm.to_string(),
            ];
            writer.write_record(record)?;
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
