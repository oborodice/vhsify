# vhsify

- 画像・映像をVHS風に変換するCLIツール

## サポート形式

- 画像: jpg, jpeg, png, webp, avif
- 動画: mp4, mov, avi, mkv

## インストール

[GitHub Releases](https://github.com/oborodice/vhsify/releases) からバイナリをダウンロードして PATH に追加してください。

## 依存ツール

- ffmpeg

## 使い方

```bash
# <input>_vhs.<ext> としてカレントディレクトリに出力
$ vhsify <input>.<ext>

# --mode でワイドコンテンツの処理方法を指定（デフォルト: bars）
$ vhsify <input>.<ext> --mode bars  # 左右を黒帯で埋めて4:3の見える範囲にする
$ vhsify <input>.<ext> --mode crop  # 左右をクロップして4:3にする

# --output-dir で出力先ディレクトリを指定
$ vhsify <input>.<ext> --output-dir <dir>

# --output-name で出力ファイル名（拡張子なし）を指定
$ vhsify <input>.<ext> --output-name <name>
```

開発者向けの情報は [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。
