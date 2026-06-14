use std::{
    env, fs,
    path::Path,
    process::Command,
    sync::{Mutex, atomic::{AtomicBool, Ordering::Relaxed}},
    thread,
    time::{Duration, Instant},
};

use crossterm::{event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers}, terminal::{disable_raw_mode, enable_raw_mode}};
use defmt_decoder::{
    DecodeError, Frame, Locations, Table,
    log::{
        DefmtLoggerType,
        format::{Formatter, FormatterConfig, HostFormatter},
        init_logger, is_defmt_frame,
    },
};
use serialport::SerialPort;
use usb_defmt_decoder::input::{SHOW_WARNING_UNTIL, STOP, poll_input};

const READ_BUFFER_SIZE: usize = 1024;
const ELF: &str = "/home/Gary/Projects/Embedded/code/Rust/GO/target/thumbv6m-none-eabi/release/go";

fn main() {
    disable_raw_mode().unwrap();

    let mut source: Box<dyn SerialPort> = connect();

    let elf = fs::read(ELF).expect("can't find path");
    let table = Table::parse(&elf).unwrap().unwrap();
    let locations = table.get_locations(&elf).unwrap();

    let locations = if table
        .indices()
        .all(|idx| locations.contains_key(&(idx as u64)))
    {
        Some(locations)
    } else {
        None
    };

    let config = FormatterConfig::custom("{t} {L} {s}").with_timestamp();
    let host_config = FormatterConfig::default().with_location().with_timestamp();
    let formatter = Formatter::new(config);
    let host_formatter = HostFormatter::new(host_config);

    init_logger(
        formatter,
        host_formatter,
        DefmtLoggerType::Stdout,
        is_defmt_frame,
    );

    let mut buf = [0; READ_BUFFER_SIZE];
    let mut stream_decoder = table.new_stream_decoder();
    let current_dir = env::current_dir().unwrap();

    loop {

        poll_input();
        if STOP.load(Relaxed) {
            continue;
        }
        if let Some(until) = *SHOW_WARNING_UNTIL.lock().unwrap() && Instant::now() < until {
            continue;
        }

        // read from stdin or tcpstream and push it to the decoder
        let b = match source.read(&mut buf) {
            Ok(x) => x,
            Err(_) => {
                source = connect();
                println!("disconnected!");
                continue;
            }
        };

        stream_decoder.received(&buf[..b]);

        // decode the received data
        loop {
            match stream_decoder.decode() {
                Ok(frame) => {
                    forward_to_logger(&frame, location_info(&locations, &frame, &current_dir))
                }
                Err(DecodeError::UnexpectedEof) => break,
                Err(DecodeError::Malformed) => match table.encoding().can_recover() {
                    // if recovery is impossible, abort
                    false => break,
                    // if recovery is possible, skip the current frame and continue with new data
                    true => {
                        // bug: https://github.com/rust-lang/rust-clippy/issues/9810
                        #[allow(clippy::print_literal)]
                        // if show_skipped_frames || verbose {
                        //     println!("(HOST) malformed frame skipped");
                        //     println!("└─ {} @ {}:{}", env!("CARGO_PKG_NAME"), file!(), line!());
                        // }
                        continue;
                    }
                },
            }
        }

    }
    disable_raw_mode().unwrap();

}

type LocationInfo = (Option<String>, Option<u32>, Option<String>);

fn forward_to_logger(frame: &Frame, location_info: LocationInfo) {
    let (file, line, mod_path) = location_info;
    defmt_decoder::log::log_defmt(frame, file.as_deref(), line, mod_path.as_deref());
}

fn location_info(locs: &Option<Locations>, frame: &Frame, current_dir: &Path) -> LocationInfo {
    let (mut file, mut line, mut mod_path) = (None, None, None);

    let loc = locs.as_ref().map(|locs| locs.get(&frame.index()));

    if let Some(Some(loc)) = loc {
        // try to get the relative path, else the full one
        let path = loc.file.strip_prefix(current_dir).unwrap_or(&loc.file);

        file = Some(path.display().to_string());
        line = Some(loc.line as u32);
        mod_path = Some(loc.module.clone());
    }

    (file, line, mod_path)
}

static LOOKING_FOR_DEVICE: AtomicBool = AtomicBool::new(false);

fn connect() -> Box<dyn SerialPort + 'static> {
    if !LOOKING_FOR_DEVICE.swap(true, Relaxed) {
        println!("looking for connection...");
    }

    let path = Command::new("bash")
        .arg("-c")
        .arg("ls /dev/ttyACM*")
        .output()
        .expect("failed to run ls");
    let path = String::from_utf8_lossy(&path.stdout);
    // println!("path: {}", path);

    match serialport::new(path.trim(), 115200)
        .timeout(Duration::from_secs(2))
        .open()
    {
        Ok(device) => {
            println!("Connected to {}!", path.trim());
            thread::sleep(Duration::from_millis(1000u64));
            LOOKING_FOR_DEVICE.store(false, Relaxed);

            device
        }
        Err(_) => {
            thread::sleep(Duration::from_millis(500u64));
            connect()
        }
    }
}


