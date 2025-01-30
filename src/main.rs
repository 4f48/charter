use clap::Parser;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio_serial::{DataBits, Parity, SerialPortBuilderExt, StopBits};
use tracing::Level;

#[derive(clap::Parser)]
struct Args {
    /// Serial port assigned to LoRa receiver
    port: String,

    /// Print debug information
    #[arg(short, long)]
    debug: bool,

    /// Define an output CSV file
    #[arg(short, long)]
    output: Option<std::path::PathBuf>,
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
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let mut writer = match args.output {
        Some(path) => Some(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&path)
                .await?,
        ),
        None => None,
    };

    serial.write_all(b"radio rx 0\r\n").await?;

    let process: tokio::task::JoinHandle<anyhow::Result<tokio_serial::SerialStream>> =
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(&mut serial).lines();
            let mut index = 0;
            while !*shutdown_rx.borrow() {
                let line = match reader.next_line().await {
                    Ok(Some(line)) => line,
                    _ => continue,
                };

                match parse_line(&line) {
                    Ok(data) => {
                        if let Some(writer) = &mut writer {
                            if let Err(error) = write_csv(writer, &data).await {
                                tracing::error!("{error}");
                            }
                        }
                        tracing::info!("{index}: {data:?}");
                        index += 1;
                    }
                    Err(error) => tracing::warn!("{error}"),
                }
            }
            Ok(serial)
        });

    tokio::signal::ctrl_c().await?;
    shutdown_tx.send(true)?;
    process.await??.write_all(b"radio rx 0\r\n").await?;
    Ok(())
}

fn parse_line(line: &str) -> anyhow::Result<Data> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 2 {
        anyhow::bail!("This line does not contain data");
    }

    let decoded = hex::decode(parts[1])?;
    let data_str = String::from_utf8(decoded)?;

    let mut data = [const { String::new() }; 22];
    data.iter_mut()
        .zip(data_str.split_whitespace())
        .for_each(|(dst, src)| *dst = src.to_string());

    Ok(Data(data))
}

async fn write_csv(writer: &mut tokio::fs::File, data: &Data) -> anyhow::Result<()> {
    let mut wtr = csv::WriterBuilder::new().from_writer(vec![]);
    wtr.serialize(data)?;
    writer.write_all(&wtr.into_inner()?).await?;
    Ok(())
}
