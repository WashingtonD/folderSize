use std::env;
use std::fs;
use std::io::{self, stdout, Write};
use std::path::Path;

use crossterm::{
    cursor,
    execute,
    style::{Stylize},
    terminal::{self, ClearType},
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum EntryType {
    File,
    Directory,
}

impl Iterator for &EntryType {
    type Item = EntryType;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EntryType::File => {
                *self = &EntryType::Directory;
                Some(EntryType::File)
            }
            EntryType::Directory => None,
        }
    }
}

struct EntryInfo {
    entry_type: EntryType,
    path: String,
    size: Option<u64>,
}

fn get_entry_info(entry: fs::DirEntry) -> io::Result<EntryInfo> {
    let path = entry.path();
    let metadata = entry.metadata()?;
    let entry_type = if metadata.is_file() {
        EntryType::File
    } else if metadata.is_dir() {
        EntryType::Directory
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Unsupported entry type",
        ));
    };

    let size = if entry_type == EntryType::File {
        Some(metadata.len())
    } else if entry_type == EntryType::Directory {
        Some(get_directory_size(&path)?)
    } else {
        None
    };

    Ok(EntryInfo {
        entry_type,
        path: path.to_string_lossy().to_string(),
        size,
    })
}

fn get_directory_size(path: &Path) -> io::Result<u64> {
    let mut total_size = 0;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            total_size += metadata.len();
        } else if metadata.is_dir() {
            total_size += get_directory_size(&entry.path())?;
        }
    }

    Ok(total_size)
}

fn get_entries_info(dir_path: &str) -> io::Result<Vec<EntryInfo>> {
    let mut entries_info = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let entry_info = get_entry_info(entry)?;
        entries_info.push(entry_info);
    }

    entries_info.sort_by(|a, b| {
        if a.entry_type == b.entry_type {
            std::cmp::Ord::cmp(&b.size.unwrap_or(0), &a.size.unwrap_or(0))
        } else {
            std::cmp::Ord::cmp(&a.entry_type, &EntryType::Directory)
        }
    });

    Ok(entries_info)
}

fn display_entries_info(entries_info: &[EntryInfo]) {
    let total_entries = entries_info.len();
    let max_size = entries_info
        .iter()
        .filter_map(|entry| entry.size)
        .max()
        .unwrap_or(1); // To avoid division by zero if no sizes available

    for (index, entry_info) in entries_info.iter().enumerate() {
        let progress_bar_length = 60;
        let entry_name = entry_info.path.rsplit('/').next().unwrap();
        let entry_type_str = match entry_info.entry_type {
            EntryType::File => "F",
            EntryType::Directory => "D",
        };

        if let Some(size) = entry_info.size {
            let progress = (size as f64 / max_size as f64 * progress_bar_length as f64) as usize;
            let progress_bar = format!("{:=<1$}", "", progress).cyan();
            let size_str = format_size(size);
            println!(
                "{:<3} {} [{}] {} [{}]",
                index + 1,
                entry_type_str,
                progress_bar,
                entry_name,
                size_str
            );
        } else {
            println!("{:<3} {} {}", index + 1, entry_type_str, entry_name);
        }
    }

    println!("\nTotal entries: {}", total_entries);
}

fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    if size < KB as u64 {
        format!("{} B", size)
    } else if size < MB as u64 {
        format!("{:.2} KB", size as f64 / KB)
    } else if size < GB as u64 {
        format!("{:.2} MB", size as f64 / MB)
    } else if size < TB as u64 {
        format!("{:.2} GB", size as f64 / GB)
    } else {
        format!("{:.2} TB", size as f64 / TB)
    }
}

fn clear_console() {
    execute!(
        stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )
    .unwrap();
}

fn prompt_user() -> Result<isize, ()> {
    print!("Please enter the number of the directory you want to analyze (-1 to go back): ");
    stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().parse::<isize>().or_else(|_| {
        println!("Invalid input. Please enter a valid number.");
        prompt_user()
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run <directory_path>");
        return;
    }

    let mut current_dir = args[1].clone();
    let mut dir_stack = vec![current_dir.clone()];

    loop {
        clear_console();
        println!("Analyzing entries in directory: {}\n", current_dir);

        let entries_info = match get_entries_info(&current_dir) {
            Ok(entries_info) => entries_info,
            Err(error) => {
                println!("Error: {}", error);
                return;
            }
        };

        display_entries_info(&entries_info);

        println!();
        let choice = prompt_user().unwrap();

        match choice {
            -1 => {
                if let Some(prev_dir) = dir_stack.pop() {
                    current_dir = prev_dir;
                }
            }
            0 => {
                break;
            }
            index if (1..=entries_info.len() as isize).contains(&index) => {
                let selected_entry = &entries_info[(index - 1) as usize];
                if selected_entry.entry_type == EntryType::Directory {
                    dir_stack.push(current_dir.clone());
                    current_dir = selected_entry.path.clone();
                }
            }
            _ => {
                println!("Invalid choice. Please enter a valid number.");
            }
        }
    }

    clear_console();
    println!("Exiting the program.");
}