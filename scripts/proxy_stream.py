import paramiko
import subprocess
import serial
import threading
import time
import sys
import select
import os

# Configuration - customize these
PI_IP = '10.140.80.155'  # Raspberry Pi's IP address
PI_USERNAME = 'harshil'       # Default username
PI_PASSWORD = ''  # Replace with your password (or use key-based auth)
SERIAL_DEVICE = '/dev/serial0'  # Serial port on Pi
BAUD_RATE = 115200         # Baud rate for your sensors (applied on Pi side only)
USE_SUDO = False         # Set to True if you need sudo for cat/stty on Pi
VSERIAL_CLIENT = '/tmp/vserial1'  # Virtual serial for PyGPSClient
VSERIAL_PROXY = '/tmp/vserial2'   # Virtual serial for this script
CHUNK_SIZE = 1           # Bytes to read/send at a time (1 for immediate byte-by-byte forwarding; increase for efficiency)


def configure_serial(ssh):
    sudo_prefix = 'sudo ' if USE_SUDO else ''
    config_cmd = f'{sudo_prefix}stty -F {SERIAL_DEVICE} {BAUD_RATE} raw -echo'
    stdin, stdout, stderr = ssh.exec_command(config_cmd)
    error = stderr.read().decode()
    if error:
        print(f"Error configuring serial: {error}")
        sys.exit(1)
    print("Serial port configured.")

def start_pi_channels(ssh):
    sudo_prefix = 'sudo ' if USE_SUDO else ''
    # Reader channel: cat from serial (with PTY allocation)
    read_cmd = f'{sudo_prefix}cat {SERIAL_DEVICE}'
    read_stdin, read_stdout, read_stderr = ssh.exec_command(read_cmd, get_pty=True)
    
    # Wait briefly for potential error
    time.sleep(0.5)
    read_error = ''
    if read_stderr.channel.recv_stderr_ready():
        read_error = read_stderr.channel.recv_stderr(1024).decode()
    if read_error:
        print(f"Error starting read channel: {read_error}")

    # Writer channel: cat to serial (with PTY allocation)
    write_cmd = f'{sudo_prefix}cat > {SERIAL_DEVICE}'
    write_stdin, write_stdout, write_stderr = ssh.exec_command(write_cmd, get_pty=True)
    
    # Wait briefly for potential error
    time.sleep(0.5)
    write_error = ''
    if write_stderr.channel.recv_stderr_ready():
        write_error = write_stderr.channel.recv_stderr(1024).decode()
    if write_error:
        print(f"Error starting write channel: {write_error}")

    return read_stdout, write_stdin

def forward_to_vserial(read_stdout, vser, stop_event):
    last_log = time.time()
    while not stop_event.is_set():
        if read_stdout.channel.recv_ready():
            data = read_stdout.read(CHUNK_SIZE)
            if data:
                # print(f"Received {len(data)} bytes from Pi's serial, forwarding to virtual serial.")
                vser.write(data)
                vser.flush()
        else:
            if time.time() - last_log > 10:
                print("Still waiting for data from Pi's serial...")
                last_log = time.time()
        time.sleep(0.01)

def forward_from_vserial(write_stdin, vser, stop_event):
    while not stop_event.is_set():
        data = vser.read(CHUNK_SIZE)
        if data:
            # print(f"Received {len(data)} bytes from virtual serial (e.g., from PyGPSClient), forwarding to Pi.")
            write_stdin.write(data)
            write_stdin.flush()
        else:
            time.sleep(0.01)  # Small delay if no data

def proxy_serial_to_vserial():
    global stop_event
    stop_event = threading.Event()
    
    # Start socat to create linked virtual serial ports (install socat if needed: sudo apt install socat)
    socat_cmd = ['socat', f'PTY,link={VSERIAL_CLIENT},raw,echo=0', f'PTY,link={VSERIAL_PROXY},raw,echo=0']
    socat_proc = subprocess.Popen(socat_cmd)
    print(f"Started socat to create virtual serial ports: {VSERIAL_CLIENT} (for PyGPSClient) and {VSERIAL_PROXY} (for proxy).")
    
    # Wait briefly for links to be created
    time.sleep(1)
    if not os.path.exists(VSERIAL_CLIENT) or not os.path.exists(VSERIAL_PROXY):
        print("Error: Virtual serial ports not created. Check socat installation and permissions.")
        sys.exit(1)
    
    # Open the proxy virtual serial with pyserial
    vser = serial.Serial(VSERIAL_PROXY, baudrate=BAUD_RATE, timeout=0.1)  # Timeout for non-blocking read
    
    # SSH connection to Pi
    ssh = paramiko.SSHClient()
    ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    ssh.connect(PI_IP, username=PI_USERNAME, password=PI_PASSWORD)
    print("Connected to Raspberry Pi via SSH.")
    
    configure_serial(ssh)
    read_stdout, write_stdin = start_pi_channels(ssh)
    
    try:
        # Start forwarding threads
        to_vser_thread = threading.Thread(target=forward_to_vserial, args=(read_stdout, vser, stop_event))
        from_vser_thread = threading.Thread(target=forward_from_vserial, args=(write_stdin, vser, stop_event))
        
        to_vser_thread.start()
        from_vser_thread.start()
        
        # Wait for stop
        to_vser_thread.join()
        from_vser_thread.join()
        
    except KeyboardInterrupt:
        stop_event.set()
        print("\nStopping...")
    except Exception as e:
        print(f"Error: {e}")
    finally:
        stop_event.set()
        vser.close()
        socat_proc.terminate()
        read_stdout.channel.close()
        write_stdin.channel.close()
        ssh.close()
        # Clean up symlinks if needed
        try:
            os.unlink(VSERIAL_CLIENT)
            os.unlink(VSERIAL_PROXY)
        except:
            pass
        print("Connections closed.")

if __name__ == "__main__":
    proxy_serial_to_vserial()
