//! HID++ 2.0 protocol for the Logitech G512 RGB keyboard and G502 HERO mouse:
//! lighting, and the mouse's onboard profiles.
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
pub mod profiles;
mod transport;

pub use device::{DEVICE_INDEX, Device, REPLY_TIMEOUT, VERY_LONG_PARAMS};
pub use error::{Error, Result};
pub use feature::Feature;
pub use transport::{Hidraw, Transport};
