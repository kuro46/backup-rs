use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use tar::Builder;

pub fn start(settings: Settings, archiver: &mut Builder<File>) {
    info!("Backup started!");

    let mut complete_count: u64 = 0;
    for target in settings.targets {
        let target_name_string = target.name.clone();
        let target_name = target_name_string.as_str();

        info!("Current target: {}", target_name);

        for path_str in target.paths {
            let path_str = path_str.as_str();
            info!("Current path: {}", path_str);

            let path = Path::new(path_str);
            execute_path(path, path.to_str().unwrap(), target_name, archiver, &mut || {
                complete_count += 1;
                if complete_count % 1000 == 0 {
                    info!("{} files completed.", complete_count);
                }
            });
        }
    }

    info!("Finishing...");
    archiver.finish().unwrap();
    info!("Backup finished! ({} files)", complete_count);
}

fn execute_path(path: &Path, root_path: &str, target_name: &str, archiver: &mut Builder<File>, listener: &mut FnMut()) {
    if !path.is_dir() {
        execute_file(path.to_str().unwrap(),
                     root_path,
                     target_name,
                     archiver, listener);
        return;
    }

    let entry_iterator = match fs::read_dir(path) {
        Ok(iterator) => {
            iterator
        },
        Err(error) => {
            warn!("Cannot iterate entries in \"{}\". message: {}",
                  path.to_str().unwrap(), error.description());
            return;
        },
    };

    for entry in entry_iterator {
        let entry = entry.unwrap();
        let entry_path_buf = entry.path();

        execute_path(&entry_path_buf, root_path, target_name, archiver, listener);
    }
}

fn execute_file(entry_path_string: &str,
                root_path: &str,
                target_name: &str,
                archiver: &mut Builder<File>,
                listener: &mut FnMut()) {
    trace!("Archiving: {}", entry_path_string);

    let mut archive_path = target_name.to_string();
    archive_path.push('/');
    if entry_path_string.eq(root_path) {
        archive_path.push_str(root_path);
    } else {
        archive_path.push_str(entry_path_string.to_string().replace(root_path, "").as_str());
    }

    archiver.append_file(archive_path, &mut File::open(entry_path_string).unwrap()).unwrap();

    trace!("Archived: {}", entry_path_string);

    listener();
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub archive_path: String,
    pub targets: Vec<Target>,
    pub filters: Vec<Filter>,
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

#[derive(Deserialize, Debug)]
pub struct Target {
    pub name: String,
    pub paths: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Filter {
    pub name: String,
    pub execute: String,
    pub scopes: Vec<String>,
    pub targets: Vec<String>,
    pub conditions: Vec<String>,
}