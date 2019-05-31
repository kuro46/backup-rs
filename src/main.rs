#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use tar::Builder;
use walkdir::WalkDir;
use chrono::Local;

fn main() {
    println!("Loading settings...");
    let settings = load_settings();
    println!("Successfully loaded!\nBacking up...");
    execute(&settings);
    println!("Successfully Backed up!");
}

fn execute(settings: &Settings) {
    let archive_file_name = Local::now().format(&settings.archive_file_path).to_string();
    let mut archive_file = File::create(archive_file_name)
        .expect("Failed to create archive file");
    let mut archiver = Builder::new(&mut archive_file);

    for target in &settings.targets {
        println!("Current target: {}({})", &target.name, &target.path);
        let mut filterable_dir: Option<PathBuf> = Option::None;
        let filters: Vec<&Filter> = settings.filters.iter()
            .filter(|filter| filter.applied_to.eq(&target.name))
            .collect();

        'outer: for entry in WalkDir::new(&target.path) {
            let entry = entry.expect("Failed to unwrap");
            let entry_path = entry.path();

            if entry_path.is_dir() && is_filterable_dir(&filters, entry_path) {
                filterable_dir = Option::Some(entry_path.to_path_buf());
            }

            if filterable_dir.is_some() &&
                entry_path.starts_with(&filterable_dir.as_ref().unwrap()) {
                continue 'outer
            }

            execute_dir_entry(entry_path, target, &mut archiver);
        }
    }

    let need_flush = archiver.into_inner().expect("Failed to finish the archiver");
    need_flush.flush().expect("Failed to flush the archive file");
}

fn is_filterable_dir(filters: &[&Filter], path: &Path) -> bool {
    for filter in filters {
        let parent = path.parent()
            .expect("Cannot get parent path");

        if append_paths(parent, &filter.if_exists).exists() {
            let path_buf = &path.to_path_buf();
            for exclude_relative_path in &filter.exclude {
                if append_paths(parent, exclude_relative_path).eq(path_buf) {
                    println!("  Filtered({}): {}", &filter.name, path.display());
                    return true
                }
            }
        }
    }

    false
}

fn append_paths(parent: &Path, child: &str) -> PathBuf {
    let mut path_buf = parent.to_path_buf();
    path_buf.push(child);

    path_buf
}

fn execute_dir_entry(
    entry_path: &Path,
    target: &Target,
    archiver: &mut Builder<&mut File>
) {
    if !entry_path.is_file() {
        return
    }

    let archive_to = {
        let mut result = PathBuf::new();
        result.push(&target.name);
        result.push(trim_duplicated(entry_path, Path::new(&target.path)));

        result
    };

    let mut opened_file = File::open(entry_path)
        .expect("Failed to open file");
    archiver.append_file(archive_to, &mut opened_file)
        .expect("Failed to archive file");
}

fn trim_duplicated(target: &Path, filter: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    let mut target_iter = target.iter();
    let mut filter_iter = filter.iter();

    loop {
        let target_next = target_iter.next();
        if target_next.is_none() {
            break
        }
        let filter_next = filter_iter.next();
        let target_next = target_next.unwrap();
        if filter_next.is_some() && target_next.eq(filter_next.unwrap()) {
            continue
        }

        result.push(target_next);
    }

    result
}

fn load_settings() -> Settings {
    let settings_path = Path::new("./settings.toml");
    if !settings_path.exists() {
        eprintln!("./settings.toml does not exists!");
        File::create(settings_path).expect("Failed to create settings file");
        std::process::exit(0)
    }

    let opened_file = File::open(settings_path).expect("Failed to open ./settings.toml");

    let settings: Settings = {
        // Loads settings

        let mut reader = BufReader::new(opened_file);
        let mut buf = String::new();
        reader.read_to_string(&mut buf).expect("Failed to read lines from settings.toml");

        toml::from_str(&buf.as_str()).expect("Failed to deserialize settings")
    };

    settings
}

#[derive(Deserialize, Debug)]
struct Settings {
    #[serde(rename = "archive-file-path")]
    archive_file_path: String,
    targets: Vec<Target>,
    filters: Vec<Filter>,
}

#[derive(Deserialize, Debug)]
struct Target {
    name: String,
    path: String,
}

#[derive(Deserialize, Debug)]
struct Filter {
    name: String,
    #[serde(rename = "applied-to")]
    applied_to: String,
    exclude: Vec<String>,
    #[serde(rename = "if-exists")]
    if_exists: String,
}
