# Contributing

## Requirements

- Rust 1.85+
- FFmpeg

## Run

```bash
$ cargo run --release -- <input>.<ext>
```

## Test

```bash
# Process all files under samples/images/ and output to output/images/
$ bash scripts/generate_samples.sh

# Process samples/sample.mp4 in various formats, aspect ratios, and modes, output to output/videos/
$ bash scripts/generate_videos.sh

# Remove generated files under output/
$ bash scripts/clean_samples.sh
```

## Release

```bash
$ git tag vX.X.X && git push origin vX.X.X
```

Pushing a tag triggers GitHub Actions to build binaries and attach them to a GitHub Release.
