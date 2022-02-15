//! # Vivo Toolkit
//#[warn(missing_docs)]

#[macro_use]
extern crate error_chain;

/// Error handling mod
pub mod errors {
    error_chain! {
        foreign_links {
            Io(std::io::Error);
            Float(std::num::ParseFloatError);
            Int(std::num::ParseIntError);
        }
    }
}
pub use errors::*;

#[macro_use]
extern crate approx;

pub mod io;
pub mod pcd;
/// Module handling PLY
pub mod ply;
/// Module handling directory of ply files
pub mod ply_dir;
pub mod point;
pub mod pointcloud;
pub mod processing;
pub mod render;

// re-export
pub use ply_dir::PlyDir;
pub use processing::seq; // interpolate, interpolate_controller};
pub use render::{gui, gui_states, renderer};

// unused
// pub use pointcloud::point;
// pub use seq::{fat, filter, transform};
// pub use io::{reader, writer};

use std::time::Instant;
