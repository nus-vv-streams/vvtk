#!/bin/bash

# 统计总代码行数

# get command args
src_path=$1
file_ext=$2
echo "src_path: $src_path"

total_lines=0

# 查找src目录下的所有Rust文件
while IFS= read -r file; do
    # Stat the current file's code lines
    lines=$(cat "$file" | 
    # grep -vE '^\s*(//|/\*)' |
    # grep -vE '^\s*$' | 
    wc -l)
    echo "Lines: $lines, File: $file"

    # Accumulate the code lines to the total count
    total_lines=$((total_lines + lines))
done < <(find "$src_path" -type f -name "*.${file_ext}")

echo "Total Lines: $total_lines"
