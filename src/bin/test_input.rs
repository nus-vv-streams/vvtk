#![allow(unused_imports)]
extern crate iswr;

use iswr::{filter, transform, reader};

use std::io::{self, BufRead, Read, Write};

fn main() -> io::Result<()> {
    reader::read(None).render();

    Ok(())
}
