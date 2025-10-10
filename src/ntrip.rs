// src/ntrip.rs
use base64::{Engine as _, engine::general_purpose};
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub fn connect_rtk2go_ntrip(tx: Sender<Vec<u8>>, mountpoint: &str) -> () {
    // Connect to the RTK2GO NTRIP caster
    let maybe_stream = TcpStream::connect(("rtk2go.com", 2101));
    let mut stream = maybe_stream.expect("Could not connect to RTK2GO!");

    // Prepare the auth header
    let auth = format!("{}:{}", "hoppingturtles@proton.me", "none");
    let auth_b64 = general_purpose::STANDARD.encode(auth);

    // Construct NTRIP request header
    let request = format!(
        "GET /{} HTTP/1.0\r\n\
        User-Agent: NTRIP RustClient\r\n\
        Accept: */*\r\n\
        Authorization: Basic {}\r\n\
        Connection: close\r\n\r\n",
        mountpoint, auth_b64
    );

    if let Err(e) = stream.write_all(request.as_bytes()) {
        eprintln!("Failed to send NTRIP request: {:?}", e);
        return;
    }

    // Check if our server accepted the connection:
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    // Read status line
    if let Err(e) = reader.read_line(&mut line) {
        eprintln!("Failed to read NTRIP response: {:?}", e);
        return;
    }

    if !line.contains("200 OK") && !line.contains("ICY 200 OK") {
        // Some servers use ICY
        eprintln!("NTRIP connection failed: {}", line.trim());
    }

    // Read and print incoming RTCM data chunks
    let mut buf = [0u8; 4096];
    loop {
        let n = stream.read(&mut buf);

        match n {
            Ok(n) => {
                if n == 0 {
                    eprintln!("NTRIP stream closed");
                }
                // Here you can process or forward the data. We'll just print length for this example:
                // println!("Received {} bytes of RTCM data", n);

                let _ = tx
                    .send(buf[0..n].to_vec())
                    .map_err(|e| io::Error::new(ErrorKind::Other, e));
            }

            Err(e) => {
                eprintln!("NTRIP read error: {:?}. Trying again in 5 seconds", e);
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}
