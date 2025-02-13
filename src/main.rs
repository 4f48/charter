use anyhow::bail;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::{broadcast, Mutex};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn, Level};

#[derive(clap::Parser)]
struct Args {
    /// Serial port assigned to LoRa receiver
    port: String,

    /// Print debug information
    #[arg(short, long)]
    debug: bool,

    /// Define an output CSV file
    #[arg(short, long, value_name = "DIRECTORY")]
    output: Option<std::path::PathBuf>,

    /// Launch WebSocket server for web panel
    #[arg(short, long, value_name = "ADDRESS")]
    websocket: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct Data([isize; 23]);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    setup_tracing(args.debug)?;
    let serial = Arc::new(Mutex::new(setup_serial(args.port)?));
    let mut writer = match args.output {
        Some(ref output) => Some(setup_writer(output).await?),
        None => None,
    };

    serial
        .clone()
        .lock()
        .await
        .write_all(b"radio rx 0\r\n")
        .await?;

    let (websocket_tx, _websocket_rx) = broadcast::channel::<String>(1024);
    let tx_clone = websocket_tx.clone();
    if let Some(address) = args.websocket.clone() {
        setup_websocket(&address, tx_clone).await?;
    };

    let serial_clone = serial.clone();
    let token = CancellationToken::new();
    let child_token = token.child_token();

    let process = tokio::spawn(async move {
        let mut serial = serial_clone.lock().await;
        let mut reader = tokio::io::BufReader::new(&mut *serial).lines();
        let mut index: usize = 0;
        loop {
            tokio::select! {
                _ = child_token.cancelled() => break,
                Ok(Some(line)) = reader.next_line() => {
                    let data = match parse_line(line) {
                Ok(data) => data,
                Err(error) => {
                    warn!("{error}");
                    continue;
                }
            };
            if let Some(writer) = &mut writer {
                if let Err(error) = write_csv(writer, &data).await {
                    error!("{error}");
                    continue;
                }
            };
            if args.websocket.is_some() {
                match serde_json::to_string(&data) {
                    Ok(data) => {
                        if let Err(error) = websocket_tx.send(data) {
                            error!("{error}");
                        }
                    }
                    Err(error) => error!("{error}"),
                };
            }
            info!("{index} {data:?}");
            index += 1;
                }
            }
        }
    });

    tokio::select! {
        _ = process => {},
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down...");
            token.cancel();
            serial
                .clone()
                .lock()
                .await
                .write_all(b"radio rxstop\r\n")
                .await?;
        }
    }
    Ok(())
}

/// Sets up a tracing subscriber and sets it as default for logging.
/// A boolean is provided for enabling debug logging.
fn setup_tracing(debug: bool) -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if debug { Level::TRACE } else { Level::INFO })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

/// Opens a serial connection on the provided serial port.
fn setup_serial(port: String) -> anyhow::Result<SerialStream> {
    let serial = tokio_serial::new(port, 115200)
        .data_bits(tokio_serial::DataBits::Eight)
        .parity(tokio_serial::Parity::None)
        .stop_bits(tokio_serial::StopBits::One)
        .open_native_async()?;
    Ok(serial)
}

/// Creates a WebSocket server and starts accepting connections
async fn setup_websocket(address: &String, tx: Sender<String>) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(&address).await?;
    info!("Websocket listening on ws://{}", address);
    while let Ok((stream, _)) = listener.accept().await {
        let peer = match stream.peer_addr() {
            Ok(peer) => peer,
            Err(error) => {
                error!("{error}");
                continue;
            }
        };
        debug!("{} initiated connection to WebSocket server...", peer);
        tokio::spawn(accept_connection(peer, stream, tx.subscribe()));
    }
    Ok(())
}

/// Accepts a WebSocket connection and starts sending messages to the client.
async fn accept_connection(
    peer: SocketAddr,
    stream: TcpStream,
    rx: Receiver<String>,
) -> anyhow::Result<()> {
    handle_connection(peer, stream, rx).await?;
    Ok(())
}

/// Sends messages to the WebSocket client.
async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    mut rx: Receiver<String>,
) -> anyhow::Result<()> {
    let ws_stream = accept_async(stream).await?;
    debug!("{peer} connected via WebSocket");
    let (mut ws_sender, _ws_receiver) = ws_stream.split();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(data) => {
                    ws_sender
                        .send(Message::from(data))
                        .await
                        .map_err(|error| error!("{error}"))
                        .unwrap();
                }
                Err(error) => error!("{error}"),
            }
        }
    });
    Ok(())
}

/// Creates a file writer for writing CSV data to the given file.
async fn setup_writer(
    file: &std::path::Path,
) -> anyhow::Result<tokio::io::BufWriter<tokio::fs::File>> {
    let writer = tokio::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file)
        .await?;
    let writer = tokio::io::BufWriter::new(writer);
    Ok(writer)
}

/// Parses a line from WLR089 LoRa module by extracting the message,
/// converting the HEX to UTF-8, and splitting the data into an array of integers.
fn parse_line(line: String) -> anyhow::Result<Data> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 2 {
        debug!("Unexpected line: {line}");
        bail!("Unexpected line");
    }
    let data_bytes = hex::decode(parts[1])?;
    let data_str = String::from_utf8(data_bytes)?;

    let mut data_array: [isize; 23] = [0; 23];
    for (index, value) in data_str.split_whitespace().enumerate() {
        data_array[index] = value.parse()?;
    }
    Ok(Data(data_array))
}

/// Writes a line to the writer in CSV format.
async fn write_csv(
    writer: &mut tokio::io::BufWriter<tokio::fs::File>,
    data: &Data,
) -> anyhow::Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    wtr.serialize(data)?;
    let bytes = wtr.into_inner()?;
    writer.write_all(&bytes).await?;
    Ok(())
}
