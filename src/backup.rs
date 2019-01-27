use std;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use tar::Builder;

pub fn start(targets: &[Target],
             filters: &[Filter],
             commands_after_backup: &[Vec<String>],
             archiver: &mut Builder<File>,
             archive_path: &str) {
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
            info!("Current path: {}", path.to_str().expect("Failed to got path"));
            let path_length = path.to_str().expect("Failed to got path").len();
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
    archiver.finish().expect("Error occurred while finishing.");
    info!("Backup finished! ({} files)", complete_count);

    if commands_after_backup.is_empty() {
        return;
    }

    info!("Executing commands...");

    for command in commands_after_backup {
        if command.is_empty() {
            continue;
        }

        let mut args = command.iter();
        let mut args_appended = String::new();
        let first_arg = args.next().unwrap();
        let mut command = Command::new(first_arg);
        args_appended.push_str(first_arg);
        for arg in args {
            let arg = arg.replace("%archive_path%", archive_path);
            let arg_str = arg.as_str();
            command.arg(arg_str);
            args_appended.push(' ');
            args_appended.push_str(arg_str);
        }

        info!("Executing {}", args_appended);
        let exit_status = command
            .spawn()
            .expect("failed to run command.")
            .wait()
            .expect("Execute failed!");
        info!("Executed in exit code {}", exit_status.code().unwrap());
    }
}

fn execute_path(path_prefix: &str,
                filters: &[&Filter],
                entry_path: &PathBuf,
                root_path_len: usize,
                archiver: &mut Builder<File>,
                listener: &mut FnMut()) {
    for filter in filters {
        let entry_path_parent = entry_path.parent().expect("Failed to get parent directory!").to_path_buf();
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
                    info!("Filter: {} applied to path: {}", filter.name, entry_path.to_str().expect("Failed to got path"));
                    return;
                }
            }
        }
    }

    if !entry_path.is_dir() {
        while execute_file(path_prefix,
                           entry_path,
                           root_path_len,
                           archiver, listener) == Action::Retry {}

        return;
    }

    let entry_iterator = match fs::read_dir(entry_path) {
        Ok(iterator) => {
            iterator
        }
        Err(error) => {
            warn!("Cannot iterate entries in \"{}\". message: {}",
                  entry_path.to_str().expect("Failed to got path"), error.description());
            return;
        }
    };

    for entry in entry_iterator {
        execute_path(path_prefix,
                     filters,
                     &entry.expect("Error occurred while iterating entry!").path(),
                     root_path_len,
                     archiver,
                     listener);
    }
}

fn execute_file(path_prefix: &str,
                entry_path: &Path,
                root_path_len: usize,
                archiver: &mut Builder<File>,
                listener: &mut FnMut()) -> Action {
    let entry_path_str = entry_path.to_str().expect("Failed to got path");
    trace!("Archiving: {}", entry_path_str);

    let mut archive_path = path_prefix.to_string();
    let entry_path_str_len = entry_path_str.len();
    if entry_path_str_len == root_path_len {
        archive_path.push('/');
        archive_path.push_str(entry_path.file_name().expect("Failed to got file name").to_str().expect("Failed to convert OsStr to str"));
    } else {
        archive_path.push_str(&entry_path_str[root_path_len..entry_path_str_len]);
    }

    let file = File::open(entry_path);
    let file = unwrap_or_confirm(file,
                                 || format!("Failed to open \"{}\"", entry_path_str));
    let mut file = match file {
        Ok(value) => {
            value
        },
        Err(action) => {
            return action;
        },
    };

    let archive_result = archiver.append_file(archive_path, &mut file);
    let archive_result = unwrap_or_confirm(archive_result,
                                           || format!("Failed to archive \"{}\"", entry_path_str));
    if let Err(action) = archive_result {
        return action;
    };

    trace!("Archived: {}", entry_path_str);

    listener();
    Action::IgnoreOrContinue
}

fn unwrap_or_confirm<T, F>(result: std::io::Result<T>, error_message_func: F) -> Result<T, Action>
    where F: FnOnce() -> String {
    match result {
        Ok(value) => {
            Result::Ok(value)
        },
        Err(error) => {
            warn!("{} : {}", error_message_func(), error);
            warn!("(E)xit/(I)gnore/(R)etry");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).expect("Failed to read line!");
            match input.trim().to_uppercase().as_str() {
                "I" => {
                    info!("Ignoring...");
                    Result::Err(Action::IgnoreOrContinue)
                },
                "R" => {
                    info!("Retrying...");
                    Result::Err(Action::Retry)
                },
                _ => {
                    info!("Exiting...");
                    std::process::exit(0);
                },
            }
        }
    }
}

#[derive(PartialEq)]
enum Action {
    Retry,
    IgnoreOrContinue
}

pub struct Target {
    pub name: String,
    pub paths: Vec<PathBuf>,
}

pub struct Filter {
    pub name: String,
    pub scopes: Vec<String>,
    pub targets: Vec<PathBuf>,
    pub conditions: Vec<Condition>,
}

pub struct Condition {
    pub not: bool,
    pub path: PathBuf,
}