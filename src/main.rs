// src/main.rs
use clap::Parser;
use crossterm::execute;
use std::io::{Write, stdout};
use std::sync::mpsc::{self, TryRecvError};
use std::path::PathBuf;


mod display;
mod gps;
mod ntrip;
mod gps_serial;
mod logging;
mod usb_serial;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// NTRIP mountpoint (e.g., MOUNTPOINT)
    #[arg(long)]
    ntrip_mount: Option<String>,  // E.g. "VMAX-LAND-1"
    /// GPS serial port path (e.g., /dev/ttyUSB0)
    #[arg(long)]
    gps_port: PathBuf,
    /// Arduino serial port path (e.g., /dev/ttyACM0)
    #[arg(long)]
    arduino_port: PathBuf,
    /// Log file path (default: bot_test.csv)
    #[arg(long, default_value = "bot_test.csv")]
    log_file: PathBuf,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let logger = logging::Logger::new(args.log_file)?;
    let mut gps_port = gps_serial::open_port(args.gps_port);
    let mut usb_port = usb_serial::open_port(args.arduino_port);

    let mut parser = gps::parser::build_parser();
    let mut stdout = stdout();
    let mut display = display::Display::new();

    // Initialize terminal
    execute!(stdout, crossterm::cursor::SetCursorStyle::BlinkingBlock)?;

    // Channel for NTRIP data to write to serial
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    // Start NTRIP thread if configured
    if let Some(mount) = args.ntrip_mount {
        std::thread::spawn(move || {
            ntrip::connect_rtk2go_ntrip(tx, &mount);
        });
        println!("Started NTRIP thread");
    }

    loop {
        let gps_serial_data = gps_port.as_mut().unwrap().read_sentences();
        let arduino_serial_data = usb_port.as_mut().unwrap().read_line();

        if gps_serial_data.is_err() {
            continue;
        }
        let sentences = gps_serial_data.unwrap();
        if arduino_serial_data.is_err() {
            continue;
        }
        let sensor_data = arduino_serial_data.unwrap();

        for sentence in &sentences {
            gps::parser::parse_nmea_sentence(&mut parser, sentence);
            logger.log_nmea(sentence);
        }
        if let Some(ref data) = sensor_data {
            logger.log_sensor_data(&data);
        }

        // Write any pending NTRIP correction data to serial
        loop {
            match rx.try_recv() {
                Ok(data) => {
                    // println!("Writing {} bytes of NTRIP data to serial", data.len());
                    // Log RTCM data
                    logger.log_rtcm(&data);
                    gps_port.as_mut().unwrap().write_all(&data)?;
                    gps_port.as_mut().unwrap().flush()?;
                }
                Err(TryRecvError::Empty) => break,
                Err(e) => {
                    eprintln!("Channel error: {:?}", e);
                    break;
                }
            }
        }

        display.update(&mut stdout, &parser, &sensor_data.unwrap())?;
    }
}
