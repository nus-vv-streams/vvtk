extern crate clipboard;
extern crate iswr;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

#[macro_use]
extern crate error_chain;

use iswr::errors::*;

quick_main!(run);

fn run() -> Result<()> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    println!("{:?}", ctx.get_contents().unwrap());
    ctx.set_contents("some string".to_owned()).unwrap();
    Ok(())
}
