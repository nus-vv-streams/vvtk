//! # in-summer-we-render
//!
//! In summer, we render
//!
//! [Github](https://github.com/hungkhoaitay/in-summer-we-render)

//#![warn(missing_docs)]

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

mod materials;
mod methods;
mod tool;

pub use materials::{
    color, coordinate, interpolate, interpolate_controller, params, ply, ply_dir, point, points,
};
pub use methods::{filter, transform};
pub use ply_dir::PlyDir;
pub use tool::{fat, reader, renderer, writer};

use std::time::Instant;
