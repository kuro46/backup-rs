use std;
use std::fs;
use std::fs::File;
use std::io::Result as IOResult;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tar::Builder;

pub fn start(targets: &[Target],
             filters: &[Filter],
             archiver: &mut Builder<File>) {
    for target in targets {
        let target_name = target.name.as_str();

        let filters = get_filters(target_name, filters);
        let filters = filters.as_slice();

        print!(
            "Backing up target: {}\nFilters:",
            target_name);
        if filters.is_empty() {
            println!(" NONE");
        } else {
            println!();
            for filter in filters {
                println!("\t- {}", &filter.name);
            }
        }

        for path in &target.paths {
            let root_path_length = path.to_str().expect("Failed to got path").len();

            let mut stack: Vec<PathBuf> = Vec::new();
            stack.push(path.clone());
            'iterate_path: while let Some(path) = stack.pop() {
                if !path.is_dir() {
                    execute_file(&path, target_name,
                                 root_path_length,
                                 archiver);
                    continue;
                }

                let read_dir = match fs::read_dir(&path) {
                    Ok(read_dir) => read_dir,
                    Err(error) => {
                        eprintln!("Failed to iterate entries in \"{}\". Ignoring it.\nError: {}",
                                  path.to_str().expect("Failed to got path"), error);
                        continue;
                    },
                };

                for path in read_dir {
                    let path = path.unwrap().path();

                    for filter in filters {
                        if is_filterable(filter, &path) {
                            continue 'iterate_path;
                        }
                    }

                    stack.push(path);
                }
            }
        }
    }

    println!("Backup finished!");
}

fn execute_file(path: &Path,
                target_name: &str,
                root_path_length: usize,
                archiver: &mut Builder<File>) {
    let entry_path_str = path.to_str().expect("Failed to got path");
    let mut archive_path = target_name.to_string();
    let entry_path_str_len = entry_path_str.len();
    if entry_path_str_len == root_path_length {
        archive_path.push('/');
        archive_path.push_str(path.file_name().expect("Failed to got file name")
            .to_str().expect("Failed to convert OsStr to str"));
    } else {
        archive_path.push_str(&entry_path_str[root_path_length..entry_path_str_len]);
    }

    let file = unwrap_or_confirm(File::open(&path),
                                 || format!("Failed to open \"{}\"", entry_path_str));
    let mut file = match file {
        Ok(value) => value,
        Err(action) => {
            if action != Action::Retry {
                return;
            } else {
                execute_file(path,
                             target_name,
                             root_path_length,
                             archiver);
                return;
            }
        },
    };

    let archive_result = unwrap_or_confirm(archiver.append_file(&archive_path, &mut file),
                                           || format!("Failed to archive \"{}\"", entry_path_str));
    if let Err(action) = archive_result {
        if action != Action::Retry {
            return;
        } else {
            execute_file(path,
                         target_name,
                         root_path_length,
                         archiver);
            return;
        }
    };
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

fn is_filterable(filter: &Filter,
                 path: &PathBuf) -> bool {
    let entry_path_parent = path.parent()
        .expect("Failed to get parent directory!")
        .to_path_buf();
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

fn unwrap_or_confirm<T, F>(result: IOResult<T>,
                           error_message_func: F) -> Result<T, Action> where F: FnOnce() -> String {
    match result {
        Ok(value) => {
            Result::Ok(value)
        },
        Err(error) => {
            println!("{} : {}", error_message_func(), error);

            //Lock and unlock stdout
            {
                let stdout = std::io::stdout();
                let mut stdout = stdout.lock();

                stdout.write_all(b"(E)xit, (I)gnore, (R)etry: ").unwrap();
                stdout.flush().unwrap();
            }

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).expect("Failed to read line!");
            match input.trim().to_ascii_lowercase().as_str() {
                "i" => {
                    println!("Ignoring...");
                    Result::Err(Action::IgnoreOrContinue)
                },
                "r" => {
                    println!("Retrying...");
                    Result::Err(Action::Retry)
                },
                _ => {
                    println!("Exiting...");
                    std::process::exit(0);
                },
            }
        }
    }
}

pub fn execute_commands(commands: &[Vec<String>],
                        archive_path: &str) {
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

        let exit_status = command
            .spawn()
            .expect("failed to run command.")
            .wait()
            .expect("Execute failed!");
        println!("Executed in exit code {}", exit_status.code().unwrap());
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