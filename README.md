# vhsify

- 画像・映像をVHS風に変換するCLIツール

## 依存ツール

- Rust
- ffmpeg

## 使い方

```bash
# <input>_vhs.<ext> として出力
$ cargo run --release -- <input>.<ext>

# --mode でワイドコンテンツの処理方法を指定（デフォルト: bars）
$ cargo run --release -- <input>.<ext> --mode bars  # 左右を黒帯で埋めて4:3の見える範囲にする
$ cargo run --release -- <input>.<ext> --mode crop  # 左右をクロップして4:3にする
```
