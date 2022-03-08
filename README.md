# The Vivo Toolkit (VivoTk)

![format badge](https://github.com/Hungkhoaitay/in-summer-we-render/actions/workflows/format.yml/badge.svg)
![build badge](https://github.com/Hungkhoaitay/in-summer-we-render/actions/workflows/build.yml/badge.svg)

### Rust version

Use Rust 1.58.1

### Coding Style

We follow the [official Rust coding style](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md).  You can use `rustfmt` (or run `cargo fmt`) to automatically format your code.

## Binaries

### `ply_to_pcd`

Converts ply files that are in Point_XYZRGBA format to pcd files

```shell
Converts ply files that are in Point_XYZRGBA format to pcd files

This assumes that the given ply files contain vertices and that the vertices are the first field in
the ply file.

USAGE:
    ply_to_pcd.exe [OPTIONS] --output-dir <OUTPUT_DIR> [FILES]...

ARGS:
    <FILES>...
            Files, glob patterns, directories

OPTIONS:
    -h, --help
            Print help information

    -o, --output-dir <OUTPUT_DIR>


    -s, --storage-type <STORAGE_TYPE>
            Storage type can be either "ascii" or "binary" [default: binary]
```

### Example

The following command will convert all `.ply` files in the `./plys/` directory to binary `.pcd` format and place them in `./converted_pcds/`.

```shell
ply_to_pcd -o ./converted_pcds -s binary ./plys/*
```