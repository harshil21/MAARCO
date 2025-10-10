// src/main.rs
use clap::Parser;
use crossterm::execute;
use std::io::{Write, stdout};
use std::sync::mpsc::{self, TryRecvError};
use std::path::PathBuf;


mod display;
mod gps;
mod ntrip;
mod serial;
mod logging;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// NTRIP mountpoint (e.g., MOUNTPOINT)
    #[arg(long)]
    ntrip_mount: Option<String>,
    /// Log file path (default: flight_data.csv)
    #[arg(long, default_value = "flight_data.csv")]
    log_file: PathBuf,
}

fn main() -> std::io::Result<()> {
    // fn main() -> () {
    let args = Args::parse();

    let logger = logging::Logger::new(args.log_file)?;
    let mut port = serial::open_port();
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
        let serial_data = port.as_mut().unwrap().read_sentences();

        if serial_data.is_err() {
            continue;
        }
        let sentences = serial_data.unwrap();

        for sentence in &sentences {
            gps::parser::parse_nmea_sentence(&mut parser, sentence);
            logger.log_nmea(sentence);
        }

        // Write any pending NTRIP correction data to serial
        loop {
            match rx.try_recv() {
                Ok(data) => {
                    // println!("Writing {} bytes of NTRIP data to serial", data.len());
                    // Log RTCM data
                    logger.log_rtcm(&data);
                    port.as_mut().unwrap().write_all(&data)?;
                    port.as_mut().unwrap().flush()?;
                }
                Err(TryRecvError::Empty) => break,
                Err(e) => {
                    eprintln!("Channel error: {:?}", e);
                    break;
                }
            }
        }

        display.update(&mut stdout, &parser)?;
    }
}
