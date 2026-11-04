use crate::Result;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub fn home() -> Option<PathBuf> {
    std::env::home_dir()
}

/// Resolve também raízes ainda inexistentes, usadas por fp init.
pub fn locate(
    explicit: Option<&Path>,
    env: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<PathBuf> {
    let selected = explicit
        .map(PathBuf::from)
        .or_else(|| env.map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("~/fiap"));
    let path = if selected == Path::new("~") || selected.starts_with("~/") {
        let home = home.ok_or("Diretório pessoal indisponível; informe --root")?;
        PathBuf::from(home).join(selected.strip_prefix("~")?)
    } else {
        selected
    };
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => {
                normalized.push(other.as_os_str());
                match std::fs::symlink_metadata(&normalized) {
                    Ok(_) => normalized = normalized.canonicalize()?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
    Ok(normalized)
}

pub fn resolve(
    explicit: Option<&Path>,
    env: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<PathBuf> {
    let root = locate(explicit, env, home)?;
    if !root.is_dir() || !root.join("Justfile").is_file() || !root.join("chapters.toml").is_file() {
        return Err(
            "A raiz deve conter Justfile e chapters.toml compatíveis com o monorepo FIAP".into(),
        );
    }
    crate::registry::load(&root)?;
    Ok(root)
}
