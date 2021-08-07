# in-summer-we-render

![format badge](https://github.com/Hungkhoaitay/in-summer-we-render/actions/workflows/format.yml/badge.svg)
![build badge](https://github.com/Hungkhoaitay/in-summer-we-render/actions/workflows/build.yml/badge.svg)

### Switching to nightly build

To speed up kd-trees, we use `kiddo`, which requires nighthly build.  To make rust nightly build the default:
```
rustup default nightly
```

To install nightly version of the tools:
```
rustup toolchain install nightly
```

### Coding Style
We follow the [official Rust coding style](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md).  You can use `rustfmt` (or run `cargo fmt`) to automatically formt your code.


``` {.}
cargo run --release --bin ply_fat -- -i plySource/binary_ply/longdress_vox10_1051.ply --filter=upper_half -t=all_green -r=all_red
```

``` {.}
cargo run --release --bin ply_to_ply -- -i plySource/binary_ply/longdress_vox10_1051.ply -f binary -o hello.ply
```

``` {.}
cargo run --release --bin ply_view -- -i plySource/binary_ply/longdress_vox10_1051.ply --eye=100,100,100
```






