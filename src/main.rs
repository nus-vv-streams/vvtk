mod color;
mod coordinate;
mod points;
mod ply_file;
mod ply_dir;
mod renderer;

fn main() {
    // ply_dir::PlyDir::new("out").play();

    let data = ply_file::PlyFile::new("longdress_vox10_1051.ply").read();

    ply_file::PlyFile::create("write/1.ply").write_binary(data).unwrap();
}
