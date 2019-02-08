\[[English](../README.md)|日本語\]

Rustで書かれたシンプルなバックアップソフト

# 機能

- 複数パスのバックアップ
- アーカイブの日付フォーマット
- バックアップ後に実行する任意のコマンドの設定
- パスのフィルタリング (例: Cargoのtargetディレクトリの除外)

# ビルド

1. Rustをインストール

2. backup-rsディレクトリで`cargo build --release`を実行

# 設定

このソフトは設定にtomlフォーマットを使用します。
また、ファイル名は`settings.toml`、
フォルダは実行可能ファイルの存在するフォルダとします。

```toml
# settings.toml

# この設定の場合、以下のような流れになる
# 1. "/foo/bar/"ディレクトリと"/buzz"ディレクトリを"C:/%Y-%m-%d.tar"に追加
# 2. "foo.exe arg1 arg2"と"bar.exe arg1 arg2 arg3"を実行
# 3. 終わり

# アーカイブファイルのパス
# 日付フォーマットが使用可能
# 詳細: https://docs.rs/chrono/0.4.6/chrono/format/strftime/index.html
# 必須
archive_path = "C:/%Y-%m-%d.tar"

# バックアップ終了後に実行するコマンドリスト
# "%archive_path%"とすることでアーカイブの絶対パスが代入される
# 任意: デフォルト値は空の配列
# 注意: コマンドには絶対パスを使用すること
commands_after_backup = [
    ["foo.exe","arg1","arg2"],
    ["bar.exe","arg1","arg2","arg3"]
]

# ターゲットリスト
# 必須
[[targets]]
# /foo/barと/buzzをバックアップする

# このターゲットの名前
# 必須
name = "target_name"
# パスのリスト
# 必須
paths = [
    "/foo/bar/",
    "/buzz/"
]

# フィルターリスト
# 任意: デフォルト値は空の配列
[[filters]]
# Cargoのtargetディレクトリを除外するフィルター

# フィルターの名前
# 必須
name = "exclude_cargo_target"
# このフィルターが有効なターゲットのリスト
# 必須
# この場合は"target_name"でのみ有効
# また、"global"と設定することですべてのターゲットに対して有効化できる
scopes = ["target_name"]
# このフィルターが適用される条件
# 必須
# この場合は"Cargo.toml"が存在した場合に"targets"に記述されているパスを除外する
# また、パスの左に!をつけると、存在しなかった場合に"targets"に記述されているパスを除外する
conditions = ["./Cargo.toml"]
# このフィルターのターゲットパス
# 必須
# この場合は"conditions"の条件が一致した場合に"targets"を除外する
targets = ["./targets/"]
```

# 実行

`cargo run --release`  
または  
`backup-rs/target/release/backup.exe`  
を実行
