use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use tar::Builder;

pub fn start(targets: Vec<Target>,
             filters: Vec<Filter>,
             archiver: &mut Builder<File>) {
    info!("Backup started!");

    let mut complete_count: u64 = 0;
    for target in &targets {
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
        for target in &filter.targets {
            let mut target_appended = entry_path.parent().unwrap().to_path_buf();
            target_appended.push(target);
            let target_appended = target_appended.as_path();

            if !target_appended.eq(entry_path) {
                continue;
            }

            for condition in &filter.conditions {
                let mut condition_path = entry_path.parent().unwrap().to_path_buf();
                condition_path.push(&condition.path);
                let condition_path = condition_path.as_path();

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
                     &dunce::canonicalize(entry.unwrap().path()).unwrap(),
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

#[derive(Deserialize, Debug)]
pub struct TargetSetting {
    pub name: String,
    pub paths: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct FilterSetting {
    pub name: String,
    pub execute: String,
    pub scopes: Vec<String>,
    pub targets: Vec<String>,
    pub conditions: Vec<String>,
}

pub struct Target {
    pub name: String,
    pub paths: Vec<PathBuf>,
}

impl Target {
    pub fn from_setting(setting: TargetSetting) -> Target {
        let paths: Vec<PathBuf> = setting.paths.iter()
            .map(|path| dunce::canonicalize(Path::new(path)).unwrap())
            .collect();

        Target {
            name: setting.name,
            paths,
        }
    }
}

pub struct Filter {
    pub name: String,
    pub filter_type: FilterType,
    pub scopes: Vec<String>,
    pub targets: Vec<PathBuf>,
    pub conditions: Vec<Condition>,
}

impl Filter {
    pub fn from_setting(setting: FilterSetting) -> Filter {
        let targets: Vec<PathBuf> = setting.targets.iter()
            .map(|path| Path::new(path).to_path_buf())
            .collect();
        let conditions: Vec<Condition> = setting.conditions.iter().map(|condition_str| {
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

        Filter {
            name: setting.name,
            filter_type: FilterType::from_str(setting.execute.as_str()).unwrap(),
            scopes: setting.scopes,
            targets,
            conditions,
        }
    }
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