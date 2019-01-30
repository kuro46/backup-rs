use std;
use std::fs;
use std::fs::File;
use std::io::Result as IOResult;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use console::Term;
use tar::Builder;

pub fn start(targets: &[Target],
             filters: &[Filter],
             archiver: &mut Builder<File>) {
    let mut terminal = Term::stdout();
    let mut complete_count: u64 = 0;
    let mut skip_count: u64 = 0;

    for target in targets {
        let target_name = target.name.as_str();

        let filters = get_filters(target_name, filters);
        let filters = filters.as_slice();

        for path in &target.paths {
            let root_path_length = path.to_str().expect("Failed to got path").len();

            let mut stack: Vec<PathBuf> = Vec::new();
            stack.push(path.clone());

            'iterate_path: while let Some(path) = stack.pop() {
                if path.is_dir() {
                    for path in fs::read_dir(&path).unwrap() {
                        let path = path.unwrap().path();

                        for filter in filters {
                            if is_filterable(filter, &path) {
                                debug!("Filter: {} applied to path: {}",
                                       filter.name,
                                       path.to_str().expect("Failed to got path"));
                                skip_count += 1;
                                continue 'iterate_path;
                            }
                        }

                        stack.push(path);
                    }

                    continue;
                }

                execute_file(&path, target_name,
                             root_path_length,
                             archiver,
                             &mut complete_count,
                             &mut skip_count,
                             &mut terminal);
            }
        }

        terminal.clear_line().unwrap();
        terminal.write_line(&format!("Backed up target: {}", target_name)).unwrap();
    }
    terminal.clear_line().unwrap();

    println!("Backup finished! ({} files)", complete_count);
}

fn execute_file(path: &Path,
                target_name: &str,
                root_path_length: usize,
                archiver: &mut Builder<File>,
                complete_count: &mut u64,
                skip_count: &mut u64,
                terminal: &mut Term) {
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
        Ok(value) => {
            value
        },
        Err(action) => {
            if action != Action::Retry {
                return;
            } else {
                execute_file(path,
                             target_name,
                             root_path_length,
                             archiver,
                             complete_count,
                             skip_count,
                             terminal);
                return;
            }
        },
    };

    let archive_result = unwrap_or_confirm(archiver.append_file(&archive_path, &mut file),
                                           || format!("Failed to archive \"{}\"", entry_path_str));
    if let Err(action) = archive_result {
        if action != Action::Retry {
            if action != Action::Retry {
                return;
            } else {
                execute_file(path,
                             target_name,
                             root_path_length,
                             archiver,
                             complete_count,
                             skip_count,
                             terminal);
                return;
            }
        }
    };

    *complete_count += 1;

    update_status_bar(*complete_count,
                      *skip_count,
                      terminal);
}

fn update_status_bar(file_count: u64,
                     skip_count: u64,
                     terminal: &mut Term) {
    let mut formatted = format!("\rcompleted: {} skipped: {}",
                                file_count,
                                skip_count, );

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
            println!("(E)xit/(I)gnore/(R)etry");

            let mut input = String::new();
            std::io::stdin().read_line(&mut input).expect("Failed to read line!");
            match input.trim().to_uppercase().as_str() {
                "I" => {
                    println!("Ignoring...");
                    Result::Err(Action::IgnoreOrContinue)
                },
                "R" => {
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

trait WriteStr {
    fn write_string(&mut self, string: &str) -> IOResult<usize>;
}

impl WriteStr for Term {
    fn write_string(&mut self, string: &str) -> IOResult<usize> {
        self.write(string.as_bytes())
    }
}