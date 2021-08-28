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

mod materials;
mod methods;
mod tool;

pub use materials::{color, coordinate, params, ply_dir, points};
pub use methods::{filter, transform};
pub use ply_dir::PlyDir;
pub use tool::{reader, renderer};

use std::time::Instant;
