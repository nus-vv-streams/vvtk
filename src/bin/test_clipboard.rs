extern crate clipboard;
extern crate vivotk;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

#[macro_use]
extern crate error_chain;

use vivotk::errors::*;

quick_main!(run);

fn run() -> Result<()> {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    println!("{:?}", ctx.get_contents().unwrap());
    ctx.set_contents("some string".to_owned()).unwrap();
    Ok(())
}
