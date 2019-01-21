#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;

use chrono::Utc;
use env_logger;
use tar::Builder;

use backup::{Filter, Target};

mod backup;

fn main() {
    for arg in std::env::args() {
        match arg.as_str() {
            "--version" => {
                println!(env!("CARGO_PKG_VERSION"));
                return;
            },
            _ => {},
        }
    }

    initialize_logger();
    let settings = Settings::load();
    let mut archiver = prepare_start(settings.archive_path.as_str());

    backup::start(settings.targets,
                  settings.filters.unwrap_or_else(|| { Vec::new() }),
                  &mut archiver);
}

fn initialize_logger() {
    let log_level_env_key = "RUST_LOG";
    if env::var(log_level_env_key).is_err() {
        env::set_var(log_level_env_key, "INFO");
    }
    env_logger::init();

    debug!("Logger initialized.");
}

fn prepare_start(archive_path: &str) -> Builder<File> {
    info!("Preparing to start...");

    let file_path_str = Utc::now().format(archive_path).to_string();
    let file_path = Path::new(&file_path_str);
    if file_path.exists() {
        warn!("File: \"{}\" already exists!", file_path_str);
        warn!("overwrite it? (Y/N)");
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim() != "y" {
            info!("Exiting...");
            std::process::exit(0);
        }
    }
    let file = File::create(file_path_str).unwrap();

    Builder::new(file)
}


#[derive(Deserialize, Debug)]
pub struct Settings {
    pub archive_path: String,
    pub targets: Vec<Target>,
    pub filters: Option<Vec<Filter>>,
}

impl Settings {
    pub fn load() -> Settings {
        info!("Loading settings...");

        let settings_path = Path::new("./settings.toml");
        if !settings_path.exists() {
            warn!("Settings file not exists! creating it and exiting...");
            File::create(settings_path).unwrap();
            std::process::exit(1);
        }
        let settings_file = File::open("settings.toml").unwrap();
        let mut reader = BufReader::new(settings_file);

        let mut settings_buffer = String::new();
        reader.read_to_string(&mut settings_buffer).unwrap();
        let settings: Settings = toml::from_str(&settings_buffer.as_str()).unwrap();

        debug!("Settings: {:?}", settings);
        info!("Settings loaded.");

        settings
    }
}

