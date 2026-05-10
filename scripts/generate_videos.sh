#!/bin/bash
set -e

BINARY="./target/release/vhsify"
SAMPLE="samples/sample.mp4"
OUTPUT_DIR="output/videos"

cargo build --release --quiet

SRC_DIR="$OUTPUT_DIR/src"

echo "Converting to other formats..."
ffmpeg -i "$SAMPLE" -c copy "$SRC_DIR/sample.mov" -y 2>/dev/null
ffmpeg -i "$SAMPLE" -c copy "$SRC_DIR/sample.avi" -y 2>/dev/null
ffmpeg -i "$SAMPLE" -c copy "$SRC_DIR/sample.mkv" -y 2>/dev/null

echo "Processing: $SAMPLE"
"$BINARY" "$SAMPLE" --output-dir "$OUTPUT_DIR"

for file in "$SRC_DIR"/sample.{mov,avi,mkv}; do
    echo "Processing: $file"
    "$BINARY" "$file" --output-dir "$OUTPUT_DIR/formats"
done

echo "Converting aspect ratio variants..."
ffmpeg -i "$SAMPLE" -vf "crop=1440:1080:240:0" "$SRC_DIR/sample_4_3.mp4" -y 2>/dev/null
ffmpeg -i "$SAMPLE" -vf "crop=608:1080:656:0" "$SRC_DIR/sample_9_16.mp4" -y 2>/dev/null

for file in "$SRC_DIR"/sample_4_3.mp4 "$SRC_DIR"/sample_9_16.mp4; do
    echo "Processing: $file"
    "$BINARY" "$file" --output-dir "$OUTPUT_DIR/aspect"
done

echo "Processing mode variants: $SAMPLE"
"$BINARY" "$SAMPLE" --output-dir "$OUTPUT_DIR/mode" --output-name sample_default_vhs
"$BINARY" "$SAMPLE" --mode bars --output-dir "$OUTPUT_DIR/mode" --output-name sample_bars_vhs
"$BINARY" "$SAMPLE" --mode crop --output-dir "$OUTPUT_DIR/mode" --output-name sample_crop_vhs

echo "Done. Outputs saved to $OUTPUT_DIR/"
