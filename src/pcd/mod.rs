//! Point Cloud Data (PCD)
//!
//! This module provides a simple representation of the [.pcd format](https://pcl.readthedocs.io/projects/tutorials/en/master/pcd_file_format.html#pcd-file-format)
//!
//! This module only supports version 0.7 of the PCD file format.
//!
//! # Examples
//!
//! ## Reading from a file
//! ```no_run
//! use vivotk::pcd::{PCDReadError, read_pcd_file};
//!
//! fn main() -> Result<(), PCDReadError> {
//!     let file_pcd = read_pcd_file("example.pcd")?;
//!     println!("{}", file_pcd.data().len());
//!     Ok(())
//! }
//! ```
//!
//! ## Writing to a file
//! ```no_run
//! use vivotk::pcd::{write_pcd_file, read_pcd_file, PCDReadError, PCDDataType};
//!
//! fn main() -> Result<(), PCDReadError> {
//!     let file_pcd = read_pcd_file("example.pcd")?;
//!     write_pcd_file(&file_pcd, PCDDataType::Ascii, "new.pcd");
//!
//!     write_pcd_file(&file_pcd, PCDDataType::Binary, "new_binary.pcd");
//!     Ok(())
//! }
//! ```

mod data_types;
mod reader;
mod writer;

pub use data_types::*;
pub use reader::{read_pcd, read_pcd_file, read_pcd_header, PCDReadError};
pub use writer::{create_pcd, write_pcd, write_pcd_file};
