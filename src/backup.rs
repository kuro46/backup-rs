use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;

use tar::Builder;

pub fn start(targets: Vec<Target>,
             _filters: Vec<Filter>,
             archiver: &mut Builder<File>) {
    info!("Backup started!");

    let mut complete_count: u64 = 0;
    for target in targets {
        let mut path_prefix = target.name.clone();
        path_prefix.push('/');
        let path_prefix = path_prefix.as_str();

        info!("Current target: {}", target.name.as_str());

        for path in target.paths {
            info!("Current path: {}", path);

            let path = Path::new(&path).canonicalize().unwrap();
            let path_length = path.to_str().unwrap().len();
            execute_path(path_prefix,
                         &path,
                         path_length,
                         archiver, &mut || {
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

fn execute_path(path_prefix: &str,
                entry_path: &Path,
                root_path_len: usize,
                archiver: &mut Builder<File>,
                listener: &mut FnMut()) {
    if !entry_path.is_dir() {
        execute_file(path_prefix,
                     entry_path,
                     root_path_len,
                     archiver, listener);
        return;
    }

    let entry_iterator = match fs::read_dir(entry_path) {
        Ok(iterator) => {
            iterator
        }
        Err(error) => {
            warn!("Cannot iterate entries in \"{}\". message: {}",
                  entry_path.to_str().unwrap(), error.description());
            return;
        }
    };

    for entry in entry_iterator {
        execute_path(path_prefix,
                     &entry.unwrap().path(),
                     root_path_len,
                     archiver,
                     listener);
    }
}

fn execute_file(path_prefix: &str,
                entry_path: &Path,
                root_path_len: usize,
                archiver: &mut Builder<File>,
                listener: &mut FnMut()) {
    let entry_path_str = entry_path.to_str().unwrap();
    trace!("Archiving: {}", entry_path_str);

    let mut archive_path = path_prefix.to_string();
    let entry_path_str_len = entry_path_str.len();
    if entry_path_str_len == root_path_len {
        archive_path.push_str(entry_path.file_name().unwrap().to_str().unwrap());
    } else {
        archive_path.push_str(&entry_path_str[root_path_len..entry_path_str_len]);
    }

    archiver.append_file(archive_path, &mut File::open(entry_path).unwrap()).unwrap();

    trace!("Archived: {}", entry_path_str);

    listener();
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