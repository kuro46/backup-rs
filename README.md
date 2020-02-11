\[English|[日本語](docs/README_JA.md)\]

**Deprecated! Please check [kuro46/simple-backup](https://github.com/kuro46/simple-backup)!**

A simple backup software written in Rust.

# Features

- Backup multiple paths
- Format archive file name by date
- Filter specified paths (e.g. exclude Cargo's "target" directory)

# Downloads

Click [here](https://github.com/kuro46/backup-rs/releases/downloads/latest/backup-windows-x86_64.zip) to download windows build.  
Or click [here](https://github.com/kuro46/backup-rs/releases) to download any versions.

# Build

Clone this repository and execute `cargo build --release` at "backup-rs" directory.

# Usage

## Windows

After setting, execute backup.exe to start backup.

# Setting

This software uses `./settings.toml` to settings file.  
Formats are shown below.

```toml
# settings.toml

# https://docs.rs/chrono/0.4/chrono/format/strftime/index.html
# Path of archive file
archive-file-path = "./%Y-%m-%d.tar"

[[targets]]
name = "target-name"
path = "target-path"

[[filters]]
# Filter that filters "target" directory if "Cargo.toml" found while backing up "target-name".
name = "rust-project"
applied-to = "target-name"
exclude = ["./target"]
if-exists = "./Cargo.toml"
[[filters]]
# Filter that filters "target" directory if "pom.xml" found while backing up "target-name".
name = "maven-project"
applied-to = "target-name"
exclude = ["./target"]
if-exists = "./pom.xml"
[[filters]]
# Filter that filters "build" directory and ".gradle" directory if "build.gradle.kts" found.
name = "gradle-project-with-kotlin-DSL"
applied-to = "target-name"
exclude = ["./build", "./.gradle"]
if-exists = "./build.gradle.kts"
[[filters]]
# Filter that filters "build" directory and ".gradle" directory if "build.gradle" found.
name = "gradle-project"
applied-to = "target-name"
exclude = ["./build", "./.gradle"]
if-exists = "./build.gradle"

```
