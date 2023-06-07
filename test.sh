# downsample and then upsample, then compare metrics
# cargo run --bin vivotk -- read ./my_test/pcd_a/longdress_vox10_1213.pcd +output=pcda \
#                     downsample -p 2 +input=pcda +output=pcda_ds \
#                     metrics +input=pcda,pcda_ds +output=metrics\
#                     write --output-dir ./test_files/metrics +input=metrics


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

