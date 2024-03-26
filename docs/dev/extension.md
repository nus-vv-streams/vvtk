# Extending vvtk with Custom Commands 
This document describes how to extend vvtk with new custom commands using ``vv extend``. 

## Assumptions for custom commands 
- The additional command should take in a ``PointCloud<PointXyzRgba>`` and output a ``PointCloud<PointXyzRgba>``. 
- The additional command is written in Rust, and the executable is stored in ``PATH`` or Cargo Home, which is ``$HOME/.cargo/bin/`` by default.
- The name of the executable should be in the format of ``vv-name``, for example ``vv-test-command``. 

## Supported usage of vv extend
- vv commands that can be placed after ``extend``
    - downsample 
    - extend 
    - metric 
    - normal_estimation 
    - render 
    - upsample 
    - write 
- vv commands that can be placed after ``extend``
    - extend 
    - downsample 
    - read 
- Example command format:
    - read + extend + downsample: 
    `` vv read ./test_files/ply_ascii/  +output=plya \extend test-command +input=plya +output=plyb \write ./test_write --output-format=ply +input=plyb``



## Example for creating a subcommand executables
1. Create a new Rust package with Cargo that has ``vv-`` as prefix in the name. 
```bash
cargo new vv-test-command 
cd vv-test-command
```
2. Add ``serde`` and ``serde_json`` as dependency in ``Cargo.toml``
```
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0.113"
```
3. Copy the code in [docs/dev/vv-extend-template.rs](./vv-extend-template.rs) to ``src/main.rs``
4. Implement additional point cloud transformation feature in ``new_pc_transform_function``.
```rust
fn main() {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line.expect("Could not read the piped stdin");
        let deserialised_input = line.clone();
        // Regenerate the SubcommandObject from input 
        let deserialized: SubcommandObject<PointCloud<PointXyzRgba>> = serde_json::from_str(&deserialised_input).unwrap();
        let mut deserialized_pc = *deserialized.content;
        // Do something here to transform the deserialized_pc
        deserialized_pc = new_pc_transform_function(deserialized_pc);
        // Serialize the point cloud
        let new_subcommand_object = SubcommandObject::new(deserialized_pc);
        // Pass serialized SubcommandObject to the parent process
        let serialized: String = serde_json::to_string(&new_subcommand_object).unwrap();
        print!("{}", serialized);
    }
}

// Custom function to transform the point cloud input
fn new_pc_transform_function(pc:PointCloud<PointXyzRgba>) -> PointCloud<PointXyzRgba> {
    pc
}
```
4. Build the project.
```bash
cargo build --release --bins
```
5. Add the binaries to ``$HOME/.cargo``
```bash
cargo install --path .
```

## Test the subcommand added
1. Go to vvtk root directory
2. Copy a ply-ascii file that has the file type ``.ply`` to ``./test_pc``
2. Run ``cargo run --bin vv read ./test_pc  +output=plya \extend test-command +input=plya``
4. If the subcommand ran sucessfully, output below will be shown:
```
Subprocess exited with status code: 0
```

## Additional information
- ``vv extend`` will find the executable from 
``PATH`` or ``$HOME/.cargo/bin/``, so if the executable is placed in other path, alter the location of Cargo Home by using ``export CARGO_HOME=/path/to/executable`` or add the path to ``PATH`` variable by using ``export PATH=$PATH:path/to/executable
`` in Linux. 
   - Refer to Rust official [Cargo Home documentation](https://doc.rust-lang.org/cargo/guide/cargo-home.html) for more information.  
