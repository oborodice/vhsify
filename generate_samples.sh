#!/bin/bash
set -e

BINARY="./target/release/vhsify"
INPUT_DIR="samples/images"
OUTPUT_DIR="output/images"

cargo build --release --quiet

find "$INPUT_DIR" -type f | grep -iE '\.(jpg|jpeg|png|webp|avif)$' | while read -r file; do
    rel="${file#$INPUT_DIR/}"
    rel_dir=$(dirname "$rel")
    [ "$rel_dir" = "." ] && out_dir="$OUTPUT_DIR" || out_dir="$OUTPUT_DIR/$rel_dir"

    echo "Processing: $file"
    "$BINARY" "$file" --output "$out_dir"
done

echo "Done. Outputs saved to $OUTPUT_DIR/"
