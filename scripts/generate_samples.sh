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

SAMPLE="$INPUT_DIR/sample.jpg"
echo "Processing mode variants: $SAMPLE"
"$BINARY" "$SAMPLE" --output "$OUTPUT_DIR/mode" --output-name sample_default_vhs
"$BINARY" "$SAMPLE" --mode bars --output "$OUTPUT_DIR/mode" --output-name sample_bars_vhs
"$BINARY" "$SAMPLE" --mode crop --output "$OUTPUT_DIR/mode" --output-name sample_crop_vhs

echo "Done. Outputs saved to $OUTPUT_DIR/"
