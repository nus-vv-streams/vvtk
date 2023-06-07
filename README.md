# The Vivo Toolkit (VivoTk)

![format badge](https://github.com/nus-vv-streams/vivotk/actions/workflows/format.yml/badge.svg)
![build badge](https://github.com/nus-vv-streams/vivotk/actions/workflows/build.yml/badge.svg)

### Rust version

Use Rust 1.69

### Coding Style

We follow the [official Rust coding style](https://github.com/rust-dev-tools/fmt-rfcs/blob/master/guide/guide.md).  You can use `rustfmt` (or run `cargo fmt`) to automatically format your code.

## Commands

### `vivotk`

Provides subcommands that can be chained together. The inputs and outputs of a subcommand must be specified with the `+input=` followed by a comma separated list of inputs or `+output=` to denote the name of its output stream.

**Example**
```shell
vivotk read ./Plys +output=plys \
        write --pcd ascii -o ./Pcds +input=plys
```

#### `read`

Reads in one of our supported file formats. Files can be of the type `.pcd` `.ply` `.metrics`

```shell
vivotk read ./Ply +output=plys
```

#### `to_png`

Writes point clouds from the input stream into images

```shell
vivotk read ./Ply +output=plys \
        to_png -o ./Pngs +input=plys
```

#### `metrics`

Calculates the metrics given two input streams where the first input stream is the original and the second is the reconstructed one.

```shell
vivotk read ./original +output=original \
        read ./reconstructed +output=reconstructed \
        metrics +input=original,reconstructed
```

#### `write`

Writes from input stream into a file

**Writing metrics**
```shell
vivotk read ./original +output=original \
        read ./reconstructed +output=reconstructed \
        metrics +input=original,reconstructed +output=metrics \
        write +input=metrics 
```

**Writing pcds**
```shell
vivotk read ./Plys +output=plys \
        write -o ./Pcds +input=plys
```

### `ply_play`

Plays a folder of pcd files in lexicographical order. A window will appear upon running the binary from which you can navigate using your mouse and keyboard. Controls are described further below.

```shell
Plays a folder of pcd files in lexicographical order

USAGE:
    ply_play.exe [OPTIONS] <DIRECTORY>

ARGS:
    <DIRECTORY>    Directory with all the pcd files in lexicographical order

OPTIONS:
    -b, --buffer-size <BUFFER_SIZE>    [default: 1]
        --controls
    -f, --fps <FPS>                    [default: 30]
    -h, --height <HEIGHT>              [default: 900]
        --help                         Print help information
        --pitch <CAMERA_PITCH>         [default: -20]
    -w, --width <WIDTH>                [default: 1600]
    -x, --camera-x <CAMERA_X>          [default: 0]
    -y, --camera-y <CAMERA_Y>          [default: 0]
        --yaw <CAMERA_YAW>             [default: -90]
    -z, --camera-z <CAMERA_Z>          [default: 0]
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
ply_play ./pcds
```

You can buffer the render with a set number of frames using `-b`

```shell
ply_play ./pcds -b 100
```

### `vvdash`

Simulates DASH streaming by sending input files to an output directory over simulated network conditions

### Example

The following command will send 300 frames of varying `"hi"` or `"lo"` qualities from `./input` to `./output`, depending on the simulated network conditions specified in `./simulated_network.txt`, which is a `.txt` file containing bandwidth conditions specified in KB/s, separated by newline characters `(\n)`.

```shell
vvdash ./input ./output ./simulated_network.txt 300
```
