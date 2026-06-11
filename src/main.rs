use std::{env, fs, path::Path, time::Duration};

use defmt_decoder::{DecodeError, Frame, Locations, Table, log::{DefmtLoggerType, format::{Formatter, FormatterConfig, HostFormatter}, init_logger, is_defmt_frame}};

const READ_BUFFER_SIZE: usize = 1024;

fn main() {
    let mut source =  serialport::new("/dev/ttyACM0", 11520)
        .timeout(Duration::from_secs(2))
        .open()
        .expect("Failed to open port");

    let elf= fs::read("/home/Gary/Projects/Embedded/code/Rust/testing/target/thumbv6m-none-eabi/release/testing").unwrap();
    let table = Table::parse(&elf).unwrap().unwrap();
    let locations = table.get_locations(&elf).unwrap();

    let locations = if table.indices().all(|idx| locations.contains_key(&(idx as u64))) {
        Some(locations)
    } else {
        None
    };

    let mut config = FormatterConfig::custom("{t} {L} {s}").with_timestamp();
    let mut host_config =  FormatterConfig::default().with_location().with_timestamp();
    let formatter = Formatter::new(config);
    let host_formatter = HostFormatter::new(host_config);
    
    init_logger(formatter, host_formatter, DefmtLoggerType::Stdout, move |metadata| {
        is_defmt_frame(metadata)
    });

    let mut buf = [0; READ_BUFFER_SIZE];
    let mut stream_decoder = table.new_stream_decoder();
    let current_dir = env::current_dir().unwrap();

    loop {
        // read from stdin or tcpstream and push it to the decoder
        let b = source.read(&mut buf).unwrap();

        stream_decoder.received(&buf[..b]);

        // decode the received data
        loop {
            match stream_decoder.decode() {
                Ok(frame) => forward_to_logger(&frame, location_info(&locations, &frame, &current_dir)),
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

