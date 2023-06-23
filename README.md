# VVTk: A Toolkit for Volumetric Video Researchers

![format badge](https://github.com/nus-vv-streams/vivotk/actions/workflows/format.yml/badge.svg)
![build badge](https://github.com/nus-vv-streams/vivotk/actions/workflows/build.yml/badge.svg)


## How to Install?

1. Install the latest Rust compiler from the [official website](https://www.rust-lang.org/tools/install)
2. Verify if `cargo` and `rustc` have been installed successfully using `cargo --version` and `rustc --version`
3. If you are using **linux**, make sure `gcc`, `g++`, `cmake`, `libssl-dev`, `pkg-config`, `libfontconfig1-dev` are installed
4. Compile and build the binaries with `cargo build --release --bins`
5. Install the binaries if you want to use it anywhere you want. `cargo install --path .`
6. Use `vv` and `vvplay` in other directory. Now you are good to go!
7. Download the [8i_dataset](https://plenodb.jpeg.org/pc/8ilabs/) to use and test our tool!

## Commands

### `vv`

Provides subcommands that can be chained together. The inputs and outputs of a subcommand must be specified with the `+input=` or `+in` followed by a comma separated list of inputs or `+output=` or `+out` to denote the name of its output stream. Note that `+input` must be specified for commands other than `read`. 

```shell
Usage: vv <COMMAND>

Commands:
  convert     Converts a pointcloud file from one format to another.
                  Supported formats are .pcd and .ply.
                  Supported storage types are binary and ascii.
  write       Writes from input stream into a file, input stream can be pointcloud data or metrics
  read        Reads in one of our supported file formats. 
                  Files can be of the type .pcd .ply. 
                  The path can be a file path or a directory path contains these files.
  render      Writes point clouds from the input stream into images
  metrics     Calculates the metrics given two input streams.
                  First input stream is the original.
                  Second is the reconstructed.
                  Then uses write command to write the metrics into a text file.
  downsample  Downsample a pointcloud from the stream
  upsample    Upsamples a pointcloud from the stream
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

**Example**

```shell
vv read ./ply_ascii +output=ply_a \
        write --output-format pcd --storage-type binary \
        ./pcd_binary +input=ply_a
```

Alternatively, you can use `+in` and `+out` as a shortcut to `+input` and `+output`.

```shell
vv read ./ply_ascii +out=ply_a \
        write --output-format pcd --storage-type binary \
        ./pcd_binary +in=ply_a
```

#### `read`

Reads in one of our supported file formats. Files can be of the type `.pcd` `.ply`. The path can be a file path or a directory path contains these files.

```shell
Usage: read [OPTIONS] [FILES]...

Arguments:
  [FILES]...  Files, glob patterns, directories

Options:
  -t, --filetype <FILETYPE>  [default: all] [possible values: all, ply, pcd]
  -h, --help                 Print help
```

```shell
vv read ./Ply +output=plys
```

#### `render`

Writes point clouds from the input stream into images.

```shell
Usage: render [OPTIONS] <OUTPUT_DIR> 

Arguments:
  <OUTPUT_DIR>  Directory to store output png images

Options:
  -x, --camera-x <CAMERA_X>        [default: 0]
  -y, --camera-y <CAMERA_Y>        [default: 0]
  -z, --camera-z <CAMERA_Z>        [default: 1.3]
      --yaw <CAMERA_YAW>           [default: -90]
      --pitch <CAMERA_PITCH>       [default: 0]
      --width <WIDTH>              [default: 1600]
      --height <HEIGHT>            [default: 900]
      --name-length <NAME_LENGTH>  [default: 5]
  -h, --help                       Print help
```

**render example**

```shell
vv read ./Ply +output=plys \
        render ./Pngs +input=plys
```

#### `metrics`

Calculates the metrics given two input streams where the first input stream is the original and the second is the reconstructed one. Then uses `write` command to write the metrics into a text file.

```shell
Usage: metrics [OPTIONS]

Options:
  -m, --metric <METRIC>  [default: psnr] [possible values: psnr]
  -h, --help             Print help
```

```shell
vv read ./original +output=original \
        read ./reconstructed +output=reconstructed \
        metrics +input=original,reconstructed +output=metrics \
        write ./metrics +input=metrics
```

#### `write`

Writes from input stream into a file, input stream can be pointcloud data or metrics

```shell
Usage: write [OPTIONS] <OUTPUT_DIR>

Arguments:
  <OUTPUT_DIR>  output directory to store point cloud files or metrics

Options:
      --output-format <OUTPUT_FORMAT>  [default: pcd]
  -s, --storage-type <STORAGE_TYPE>    [default: binary]
      --name-length <NAME_LENGTH>      [default: 5]
  -h, --help                           Print help
```

**Writing metrics**

```shell
vv read ./original +output=original \
        read ./reconstructed +output=reconstructed \
        metrics +input=original,reconstructed +output=metrics \
        write ./metrics +input=metrics 
```

#### `upsample`

Upsamples a point cloud.

```shell
Usage: upsample --factor <FACTOR>

Options:
  -f, --factor <FACTOR>  
  -h, --help             Print help
```

**Upsampling a file**

Upsamples pcd files and write as ply binary

```shell
vv read ./pcd +output=pcdb \
       upsample --factor 2 +input=pcdb +output=pcdb_up \
       write ./pcd_up \
             +input=pcdb_up \
             --storage-type binary \
             --output-format ply
```

#### `downsample`

downsamples a point cloud.

```shell
Usage: downsample --points-per-voxel <POINTS_PER_VOXEL>

Options:
  -p, --points-per-voxel <POINTS_PER_VOXEL>  
  -h, --help 
```

**Downsampling a file**

Downsamples pcd files and write as ply binary

```shell
vv read ./pcd +output=pcdb \
       downsample -p 2 +input=pcdb +output=pcdb_down \
       write ./pcdb_down \
             +input=pcdb_up \
             --storage-type binary \
             --output-format ply
```

**Complex Example**

```shell
vv read ./pcd                       +output=pcdb \
       read ./pcd_compressed            +output=pcd_comp \
       downsample -p 5 +input=pcdb      +output=pcdb_down \
       upsample   -f 2 +input=pcdb_down +output=pcdb_down_up \
       metrics +input=pcd_comp,pcdb_down_up +output=metric \
       write  ./metrics     +input=metric \
       write  ./down_up     +input=pcdb_down_up \
       render ./tmp/down_up +input=pcdb_down_up 
```

#### `convert`

We recognize that some users may just want to convert a file from one format to another. So `convert` is provided as a shortcut for `read` and `write`. Currently we support any conversion between ply and pcd. For `convert`, named input-ouput is not needed.

```shell
Usage: convert [OPTIONS] --output <OUTPUT>

Options:
  -o, --output <OUTPUT>                
      --output-format <OUTPUT_FORMAT>  [default: pcd]
  -s, --storage-type <STORAGE_TYPE>    [default: binary]
  -i, --input <INPUT>                  
  -h, --help                           Print help
```

**convert** from ply to pcd(binary)

```shell
vv convert --input ./ply_a --output ./pcd_b
```

**convert** from pcd to ply(ascii)

```shell
vv convert --input ./pcd_b --output ./ply_a --storage-type ascii --output-format ply
```

**convert** from pcd(binary) to pcd(ascii)

```shell
vv convert --input ./pcd_b --output ./pcd_a --storage-type ascii --output-format pcd
```

### `vvplay`

Plays a folder of pcd/ply files in lexicographical order. A window will appear upon running the binary from which you can navigate using your mouse and keyboard. Controls are described further below.

```shell
Plays a folder of pcd files in lexicographical order

USAGE:
    vvplay.exe [OPTIONS] <DIRECTORY>

ARGS:
    <DIRECTORY>    Directory with all the pcd files in lexicographical order

OPTIONS:
    -b, --buffer-size <BUFFER_SIZE>    [default: 1]
        --controls
    -f, --fps <FPS>                    [default: 30]
    -h, --height <HEIGHT>              [default: 900]
        --help                         Print help information
        --pitch <CAMERA_PITCH>         [default: 0]
    -w, --width <WIDTH>                [default: 1600]
    -x, --camera-x <CAMERA_X>          [default: 0]
    -y, --camera-y <CAMERA_Y>          [default: 0]
        --yaw <CAMERA_YAW>             [default: -90]
    -z, --camera-z <CAMERA_Z>          [default: 1.3]
```

### Controls

With the main screen focused,

1. `W` Key - Moves your position to the front
2. `A` Key - Moves your position to the left
3. `S` Key - Moves your position to the back
4. `D` Key - Moves your position to the right
5. 'Q' Key - Moves your position up
6. `E` Key - Moves your position down
7. `Space` Key - Toggles Play/Pause
8. `LeftArrow` Key - Rewinds by 1 frame
9. `RightArrow` Key - Advances by 1 frame
10. `Mouse` Drag - Adjusts camera yaw / pitch (Hold right click on Mac, left click on Windows)

With the secondary window focused,

![Playback Controls Secondary Window](docs/images/playback_controls.png)

The Play/Pause button toggles between play and pause. The slider allows you to navigate to any frame you wish.

The information displayed in the window are:

1. Current Frame / Total Frames
2. Camera Information - Useful to recreate a certain view through command line arguments

### Example

The following command will play all `.pcd` files in the `./pcds/` directory.

```shell
vvplay ./pcds
```

You can buffer the render with a set number of frames using `-b`

```shell
vvplay ./pcds -b 100
```

### `vvdash`

Simulates DASH streaming by sending input files to an output directory over simulated network conditions

### Example

The following command will send 300 frames of varying `"hi"` or `"lo"` qualities from `./input` to `./output`, depending on the simulated network conditions specified in `./simulated_network.txt`, which is a `.txt` file containing bandwidth conditions specified in KB/s, separated by newline characters `(\n)`.

```shell
vvdash ./input ./output ./simulated_network.txt 300
```

## For Developers

### Rust version

Use Rust 1.69

### Coding Style

We follow the [official Rust coding style](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md).  You can use `rustfmt` (or run `cargo fmt`) to automatically format your code.

