mod error;

use crate::error::{ConfigureWriterError, MainError, ParseLineError};
use clap::Parser;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use tracing::{error, info, Level};
use tungstenite::{accept, WebSocket};

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
    #[arg(short, long, value_name = "PORT")]
    websocket: Option<String>,
}

const TELEMETRY_LEN: usize = 28;
type Telemetry = [String; TELEMETRY_LEN];

type Websocket = Result<Option<Arc<Mutex<Vec<WebSocket<TcpStream>>>>>, MainError>;

fn main() -> Result<(), MainError> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    ctrlc::set_handler(move || running_clone.store(false, Ordering::SeqCst))?;

    let args = Args::parse();
    configure_tracing(args.debug)?;
    info!("Starting charter on {}...", args.port);

    let mut serial = initialize_serial(&args.port)?;
    let mut serial_clone = serial.try_clone().unwrap();
    serial.write_all(b"radio rxstop")?;
    serial.write_all(b"radio rx 0")?;
    let mut reader = std::io::BufReader::new(serial);

    let mut writer = match args.output {
        Some(ref path) => Some(configure_writer(path)?),
        None => None,
    };

    let websocket = initialize_websocket(&running, &args)?;

    while running.load(Ordering::SeqCst) {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => (),
            Err(error) => {
                error!("{error}");
                continue
            },
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

        if let Some(clients) = &websocket {
            let json = match serde_json::to_string(&data) {
                Ok(json) => json,
                Err(error) => {
                    error!("{error}");
                    continue;
                }
            };
            let mut clients = match clients.lock() {
                Ok(clients) => clients,
                Err(error) => {
                    error!("{error}");
                    continue;
                }
            };
            clients.retain_mut(|websocket| {
                match websocket.send(tungstenite::Message::Text(json.clone().into())) {
                    Ok(_) => true,
                    Err(error) => {
                        error!("{error}");
                        false
                    }
                }
            });
        }

        info!("{:?}", data);
    }

    info!("Stopping charter...");
    serial_clone.write_all(b"radio rxstop")?;
    Ok(())
}

fn initialize_websocket(running: &Arc<AtomicBool>, args: &Args) -> Websocket {
    let websocket = match &args.websocket {
        Some(port) => {
            let clients = Arc::new(Mutex::new(Vec::new()));
            let websocket_clients = clients.clone();

            let server = std::net::TcpListener::bind(format!("0.0.0.0:{port}"))?;
            server.set_nonblocking(true)?;
            info!("WebSocket server listening on 0.0.0.0:{port}...");

            let running = running.clone();
            spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match server.accept() {
                        Ok((stream, address)) => {
                            let clients = clients.clone();
                            spawn(move || match accept(stream) {
                                Ok(stream) => {
                                    let mut clients = clients.lock().unwrap();
                                    clients.push(stream);
                                    info!("New WebSocket connection from {address}");
                                }
                                Err(error) => error!("{error}"),
                            });
                        }
                        Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            continue
                        }
                        Err(error) => error!("{error}"),
                    }
                }
            });
            Some(websocket_clients)
        }
        None => None,
    };
    Ok(websocket)
}

fn configure_tracing(debug: bool) -> Result<(), tracing::dispatcher::SetGlobalDefaultError> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if debug { Level::TRACE } else { Level::INFO })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn configure_writer(
    output: &std::path::PathBuf,
) -> Result<csv::Writer<BufWriter<File>>, ConfigureWriterError> {
    if output.is_file() {
        return Err(ConfigureWriterError::NotFile);
    }
    let file = BufWriter::new(
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(output)?,
    );
    Ok(csv::Writer::from_writer(file))
}

fn initialize_serial(port: &String) -> Result<Box<dyn serialport::SerialPort>, std::io::Error> {
    Ok(serialport::new(port, 115200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(std::time::Duration::from_secs(1))
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
