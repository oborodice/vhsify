# Contributing

## Requirements

- Rust 1.85+
- [FFmpeg](https://ffmpeg.org/)

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
$ bash scripts/release.sh X.X.X
```

1. Bumps the version in `Cargo.toml` and commits
1. Pushes the tag
1. GitHub Actions builds binaries and attaches them to a GitHub Release
