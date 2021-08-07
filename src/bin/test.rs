extern crate iswr;

#[macro_use]
extern crate error_chain;

use iswr::{errors::*};

quick_main!(run);

fn run() -> Result<()> {
    Ok(())
}
