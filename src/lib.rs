#[macro_use]
extern crate error_chain;

pub mod errors {
    error_chain! {
        foreign_links {
            Io(std::io::Error);
            Num(std::num::ParseFloatError);
        }
    }
}
pub use errors::*;

mod materials;
mod methods;
mod tool;

pub use materials::{color, coordinate, ply_dir, points, params};
pub use methods::{filter, transform};
pub use ply_dir::PlyDir;
pub use tool::{reader, renderer};

use std::time::{Duration, Instant};