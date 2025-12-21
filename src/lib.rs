//! # `rust_engine`
//!
//! Fournit des informations système cross-platform (Linux, macOS, Windows).
//!
//! ## Exemple
//!
//! ```
//! use rust_engine::SystemInfo;
//!
//! let info = SystemInfo::new();
//! println!("{}", info.os);
//! ```

mod error;
mod system;

pub use error::SystemError;
pub use system::SystemInfo;
