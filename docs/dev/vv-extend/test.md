# Test vv-extend 
This document describes how to test vv-extend. 

1. ``cd`` to ``vvtk`` project root directory.   
2. Move the executable in ``vvtk/test_files/extend`` to ``~/.cargo/bin`` using
```sh
mv ./test_files/extend/* ~/.cargo/bin
```
3. These are commands that should be sucessfully executed:
    - read + extend + extend:
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plyb \extend downsampletest +input=plyb +output=plyc \extend downsampletest +input=plyc``
        - double invocation is a bit weird as an extra invocation is created, but it doesn't affect the result, something start from the main branch, record in issue
    - read + downsample + extend:
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plyb \downsample -p 2 +input=plyb +output=plyc \extend nochange +input=plyc``
    - read + extend: 
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plyc \extend nochange +input=plyc``
    - read + extend + downsample:
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plyc \extend nochange +input=plyc +output=plyd \downsample -p 2 +input=plyd``
    - read + read + metric
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plya \read ./test_files/ply_ascii/  +output=plyb \metrics +input=plya,plyb +output=metrics``
    - read + read + extend + metric
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plya \read ./test_files/ply_ascii/  +output=plyb \extend nochange +input=plyb +output=plyc \metrics +input=plya,plyc +output=metrics``
    - read + normal_estimation
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plya \normal --k 30 +input=plya``
    - read + extend + normal_estimation
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plya \extend nochange +input=plya +output=plyb \normal --k 30 +input=plyb``
    - read + extend + render
        - ./test_files/longdress/longdress/test_single_ply
        - ``cargo run --bin vv read ./test_files/longdress/longdress/test_single_ply +output=plya \extend nochange +input=plya +output=plyb \render ./test_render +input=plyb``I
    - read + extend + upsample
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plya \extend nochange +input=plya +output=plyb \upsample --factor 2``
    - read + extend + write
        - ``cargo run --bin vv read ./test_files/ply_ascii/  +output=plya \extend nochange +input=plya +output=plyb \write ./test_write --output-format=ply +input=plyb``
