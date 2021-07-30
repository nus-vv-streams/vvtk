#![allow(unused_imports)]
extern crate iswr;

use iswr::methods::{filter, transform};

use std::io::{self, BufRead, Read, Write};

fn main() -> io::Result<()> {
    iswr::tool::reader::read(None).render();

    Ok(())
}
