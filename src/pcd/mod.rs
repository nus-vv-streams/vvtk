//! Point Cloud Data (PCD)
//!
//! This module provides a simple representation of the [.pcd format](https://pcl.readthedocs.io/projects/tutorials/en/master/pcd_file_format.html#pcd-file-format)
//!
//! This module only supports version 0.7 of the PCD file format.
//!
//! # Examples
//!
//! ```no_run
//! use vivotk::pcd::{PCDReadError, read_pcd};
//!
//! fn main() -> Result<(), PCDReadError> {
//!     let reader_pcd = read_pcd("VERSION .7 ...".as_bytes())?;
//!     println!("{}", reader_pcd.data().len());
//!     Ok(())
//! }
//! ```
//!
//! ```no_run
//! use vivotk::pcd::{PCDReadError, read_pcd_file};
//!
//! fn main() -> Result<(), PCDReadError> {
//!     let file_pcd = read_pcd_file("example.pcd")?;
//!     println!("{}", file_pcd.data().len());
//!     Ok(())
//! }
//! ```

mod data_types;
mod reader;

pub use data_types::*;
pub use reader::{read_pcd, read_pcd_file, PCDReadError};
