# downsample and then upsample, then compare metrics
cargo run --bin vivotk -- read ./my_test/pcd_a/longdress_vox10_1213.pcd +output=pcda \
                    downsample -p 2 +input=pcda +output=pcda_ds \
                    metrics +input=pcda,pcda_ds +output=metrics\
                    write --output-dir ./test_files/metrics +input=metrics


# ply ascii to pcd binary
echo "ply ascii to pcd binary"
cargo run --bin vivotk -- convert --input  ./8i_dataset/red_black/ply_a \
                                  --output ./8i_dataset/red_black/pcd_b \
                                  --storage-type binary \
                                  --output-format pcd

# pcd binary to pcd ascii
echo "pcd binary to pcd ascii"
cargo run --bin vivotk -- convert --input  ./8i_dataset/red_black/pcd_b \
                                  --output ./8i_dataset/red_black/pcd_a \
                                  --storage-type ascii \
                                  --output-format pcd

# pcd binary to ply binary
echo "pcd binary to ply binary" 
cargo run --bin vivotk -- convert --input  ./8i_dataset/red_black/pcd_b \
                                  --output ./8i_dataset/red_black/ply_b \
                                  --storage-type binary \
                                  --output-format ply

# play pcd
cargo run --release --bin ply_play --   ./8i_dataset/red_black/pcd_b

# play ply binary
cargo run --release --bin ply_play --   ./8i_dataset/red_black/ply_b


# try read write in release mode
mkdir tmp

# read pcd binary and downsampled pcd binary, calculate metrics, write to tmp
cargo run --release --bin vivotk -- read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    read ./8i_dataset/red_black/pcd_b_down +output=pcdb_ds \
                                    metrics +input=pcdb,pcdb_ds +output=metrics            \
                                    write --output-dir ./tmp/metrics +input=metrics


cargo run --release --bin vivotk -- read ./8i_dataset/red_black/pcd_b_down/00000.pcd +output=pcdb_ds \
                                    read ./8i_dataset/red_black/pcd_b/redandblack_vox10_1510.pcd      +output=pcdb    \
                                    upsample -f 1 +input=pcdb_ds         +output=pcdb_us \
                                    metrics +input=pcdb,pcdb_us            +output=metrics \
                                    write --output-dir ./tmp/metrics_down_up +input=metrics 


# a lot of reading
cargo run --release --bin vivotk -- read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    read ./8i_dataset/red_black/pcd_b      +output=pcdb    \
                                    write --output-dir ./tmp/ +input=pcdb

cargo run --release --bin vivotk -- read ./8i_dataset/red_black/ply_a/redandblack_vox10_1510.ply        +output=plya \
                                    write --output-dir ./tmp/ --output-format pcd --storage-type binary +input=plya


cargo run --release --bin vivotk -- read ./8i_dataset/red_black/ply_b/redandblack_vox10_1510.ply +output=plyb \
                                    to_png +input=plyb --output-dir ./tmp/

cargo run --release --bin vivotk -- read ./8i_dataset/red_black/ply_b/redandblack_vox10_1510.ply +output=pcdb \
                                    read ./8i_dataset/red_black/pcd_b_down/00000.pcd +output=pcd_comp \
                                    downsample -p 5 +input=pcdb      +output=pcdb_down \
                                    upsample   -f 1 +input=pcdb_down +output=pcdb_down_up \
                                    metrics +input=pcd_comp,pcdb_down_up +output=metric \
                                    write --output-dir ./tmp/metrics +input=metric \
                                    write --output-dir ./tmp/down_up +input=pcdb_down_up \
                                    to_png +input=pcdb_down_up  --output-dir ./tmp/down_up




rm -rf tmp
