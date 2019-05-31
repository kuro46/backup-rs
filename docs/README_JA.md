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

Windowsの場合は[ここ](https://github.com/kuro46/backup-rs/releases)からバイナリをダウンロードできます。  
Windows10Home(64bit)で動作確認済

# 設定

このソフトは設定にtomlフォーマットを使用します。
また、ファイル名は`settings.toml`、
フォルダは実行可能ファイルの存在するフォルダとします。

```toml
# settings.toml

# https://docs.rs/chrono/0.4/chrono/format/strftime/index.html
archive-file-path = "./%Y-%m-%d.tar.tar"

[[targets]]
name = "target-name"
path = "target-path"

[[filters]]
# target-nameをバックアップ中にCargo.tomlがあれば、
# そのファイルのディレクトリ内のtargetディレクトリを除外するフィルタ
name = "rust-project"
applied-to = "target-name"
exclude = ["./target"]
if-exists = "./Cargo.toml"
[[filters]]
# target-nameをバックアップ中にpom.xmlがあれば、
# そのファイルのディレクトリ内のtargetディレクトリを除外するフィルタ
name = "maven-project"
applied-to = "target-name"
exclude = ["./target"]
if-exists = "./pom.xml"
[[filters]]
# target-nameをバックアップ中にbuild.gradle.ktsがあれば、
# そのファイルのディレクトリ内のbuildディレクトリと.gradleディレクトリを除外する
name = "gradle-project-with-kotlin-DSL"
applied-to = "target-name"
exclude = ["./build", "./.gradle"]
if-exists = "./build.gradle.kts"
[[filters]]
# target-nameをバックアップ中にbuild.gradleがあれば、
# そのファイルのディレクトリ内のbuildディレクトリと.gradleディレクトリを除外する
name = "gradle-project"
applied-to = "target-name"
exclude = ["./build", "./.gradle"]
if-exists = "./build.gradle"

```

# 実行

`cargo run --release`  
または  
`backup-rs/target/release/backup.exe`  
を実行
