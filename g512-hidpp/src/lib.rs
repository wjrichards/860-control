//! HID++ 2.0 lighting protocol for the Logitech G512 RGB keyboard.
//!
//! Only the features in [`Feature`] can be addressed. Feature slots are
//! resolved from the device each time it is opened.

mod device;
pub mod discovery;
pub mod effects;
mod error;
#[cfg(any(test, feature = "fake"))]
pub mod fake;
mod feature;
pub mod info;
pub mod perkey;
mod transport;

pub use device::{DEVICE_INDEX, Device, REPLY_TIMEOUT, VERY_LONG_PARAMS};
pub use error::{Error, Result};
pub use feature::Feature;
pub use transport::{Hidraw, Transport};
