# MAARCO

This is the code for the MAARCO project. It involves reading GPS RTK data from a serial port, displaying it in a terminal UI, and optionally connecting to an NTRIP server for real-time corrections.

We are using a Raspberry Pi Zero 2W as the main hardware platform.

All data is also logged to a file. The file is a CSV, which you can find under `logs/`.

## Project structure

- `src/`: Contains the Rust source code for the project.
- `examples/`: Contains example Python scripts for decoding log files and streaming NMEA data. Also includes some Rust code.
- `logs/`: Directory where CSV log files are stored.
- `field_tests/`: This is where the data from field tests is stored. The data can be decoded using the scripts in `examples/decode_log_file.py`.


## How to run

Since this project uses both Rust and Python, ensure you have both installed. For python, you need to install [`uv`](https://docs.astral.sh/uv/getting-started/installation/) to manage dependencies.

To install Rust, follow the instructions at [rustup.rs](https://rustup.rs/).

1. Clone the repository:
   ```bash
   git clone https://github.com/harshil21/MAARCO.git && cd MAARCO
   ```

2. Install Python dependencies:
   ```bash
   uv sync
   ```

3. Build the Rust project:
   ```bash
    cargo build --release
    ``` 

Now, if you want to run the main application on the Raspberry Pi, you would need to compile the code for the ARM architecture. You can do this by setting up a cross-compilation environment. It is highly recommended to use your computer for cross compiling, as compiling directly on the Raspberry Pi can be *very* slow.

You can cross compile via cross and Docker - `docker build -t maarco-aarch64-udev .`, and then: `cross build --target aarch64-unknown-linux-gnu`, and then copy the binary (from `target/`) to the Pi Zero 2W's `target/` folder.

4. Finally, run the application:
    ```bash
    cargo run --release -- --ntrip-mount MOUNTPOINT
    ```

Replace `MOUNTPOINT` with your actual NTRIP mount point. If you don't have one, you can omit the `--ntrip-mount` argument, and the application will run without NTRIP support (so you will not get RTK corrections).


### Running without the Pi:

You can also run the application on your computer if you have a GPS device connected via USB or serial port. Just make sure to specify the correct serial port in the code (currently set to `/dev/ttyUSB0`).


### Running the example scripts:

You can run the example Python scripts to decode log files or stream NMEA data. For example, to decode a log file:

```bash
uv run --script examples/decode_log_file.py
```

Or to run a Rust example:

```bash
cargo run --example file_name[no .rs]
```