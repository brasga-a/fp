use crate::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Entry {
    pub number: u32,
    pub path: PathBuf,
    pub label: String,
}

pub fn discover(parent: &Path, prefix: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        // Não atravessar links para outros repositórios.
        if !entry.file_type()?.is_dir()
            || crate::identity::is_link(&fs::symlink_metadata(entry.path())?)
        {
            continue;
        }
        let label = entry.file_name().to_string_lossy().into_owned();
        if let Some(rest) = label.strip_prefix(prefix) {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            let suffix = &rest[digits.len()..];
            if !(suffix.is_empty() || suffix.starts_with('-')) {
                continue;
            }
            if let Ok(number) = digits.parse::<u32>() {
                if number == 0 {
                    continue;
                }
                if entries.iter().any(|e: &Entry| e.number == number) {
                    return Err(
                        format!("Numeração ambígua em {}: {number}", parent.display()).into(),
                    );
                }
                entries.push(Entry {
                    number,
                    path: entry.path(),
                    label,
                });
            }
        }
    }
    entries.sort_by_key(|e| e.number);
    Ok(entries)
}

pub fn next(entries: &[Entry]) -> Result<u32> {
    entries
        .iter()
        .map(|e| e.number)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "Numeração esgotada".into())
}

pub fn dirname(prefix: &str, number: u32, entries: &[Entry]) -> String {
    let width = entries
        .iter()
        .filter_map(|e| e.label.strip_prefix(prefix))
        .map(|s| s.chars().take_while(char::is_ascii_digit).count())
        .max()
        .unwrap_or(2);
    format!("{prefix}{number:0width$}")
}

pub fn technology(path: &Path) -> &'static str {
    if path.join("platformio.ini").is_file() {
        "ESP32 / IoT"
    } else if path.join("Cargo.toml").is_file() {
        "Rust"
    } else if path.join("pyproject.toml").is_file() {
        "Python"
    } else if path.join("package.json").is_file() {
        "TypeScript / JavaScript"
    } else if path.join("sql").is_dir() {
        "SQL / Database"
    } else {
        "Pasta / não identificada"
    }
}
