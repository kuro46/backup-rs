#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

use chrono::Utc;
use env_logger;
use tar::Builder;

use backup::Settings;

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

    backup::start(settings, &mut archiver);
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


