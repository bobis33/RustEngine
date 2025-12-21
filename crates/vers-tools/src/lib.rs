//! # `vers-tools`
//!
//! ## Example
//!
//! ```
//! use vers_tools::SystemInfo;
//!
//! let info = SystemInfo::new();
//! println!("{}", info.os_arch);
//! ```

mod system_info;

pub use system_info::SystemInfo;
pub use system_info::error::SystemError;
