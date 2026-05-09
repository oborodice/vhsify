# vhsify

- 画像・映像をVHS風に変換するCLIツール

## 依存ツール

- Rust
- ffmpeg

## 使い方

```bash
# <input>_vhs.<ext> としてカレントディレクトリに出力
$ cargo run --release -- <input>.<ext>

# --mode でワイドコンテンツの処理方法を指定（デフォルト: bars）
$ cargo run --release -- <input>.<ext> --mode bars  # 左右を黒帯で埋めて4:3の見える範囲にする
$ cargo run --release -- <input>.<ext> --mode crop  # 左右をクロップして4:3にする

# --output で出力先ディレクトリを指定
$ cargo run --release -- <input>.<ext> --output <dir>

# [テスト用] samples/images/ 以下を一括処理して output/images/ に出力
$ bash scripts/generate_samples.sh

# [テスト用] output/images/ の生成物を削除
$ bash scripts/clean_samples.sh
```
