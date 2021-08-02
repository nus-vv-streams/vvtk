#![allow(unused_imports)]
extern crate iswr;

use iswr::{filter, transform, reader};

use std::io::{self, BufRead, Read, Write};

fn main() -> io::Result<()> {
    let path = "plySource/binary_ply";

    let data_1051 =
        reader::read(Some(&(path.to_owned() + "/longdress_vox10_1051.ply")));

    // data_1051
    //     .fat(
    //         &filter::upper_half(),
    //         &transform::all_green(),
    //         &transform::do_nothing(),
    //     )
    //     .render();

    // iswr::materials::ply_file::PlyFile::writen_as_ascii_to_stdout(data_1051)?;

    // Ok(())

    data_1051.write(Some("ascii"), None)
}
