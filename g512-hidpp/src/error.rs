use std::{fmt, io};

use crate::{Feature, effects::EffectKind};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    /// No matching reply within the request timeout.
    Timeout { feature: Feature, function: u8 },
    /// The device answered with a HID++ error report.
    Hidpp { feature: Feature, function: u8, code: u8 },
    /// The device does not implement this feature.
    FeatureMissing(Feature),
    /// The zone does not offer this effect.
    EffectUnavailable(EffectKind),
    /// A speed or intensity value outside the range tested on the keyboard.
    OutOfRange { kind: EffectKind, what: &'static str, value: u16 },
    /// A key usage ID outside the ANSI 104-key set.
    InvalidKey(u8),
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
            Error::EffectUnavailable(kind) => write!(f, "zone does not offer the {} effect", kind.name()),
            Error::OutOfRange { kind, what, value } => {
                write!(f, "{what} {value} is outside the {} range", kind.name())
            }
            Error::InvalidKey(usage) => write!(f, "key {usage:#04x} is not in the ANSI 104-key set"),
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
