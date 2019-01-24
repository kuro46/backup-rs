use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use tar::Builder;

pub fn start(targets: &[Target],
             filters: &[Filter],
             archiver: &mut Builder<File>) {
    info!("Backup started!");

    let mut complete_count: u64 = 0;
    for target in targets {
        let filters_for_target: Vec<&Filter> = filters.iter()
            .filter(|filter| {
                filter.scopes.contains(&target.name)
                    || filter.scopes.contains(&"global".to_string())
            })
            .collect();
        let filters_for_target = filters_for_target.as_slice();

        let path_prefix = target.name.as_str();
        info!("Current target: {}", path_prefix);

        for path in &target.paths {
            info!("Current path: {}", path.to_str().unwrap());
            let path_length = path.to_str().unwrap().len();
            execute_path(path_prefix,
                         filters_for_target,
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
                filters: &[&Filter],
                entry_path: &PathBuf,
                root_path_len: usize,
                archiver: &mut Builder<File>,
                listener: &mut FnMut()) {
    for filter in filters {
        let entry_path_parent = entry_path.parent().unwrap().to_path_buf();
        for target in &filter.targets {
            let mut target_appended = entry_path_parent.clone();
            target_appended.push(target);
            let target_appended = target_appended;

            if !target_appended.eq(entry_path) {
                continue;
            }

            for condition in &filter.conditions {
                let mut condition_path = entry_path_parent.clone();
                condition_path.push(&condition.path);
                let condition_path = condition_path;

                let found = condition_path.exists();
                if (found && !condition.not) || (!found && condition.not) {
                    info!("Filter: {} applied to path: {}", filter.name, entry_path.to_str().unwrap());
                    return;
                }
            }
        }
    }

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
                     filters,
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
        archive_path.push('/');
        archive_path.push_str(entry_path.file_name().unwrap().to_str().unwrap());
    } else {
        archive_path.push_str(&entry_path_str[root_path_len..entry_path_str_len]);
    }

    archiver.append_file(archive_path, &mut File::open(entry_path).unwrap()).unwrap();

    trace!("Archived: {}", entry_path_str);

    listener();
}

pub struct Target {
    pub name: String,
    pub paths: Vec<PathBuf>,
}

pub struct Filter {
    pub name: String,
    pub filter_type: FilterType,
    pub scopes: Vec<String>,
    pub targets: Vec<PathBuf>,
    pub conditions: Vec<Condition>,
}

pub struct Condition {
    pub not: bool,
    pub path: PathBuf,
}

pub enum FilterType {
    Exclude,
//Implement in later
//    Include,
}

impl FilterType {
    pub fn from_str(type_str: &str) -> Option<FilterType> {
        match type_str {
            "exclude" => Some(FilterType::Exclude),
//            "include" => Some(FilterType::Include),
            _ => None
        }
    }
}