use hex::FromHexError;
use std::error::Error;
use std::fmt::Display;
use std::num::ParseFloatError;
use std::string::FromUtf8Error;
use tracing::dispatcher::SetGlobalDefaultError;

#[derive(Debug)]
pub(crate) enum MainError {
    TracingSetGlobalDefault(SetGlobalDefaultError),
    Io(std::io::Error),
    CtrlC(ctrlc::Error),
    ParseLine(ParseLineError),
    ConfigureWriter(ConfigureWriterError),
    Csv(csv::Error),
}
impl Display for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MainError::TracingSetGlobalDefault(error) => {
                write!(f, "Failed to set global default subscriber: {}", error)
            }
            MainError::Io(error) => write!(f, "I/O {} error: {}", error.kind(), error),
            MainError::CtrlC(error) => {
                write!(f, "Failed to set CtrlC handler: {}", error)
            }
            MainError::ParseLine(error) => {
                write!(f, "Failed to parse line: {}", error)
            }
            MainError::ConfigureWriter(error) => {
                write!(f, "Failed to configure a file writer: {}", error)
            }
            MainError::Csv(error) => write!(f, "CSV error: {}", error),
        }
    }
}
impl Error for MainError {}
impl From<SetGlobalDefaultError> for MainError {
    fn from(e: SetGlobalDefaultError) -> Self {
        MainError::TracingSetGlobalDefault(e)
    }
}
impl From<std::io::Error> for MainError {
    fn from(e: std::io::Error) -> Self {
        MainError::Io(e)
    }
}
impl From<ctrlc::Error> for MainError {
    fn from(e: ctrlc::Error) -> Self {
        MainError::CtrlC(e)
    }
}
impl From<ParseLineError> for MainError {
    fn from(e: ParseLineError) -> Self {
        MainError::ParseLine(e)
    }
}
impl From<ConfigureWriterError> for MainError {
    fn from(e: ConfigureWriterError) -> Self {
        MainError::ConfigureWriter(e)
    }
}
impl From<csv::Error> for MainError {
    fn from(e: csv::Error) -> Self {
        MainError::Csv(e)
    }
}

#[derive(Debug)]
pub(crate) enum ParseLineError {
    UnexpectedLine(&'static str),
    InvalidDataFormat(&'static str),
    FromHex(FromHexError),
    FromUtf8(FromUtf8Error),
    ParseFloat(ParseFloatError),
}
impl Display for ParseLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParseLineError::FromHex(error) => {
                write!(f, "Failed to decode hexadecimal string: {}", error)
            }
            ParseLineError::UnexpectedLine(error) => write!(f, "Unexpected line: {}", error),
            ParseLineError::InvalidDataFormat(error) => write!(f, "Invalid data format: {}", error),
            ParseLineError::FromUtf8(error) => {
                write!(f, "Failed to create UTF-8 string from bytes: {}", error)
            }
            ParseLineError::ParseFloat(error) => write!(f, "Failed to parse float: {}", error),
        }
    }
}
impl Error for ParseLineError {}
impl From<FromHexError> for ParseLineError {
    fn from(e: FromHexError) -> Self {
        ParseLineError::FromHex(e)
    }
}
impl From<FromUtf8Error> for ParseLineError {
    fn from(e: FromUtf8Error) -> Self {
        ParseLineError::FromUtf8(e)
    }
}
impl From<ParseFloatError> for ParseLineError {
    fn from(e: ParseFloatError) -> Self {
        ParseLineError::ParseFloat(e)
    }
}

#[derive(Debug)]
pub(crate) enum ConfigureWriterError {
    Io(std::io::Error),
    NotFile,
}
impl Display for ConfigureWriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigureWriterError::Io(error) => write!(f, "I/O {} error: {}", error.kind(), error),
            ConfigureWriterError::NotFile => write!(f, "Not a file"),
        }
    }
}
impl Error for ConfigureWriterError {}
impl From<std::io::Error> for ConfigureWriterError {
    fn from(e: std::io::Error) -> Self {
        ConfigureWriterError::Io(e)
    }
}
