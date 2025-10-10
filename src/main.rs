// src/main.rs
use clap::Parser;
use crossterm::execute;
use std::io::{Write, stdout};
use std::sync::mpsc::{self, TryRecvError};

mod display;
mod gps;
mod ntrip;
mod serial;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// NTRIP mountpoint (e.g., MOUNTPOINT)
    #[arg(long)]
    ntrip_mount: Option<String>,
}

fn main() -> std::io::Result<()> {
    // fn main() -> () {
    let args = Args::parse();

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
        }

        // Write any pending NTRIP correction data to serial
        loop {
            match rx.try_recv() {
                Ok(data) => {
                    // println!("Writing {} bytes of NTRIP data to serial", data.len());
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
