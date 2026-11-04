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
            // Um prefixo Windows isolado (especialmente \\?\C:) ainda não é
            // um caminho consultável. Aguarde RootDir antes de acessar o disco.
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn drive_and_verbatim_prefixes_are_not_queried_before_root() {
        let directory = tempfile::tempdir().unwrap();
        let canonical = directory.path().canonicalize().unwrap();
        for path in [directory.path().to_path_buf(), canonical.clone()] {
            assert_eq!(locate(Some(&path), None, None).unwrap(), canonical);
            assert_eq!(
                locate(Some(&path.join("ação com espaços")), None, None).unwrap(),
                canonical.join("ação com espaços")
            );
        }
    }
}
