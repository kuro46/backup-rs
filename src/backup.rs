use std;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Result as IOResult;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use console::Term;
use tar::Builder;

pub fn start(targets: &[Target],
             filters: &[Filter],
             archiver: &mut Builder<File>) {
    info!("Backup started!");

    let mut terminal = Term::stdout();
    let mut complete_count: u64 = 0;

    for target in targets {
        let target_name = target.name.as_str();

        let filters = get_filters(target_name, filters);
        let filters = filters.as_slice();

        for path in &target.paths {
            let path_length = path.to_str().expect("Failed to got path").len();

            let mut on_file_detected = |path: &PathBuf| {
                let mut on_archived = |path: &String| {
                    complete_count += 1;

                    update_status_bar(complete_count,
                                      target_name,
                                      &mut terminal,
                                      path.as_str());
                };

                while execute_file(target_name,
                                   path,
                                   path_length,
                                   archiver,
                                   &mut on_archived) == Action::Retry {};
            };

            execute_path(filters,
                         &path,
                         &mut on_file_detected);
        }
    }
    terminal.clear_line().unwrap();

    info!("Backup finished! ({} files)", complete_count);
}

fn update_status_bar(file_count: u64,
                     target_name: &str,
                     terminal: &mut Term,
                     path: &str) {
    let mut formatted = format!("files: {} target: \"{}\" path: \"{}\"",
                                file_count,
                                target_name,
                                path);

    //Trim or push space
    {
        let mut formatted_chars = formatted.chars();
        let mut trimmed = String::new();
        for _ in 0..terminal.size().1 {
            let next_char = formatted_chars.next();
            if let Some(next_char) = next_char {
                trimmed.push(next_char);
            } else {
                trimmed.push(' ');
            }
        }
        formatted = trimmed;
    }

    formatted.push('\r');
    terminal.write_string(formatted.as_str()).unwrap();
}

fn get_filters<'a>(target_name: &'a str,
                   filters: &'a [Filter]) -> Vec<&'a Filter> {
    let target_name = &target_name.to_string();
    let global = &"global".to_string();
    filters.iter()
        .filter(|filter|
            filter.scopes.contains(target_name) || filter.scopes.contains(global))
        .collect()
}

fn execute_path<F>(filters: &[&Filter],
                   entry_path: &PathBuf,
                   on_file_detected: &mut F) where F: FnMut(&PathBuf) {
    for filter in filters {
        if is_filterable(filter, entry_path) {
            info!("Filter: {} applied to path: {}",
                  filter.name,
                  entry_path.to_str().expect("Failed to got path"));
            return;
        }
    }

    if !entry_path.is_dir() {
        on_file_detected(entry_path);
        return;
    }

    let entry_iterator = match fs::read_dir(entry_path) {
        Ok(iterator) => iterator,
        Err(error) => {
            warn!("Cannot iterate entries in \"{}\". message: {}",
                  entry_path.to_str().expect("Failed to got path"), error.description());
            return;
        },
    };

    for entry in entry_iterator {
        execute_path(filters,
                     &entry.expect("Error occurred while iterating entry!").path(),
                     on_file_detected);
    }
}

fn is_filterable(filter: &Filter,
                 path: &PathBuf) -> bool {
    let entry_path_parent = path.parent().expect("Failed to get parent directory!").to_path_buf();
    for target in &filter.targets {
        let mut target_appended = entry_path_parent.clone();
        target_appended.push(target);
        let target_appended = target_appended;

        if !target_appended.eq(path) {
            continue;
        }

        for condition in &filter.conditions {
            let mut condition_path = entry_path_parent.clone();
            condition_path.push(&condition.path);
            let condition_path = condition_path;

            let found = condition_path.exists();
            if (found && !condition.not) || (!found && condition.not) {
                return true;
            }
        }
    }

    false
}

fn execute_file<F>(path_prefix: &str,
                   entry_path: &Path,
                   root_path_len: usize,
                   archiver: &mut Builder<File>,
                   on_archived: &mut F) -> Action where F: FnMut(&String) {
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

    let archive_result = archiver.append_file(&archive_path, &mut file);
    let archive_result = unwrap_or_confirm(archive_result,
                                           || format!("Failed to archive \"{}\"", entry_path_str));
    if let Err(action) = archive_result {
        return action;
    };

    trace!("Archived: {}", entry_path_str);

    on_archived(&archive_path);
    Action::IgnoreOrContinue
}

fn unwrap_or_confirm<T, F>(result: IOResult<T>,
                           error_message_func: F) -> Result<T, Action> where F: FnOnce() -> String {
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

pub fn execute_commands(commands: &[Vec<String>],
                        archive_path: &str) {
    info!("Executing commands...");

    for command in commands {
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

    info!("Commands were executed.");
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

trait WriteStr {
    fn write_string(&mut self, string: &str) -> IOResult<usize>;
}

impl WriteStr for Term {
    fn write_string(&mut self, string: &str) -> IOResult<usize> {
        self.write(string.as_bytes())
    }
}