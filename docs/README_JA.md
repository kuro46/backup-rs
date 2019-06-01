\[[English](../README.md)|日本語\]

Rustで書かれたシンプルなバックアップソフト

# 機能

- 複数パスのバックアップ
- アーカイブの日付フォーマット
- パスのフィルタリング (例: Cargoのtargetディレクトリの除外)

# ダウンロード

Windowsの用のビルドは[ここ](https://github.com/kuro46/backup-rs/releases/downloads/latest/backup-windows-x86_64.zip)からダウンロードできます。  
また、[releases](https://github.com/kuro46/backup-rs/releases)から任意のバージョンをダウンロードすることもできます。

# ビルド

このリポジトリをcloneし、backup-rsディレクトリで`cargo build --release`を実行してください。

# 使い方

## Windows

設定後、backup.exeを実行することでバックアップを開始します。

# 設定

このソフトは、`./settings.toml`を設定ファイルとして扱います。  
書き方は下を参考にしてください。

```toml
# settings.toml

# https://docs.rs/chrono/0.4/chrono/format/strftime/index.html
# バックアップしたアーカイブの保存先(tar形式です)
archive-file-path = "./%Y-%m-%d.tar"

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
