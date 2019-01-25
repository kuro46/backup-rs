#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use chrono::Local;
use env_logger;
use tar::Builder;

use backup::{Condition, Filter, FilterType, Target};

mod backup;

fn main() {
    for arg in std::env::args() {
        if let "--version" = arg.as_str() {
            println!(env!("CARGO_PKG_VERSION"));
            return;
        }
    }

    initialize_logger();

    let mut archiver;
    let targets: Vec<Target>;
    let filters: Vec<Filter>;
    //initialization
    {
        let settings = Settings::load();
        archiver = prepare_start(settings.archive_path.as_str());

        targets = settings.targets.into_iter()
            .map(|setting| setting.into_target())
            .collect();
        filters = settings.filters.unwrap_or_default()
            .into_iter()
            .map(|setting| setting.into_filter())
            .collect();
    }

    backup::start(targets.as_slice(),
                  filters.as_slice(),
                  &mut archiver);
}

fn initialize_logger() {
    let log_level_env_key = "RUST_LOG";
    if env::var(log_level_env_key).is_err() {
        env::set_var(log_level_env_key, "INFO");
    }

    env_logger::Builder::from_default_env().format(|formatter, record| {
        use log::Level;
        use colored::Colorize;

        let level = record.level();
        let level_string = format!("{}", level);
        let level_str = level_string.as_str();
        let level_str = match level {
            Level::Debug | Level::Trace => level_str.bright_black(),
            Level::Warn => level_str.yellow(),
            Level::Error => level_str.bright_red(),
            _ => colored::ColoredString::from(level_str)
        };
        //1: timestamp 2: level 3: body
        writeln!(formatter, "[{} {}] {}",
                 Local::now().format("%Y-%m-%d %H:%M:%S"),
                 level_str,
                 record.args())
    }).init();

    debug!("Logger initialized.");
}

fn prepare_start(archive_path: &str) -> Builder<File> {
    info!("Preparing to start...");

    let file_path_str = Local::now().format(archive_path).to_string();
    let file_path = Path::new(&file_path_str);
    if file_path.exists() {
        warn!("File: \"{}\" already exists!", file_path_str);
        warn!("overwrite it? (Y/N)");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line!");
        if input.trim() != "y" {
            info!("Exiting...");
            std::process::exit(0);
        }
    }
    let file = File::create(file_path_str).expect("Failed to create archive file.");

    Builder::new(file)
}

#[derive(Deserialize, Debug)]
struct FilterSetting {
    name: String,
    execute: String,
    scopes: Vec<String>,
    targets: Vec<String>,
    conditions: Vec<String>,
}

impl FilterSetting {
    fn into_filter(self) -> Filter {
        let targets: Vec<PathBuf> = self.targets.iter()
            .map(|path| Path::new(path).to_path_buf())
            .collect();
        let conditions: Vec<Condition> = self.conditions.iter().map(|condition_str| {
            let not = condition_str.starts_with('!');
            let condition_str = if not {
                condition_str.replacen("!", "", 1)
            } else {
                condition_str.to_string()
            };
            let path = Path::new(&condition_str).to_path_buf();

            Condition {
                not,
                path,
            }
        }).collect();

        let execute = self.execute.as_str();
        Filter {
            name: self.name,
            filter_type: FilterType::from_str(execute)
                .unwrap_or_else(|| panic!("Filter type: {} not exists!", execute)),
            scopes: self.scopes,
            targets,
            conditions,
        }
    }
}

#[derive(Deserialize, Debug)]
struct TargetSetting {
    name: String,
    paths: Vec<String>,
}

impl TargetSetting {
    fn into_target(self) -> Target {
        let paths: Vec<PathBuf> = self.paths.iter()
            .map(|path| dunce::canonicalize(Path::new(path)).unwrap())
            .collect();

        Target {
            name: self.name,
            paths,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Settings {
    archive_path: String,
    targets: Vec<TargetSetting>,
    filters: Option<Vec<FilterSetting>>,
}

impl Settings {
    pub fn load() -> Settings {
        info!("Loading settings...");

        let settings_path = Path::new("./settings.toml");
        if !settings_path.exists() {
            warn!("Settings file not exists! creating it and exiting...");
            File::create(settings_path).expect("Failed to create settings.toml.");
            std::process::exit(1);
        }
        let settings_file = File::open("settings.toml").expect("Failed to open settings.toml");
        let mut reader = BufReader::new(settings_file);

        let mut settings_buffer = String::new();
        reader.read_to_string(&mut settings_buffer).expect("Failed to read lines from settings.toml");
        let settings: Settings = toml::from_str(&settings_buffer.as_str()).expect("Failed to deserialize settings.");

        debug!("Settings: {:?}", settings);
        info!("Settings loaded.");

        settings
    }
}

