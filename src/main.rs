use clap::Parser;
use csv::Writer;
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::backtrace;
use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt::{Display, Formatter, Write};
use std::fs::File;
use std::io::{BufWriter, ErrorKind, Read, Write as IoWrite};
use std::path::PathBuf;
use std::process::exit;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
struct Args {
    /// Serial port assigned to LoRa receiver
    port: String,
    #[arg(short, long)]
    /// Log debug information
    debug: bool,
    #[arg(short, long)]
    /// CSV file name to print data to
    output: Option<PathBuf>,
    #[arg(short, long)]
    /// Allow the creation of a new CSV file
    create: bool,
}

fn main() {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(if args.debug {
            Level::TRACE
        } else {
            Level::INFO
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    std::panic::set_hook(Box::new(|panic| {
        let trace = Backtrace::capture();
        if trace.status() == backtrace::BacktraceStatus::Disabled {
            error!("{panic}");
        } else {
            error!("{panic} {trace}");
        }
        exit(1);
    }));

    let mut serial = serialport::new(&args.port, 115200)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .timeout(Duration::from_millis(1000))
        .open()
        .unwrap_or_else(|error| panic!("Failed to open {}: {:?}", &args.port, error.kind()));

    let running = serial_begin(&mut serial).expect("Failed to start communication");
    let r = running.clone();
    let mut serial_clone = serial.try_clone().unwrap();
    ctrlc::set_handler(move || {
        serial_end(&mut serial_clone).unwrap();
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl-C handler");

    let mut serial_buf: Vec<u8> = vec![0; 1024];
    let mut line_buf = String::new();
    let mut index: usize = 0;
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        match serial.read(serial_buf.as_mut_slice()) {
            Ok(n) => {
                let str = match String::from_utf8(Vec::from(&serial_buf[..n])) {
                    Ok(str) => str,
                    Err(error) => {
                        error!("{error}");
                        continue;
                    }
                };
                line_buf.write_str(&str).unwrap();
                while let Some(pos) = line_buf.find("\r\n") {
                    let line = line_buf[..pos].trim_end().to_string();
                    line_buf.clear();
                    match get_data(line) {
                        Ok(data) => {
                            if let Ok(data) = parse_data(data) {
                                match args.output {
                                    Some(ref output) => {
                                        match write_csv(&data, output, args.create) {
                                            Ok(_) => debug!(
                                                "Written {:?} to {} ({})",
                                                &data,
                                                &output.display(),
                                                index
                                            ),
                                            Err(error) => {
                                                if let Some(io_error) =
                                                    error.downcast_ref::<std::io::Error>()
                                                {
                                                    match io_error.kind() {
                                                        ErrorKind::NotFound => panic!("{error}"),
                                                        _ => error!("{error}"),
                                                    }
                                                } else {
                                                    error!("{error}");
                                                }
                                            }
                                        };
                                    }
                                    None => info!("{index}: {data:?}"),
                                }

                                index += 1;
                            };
                        }
                        Err(error) => tracing::warn!("{error}"),
                    };
                }
            }
            Err(ref error) if error.kind() == ErrorKind::TimedOut => (),
            Err(ref error) if error.kind() == ErrorKind::Interrupted => {
                exit(0);
            }
            Err(error) => panic!("{}", error),
        }
    }
}

fn serial_begin(serial: &mut Box<dyn SerialPort>) -> Result<Arc<AtomicBool>, serialport::Error> {
    info!("Starting serial communication...");
    serial.write_all("radio rx 0\r\n".as_bytes())?;
    Ok(Arc::new(AtomicBool::new(true)))
}

fn serial_end(serial: &mut Box<dyn SerialPort>) -> Result<(), serialport::Error> {
    Ok(serial.write_all("radio rxstop\r\n".as_bytes())?)
}

#[derive(Debug)]
enum GetDataError {
    IrregularMessage(&'static str),
    ParseError(&'static str),
}

impl Display for GetDataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GetDataError::IrregularMessage(msg) => write!(f, "Irregular message: {}", msg),
            GetDataError::ParseError(msg) => write!(f, "Error while parsing data: {}", msg),
        }
    }
}

impl Error for GetDataError {}

fn get_data(line: String) -> Result<String, Box<dyn Error>> {
    let mut message = line.split_whitespace();
    if message.clone().count() != 2 {
        debug!("{line}");
        return Err(Box::new(GetDataError::IrregularMessage(
            "this line doesn't contain any data",
        )));
    };
    let data = match message.nth(1) {
        Some(data) => data,
        None => {
            debug!("{line}");
            return Err(Box::new(GetDataError::ParseError(
                "failed to retrieve data",
            )));
        }
    };
    Ok(String::from_utf8(hex::decode(data)?)?)
}

fn parse_data(line: String) -> Result<[String; 11], Box<dyn Error>> {
    let mut data: [String; 11] = [const { String::new() }; 11];
    for (index, item) in line.split_whitespace().take(11).enumerate() {
        data[index] = item.to_string();
    }
    Ok(data)
}

fn write_csv(data: &[String; 11], path: &PathBuf, create: bool) -> Result<(), Box<dyn Error>> {
    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(create)
        .open(path)?;

    let buf_writer = BufWriter::new(file);
    let mut writer = Writer::from_writer(buf_writer);
    writer.write_record(data)?;
    writer.flush()?;
    Ok(())
}
