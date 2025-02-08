use clap::Parser;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio_serial::{DataBits, Parity, SerialPortBuilderExt, StopBits};
use tracing::{error, info, Level};

use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Utf8Bytes;

#[derive(Parser)]
struct Args {
    /// Serial port assigned to LoRa receiver
    port: String,

    /// Print debug information
    #[arg(short, long)]
    debug: bool,

    /// Define an output CSV file
    #[arg(short, long)]
    output: Option<String>,

    /// Launch WebSocket server with address
    #[arg(short, long)]
    websocket: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct Data([String; 22]);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(if args.debug {
            Level::TRACE
        } else {
            Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let mut serial = tokio_serial::new(&args.port, 115200)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .open_native_async()?;

    let mut writer = match args.output {
        Some(ref path) => Some(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
                .await?,
        ),
        None => None,
    };

    let (broadcast_tx, _broadcast_rx) = broadcast::channel(100);

    if let Some(address) = args.websocket {
        let ws_broadcast_tx = broadcast_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = websocket(ws_broadcast_tx, &address).await {
                error!("WebSocket server error: {}", e);
            }
        });
    }

    // Tell the serial device to start “radio rx”.
    serial.write_all(b"radio rx 0\r\n").await?;

    // Process incoming serial data.
    // Each line is parsed; CSV is optionally written, and the parsed data
    // is broadcast to all connected WebSocket clients.
    let broadcast_tx_clone = broadcast_tx.clone();
    let process = tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(&mut serial).lines();
        let mut index = 0;
        while let Ok(Some(line)) = reader.next_line().await {
            match parse_line(&line) {
                Ok(data) => {
                    // If CSV output is enabled, write data.
                    if let Some(ref mut file) = writer {
                        if let Err(e) = write_csv(file, &data).await {
                            error!("CSV write error: {}", e);
                        }
                    }
                    // If WebSocket broadcasting is enabled, send JSON.
                    // (Even if no client is connected, broadcast_tx.send() works fine.)
                    if broadcast_tx_clone.receiver_count() > 0 {
                        match serde_json::to_string(&data) {
                            Ok(json_msg) => {
                                let _ = broadcast_tx_clone.send(json_msg);
                            }
                            Err(e) => error!("JSON serialization error: {}", e),
                        }
                    }
                    info!("{}: {:?}", index, data);
                    index += 1;
                }
                Err(e) => {
                    tracing::warn!("Parse error for line {}: {}", line, e);
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    // Wait for either the processing task to finish or a CTRL+C signal.
    tokio::select! {
        _ = process => {},
        _ = tokio::signal::ctrl_c() => {
            info!("Shutting down...");
        }
    }
    Ok(())
}

/// Parses a given line from the serial port into a Data struct.
/// The line is expected to contain two whitespace‐separated parts,
/// with the second being a hexadecimal string to decode.
fn parse_line(line: &str) -> anyhow::Result<Data> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 2 {
        anyhow::bail!("This line does not contain data");
    }
    let decoded = hex::decode(parts[1])?;
    let data_str = String::from_utf8(decoded)?;
    let mut data_arr = [const { String::new() }; 22];
    // Zip over the array and the whitespace‑split decoded string.
    for (dst, src) in data_arr.iter_mut().zip(data_str.split_whitespace()) {
        *dst = src.to_string();
    }
    Ok(Data(data_arr))
}

/// Serializes the Data and writes it as CSV to the given file.
async fn write_csv(writer: &mut tokio::fs::File, data: &Data) -> anyhow::Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(vec![]);
    wtr.serialize(data)?;
    let bytes = wtr.into_inner()?;
    writer.write_all(&bytes).await?;
    Ok(())
}

/// Launches a simple WebSocket server.
/// For every new connection, a subscription to the broadcast channel is created;
/// messages received on the channel are sent to the client.
async fn websocket(
    broadcast_tx: broadcast::Sender<String>,
    address: &String,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(address).await?;
    info!("WebSocket server listening on ws://{}", address);

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        info!("New WebSocket connection from {}", peer_addr);
        let tx = broadcast_tx.clone();
        tokio::spawn(async move {
            // Complete the WebSocket handshake.
            let ws_stream = accept_async(stream).await;
            if let Err(e) = ws_stream {
                error!("WebSocket handshake error: {}", e);
                return;
            }
            let mut ws_stream = ws_stream.unwrap();
            // For each client, subscribe to the broadcast channel.
            let mut rx = tx.subscribe();
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        // Send the broadcast message to the client.
                        if ws_stream
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                Utf8Bytes::from(msg),
                            ))
                            .await
                            .is_err()
                        {
                            info!("Client {} disconnected", peer_addr);
                            break;
                        }
                    }
                    // Skip messages if the receiver lags behind.
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}
