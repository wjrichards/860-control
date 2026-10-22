use std::{fmt, io};

use crate::Feature;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// No matching reply within the request timeout.
    Timeout { feature: Feature, function: u8 },
    /// The device answered with a HID++ error report.
    Hidpp { feature: Feature, function: u8, code: u8 },
    /// The device does not implement this feature.
    FeatureMissing(Feature),
    /// The device replied with data this crate cannot interpret.
    Protocol(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Timeout { feature, function } => {
                write!(f, "no reply from {feature} function {function}")
            }
            Error::Hidpp { feature, function, code } => {
                write!(f, "HID++ error {code:#04x} from {feature} function {function}")
            }
            Error::FeatureMissing(feature) => write!(f, "device does not support {feature}"),
            Error::Protocol(msg) => write!(f, "unexpected reply: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}
