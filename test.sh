# downsample and then upsample, then compare metrics
cargo run --bin vivotk -- read ./my_test/pcd_a/longdress_vox10_1213.pcd +output=pcda \
                    downsample -p 2 +input=pcda +output=pcda_ds \
                    metrics +input=pcda,pcda_ds +output=metrics\
                    write --output-dir ./test_files/metrics +input=metrics
