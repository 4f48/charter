mod error;

use crate::error::{ConfigureWriterError, ParseLineError};
use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::sync::atomic::Ordering;
use tracing::{error, info, Level};

#[derive(Parser)]
struct Args {
    /// Serial port assigned to LoRa receiver
    port: String,
    #[arg(short, long)]
    /// Print debug information
    debug: bool,
    #[arg(short, long, value_name = "FILE")]
    /// Output CSV file
    output: Option<std::path::PathBuf>,
    /// Launch WebSocket server
    #[arg(short, long, value_name = "ADDRESS")]
    websocket: Option<String>,
}

const TELEMETRY_LEN: usize = 28;
type Telemetry = [String; TELEMETRY_LEN];

fn main() -> Result<(), error::MainError> {
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();
    ctrlc::set_handler(move || running_clone.store(false, Ordering::SeqCst))?;

    let args = Args::parse();
    configure_tracing(args.debug)?;
    info!("Starting charter on {}...", args.port);

    let mut serial = initialize_serial(args.port)?;
    serial.write_all(b"radio rxstop")?;
    serial.write_all(b"radio rx 0")?;
    let mut reader = std::io::BufReader::new(&mut serial).lines();
    let mut writer = match args.output {
        Some(path) => Some(configure_writer(path)?),
        None => None,
    };

    while running.load(Ordering::SeqCst) {
        let line = match reader.next() {
            Some(line) => match line {
                Ok(line) => line,
                Err(error) => {
                    error!("{error}");
                    continue;
                }
            },
            None => continue,
        };
        let data = match parse_line(line) {
            Ok(data) => data,
            Err(error) => {
                error!("{error}");
                continue;
            }
        };
        if let Some(writer) = &mut writer {
            writer.write_record(&data)?;
        }
        info!("{:?}", data);
    }

    info!("Stopping charter...");
    serial.write_all(b"radio rxstop")?;
    Ok(())
}

fn configure_tracing(debug: bool) -> Result<(), tracing::dispatcher::SetGlobalDefaultError> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if debug { Level::TRACE } else { Level::INFO })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn configure_writer(
    output: std::path::PathBuf,
) -> Result<csv::Writer<BufWriter<File>>, ConfigureWriterError> {
    if output.is_file() {
        return Err(ConfigureWriterError::NotFile);
    }
    fs::create_dir_all(&output)?;
    let file = BufWriter::new(
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(output.join("primary.csv"))?,
    );
    Ok(csv::Writer::from_writer(file))
}

fn initialize_serial(port: String) -> Result<Box<dyn serialport::SerialPort>, std::io::Error> {
    Ok(serialport::new(port, 115200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(std::time::Duration::from_millis(3000))
        .open()?)
}

fn parse_line(line: String) -> Result<Telemetry, ParseLineError> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(ParseLineError::UnexpectedLine(
            "line does not contain a message or is corrupted",
        ));
    }
    let data = hex::decode(parts[1])?;
    let data = String::from_utf8(data)?;
    let data = data.split_whitespace().collect::<Vec<&str>>();
    let mut telemetry: Telemetry = [const { String::new() }; TELEMETRY_LEN];
    if data.len() != TELEMETRY_LEN {
        return Err(ParseLineError::InvalidDataFormat("unexpected data length"));
    }
    for (index, value) in data.iter().enumerate() {
        telemetry[index] = value.to_string();
    }
    Ok(telemetry)
}
