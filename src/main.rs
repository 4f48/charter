use clap::Parser;
use plotters::prelude::*;
use serialport::{DataBits, Parity, SerialPort, StopBits};
use std::error::Error;
use std::fmt::{Display, Formatter, Write};
use std::io::{ErrorKind, Read, Write as IoWrite};
use std::process::exit;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
struct Args {
    port: String,
}

fn main() {
    let args = Args::parse();
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
        serial_end(&mut serial_clone);
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    })
    .expect("Failed to set Ctrl-C handler");

    let mut serial_buf: Vec<u8> = vec![0; 1024];
    let mut line_buf = String::new();
    let mut index: usize = 0;
    while running.load(std::sync::atomic::Ordering::SeqCst) {
        match serial.read(serial_buf.as_mut_slice()) {
            Ok(n) => {
                let str = String::from_utf8(Vec::from(&serial_buf[..n])).unwrap();
                line_buf.write_str(&str).unwrap();
                while let Some(pos) = line_buf.find("\r\n") {
                    let line = line_buf[..pos].trim_end().to_string();
                    line_buf.clear();
                    match get_data(line) {
                        Ok(data) => {
                            if let Ok(data) = parse_data(data) {
                                index += 1;
                                create_chart(&data, format!("chart{}.svg", index));
                            };
                        }
                        Err(error) => println!("{error}"),
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
    println!("Starting serial communication...");
    serial.write_all("radio rx 0\r\n".as_bytes())?;
    Ok(Arc::new(AtomicBool::new(true)))
}

fn serial_end(serial: &mut Box<dyn SerialPort>) {
    println!("Stopping Serial communication...");
    serial.write_all("radio rxstop\r\n".as_bytes()).unwrap();
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
        return Err(Box::new(GetDataError::IrregularMessage(
            "this line doesn't contain any data",
        )));
    };
    let data = match message.nth(1) {
        Some(data) => data,
        None => {
            return Err(Box::new(GetDataError::ParseError(
                "failed to retrieve data",
            )))
        }
    };
    Ok(String::from_utf8(hex::decode(data)?)?)
}

fn parse_data(line: String) -> Result<[usize; 11], Box<dyn Error>> {
    let mut data: [usize; 11] = [0; 11];
    for (index, item) in line.split_whitespace().take(11).enumerate() {
        data[index] = item.parse()?;
    }
    Ok(data)
}

fn create_chart(data: &[usize; 11], filename: String) {
    let root = SVGBackend::new(filename.as_str(), (1000, 550)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(35)
        .y_label_area_size(40)
        .margin(5)
        .caption("Spectrophotometer", ("sans-serif", 30.0))
        .build_cartesian_2d(0usize..11usize, 0usize..*data.iter().max().unwrap())
        .unwrap();
    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(WHITE.mix(0.3))
        .y_desc("Intensity")
        .x_desc("Channels")
        .axis_desc_style(("sans-serif", 15))
        .draw()
        .unwrap();
    chart
        .draw_series(
            Histogram::vertical(&chart)
                .style(RED.mix(0.5).filled())
                .data(data.iter().enumerate().map(|(index, item)| (index, *item)))
                .step_by(1),
        )
        .unwrap();
    root.present().unwrap();
}
