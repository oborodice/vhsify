# Contributing

## 開発環境

- Rust
- ffmpeg

## 実行

```bash
$ cargo run --release -- <input>.<ext>
```

## テスト

```bash
# samples/images/ 以下を一括処理して output/images/ に出力
$ bash scripts/generate_samples.sh

# samples/sample.mp4 を各フォーマット・アスペクト比・モードで処理して output/videos/ に出力
$ bash scripts/generate_videos.sh

# output/ の生成物を削除
$ bash scripts/clean_samples.sh
```

## リリース

```bash
$ git tag vX.X.X && git push origin vX.X.X
```

タグを push すると GitHub Actions が自動ビルドし、GitHub Releases にバイナリを添付します。
