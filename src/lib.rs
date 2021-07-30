#![allow(dead_code)]
pub mod materials;
pub mod methods;
pub mod tool;
pub mod traits;

#[allow(unused_imports)]
use methods::{filter, transform};
#[allow(unused_imports)]
use tool::{reader, renderer};

use materials::{color, coordinate, ply_dir, points};

#[allow(unused_imports)]
use ply_dir::PlyDir;

fn main() {}
