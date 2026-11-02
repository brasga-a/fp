use crate::{Result, processes, registry};
use std::{
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub const DIRECTORIES: &[&str] = &["docs", "scripts", "tests"];
pub const FILES: &[(&str, &str)] = &[
    (
        "AGENTS.md",
        include_str!("../../../assets/workspace/AGENTS.md"),
    ),
    (
        "README.md",
        include_str!("../../../assets/workspace/README.md"),
    ),
    (
        "Justfile",
        include_str!("../../../assets/workspace/Justfile"),
    ),
    (
        "chapters.toml",
        include_str!("../../../assets/workspace/chapters.toml"),
    ),
    (
        "pyproject.toml",
        include_str!("../../../assets/workspace/pyproject.toml"),
    ),
    (
        ".gitignore",
        include_str!("../../../assets/workspace/gitignore"),
    ),
    (
        "docs/README.md",
        include_str!("../../../assets/workspace/docs/README.md"),
    ),
    (
        "scripts/chapters.py",
        include_str!("../../../assets/workspace/scripts/chapters.py"),
    ),
    (
        "tests/test_chapters.py",
        include_str!("../../../assets/workspace/tests/test_chapters.py"),
    ),
];

#[derive(Debug, Default)]
pub struct Inspection {
    pub missing: Vec<PathBuf>,
    pub problems: Vec<String>,
}

fn inspect_path(path: &Path, directory: bool, report: &mut Inspection) {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            report.missing.push(path.to_path_buf())
        }
        Err(e) => report.problems.push(format!("{}: {e}", path.display())),
        Ok(m) if m.is_symlink() => report.problems.push(format!(
            "{}: link simbólico preservado; resolva este caminho manualmente",
            path.display()
        )),
        Ok(m) if (directory && m.is_dir()) || (!directory && m.is_file()) => {}
        Ok(_) => report.problems.push(format!(
            "{}: esperado {}",
            path.display(),
            if directory { "diretório" } else { "arquivo" }
        )),
    }
}

pub fn inspect(root: &Path) -> Inspection {
    let mut report = Inspection::default();
    inspect_path(root, true, &mut report);
    for dir in DIRECTORIES {
        inspect_path(&root.join(dir), true, &mut report);
    }
    for (file, _) in FILES {
        inspect_path(&root.join(file), false, &mut report);
    }
    if report.problems.is_empty() {
        if root.join("chapters.toml").is_file()
            && let Err(e) = registry::load(root)
        {
            report.problems.push(format!("chapters.toml: {e}"));
        }
        if root.join("pyproject.toml").is_file() {
            let parsed = fs::read_to_string(root.join("pyproject.toml"))
                .map_err(|e| e.to_string())
                .and_then(|s| {
                    s.parse::<toml_edit::DocumentMut>()
                        .map_err(|e| e.to_string())
                });
            if let Err(e) = parsed {
                report.problems.push(format!("pyproject.toml: {e}"));
            }
        }
    }
    report
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Operação aditiva: nunca substitui configurações nem cria projetos acadêmicos.
pub fn initialize(root: &Path) -> Result<Vec<PathBuf>> {
    let preflight = inspect(root);
    if !preflight.problems.is_empty() {
        return Err(preflight.problems.join("\n").into());
    }
    let mut created = Vec::new();
    if !root.exists() {
        fs::create_dir_all(root)?;
        created.push(root.to_path_buf());
    }
    let lock_path = root.join(".fp-init.lock");
    let _file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| format!("Não foi possível obter .fp-init.lock: {e}"))?;
    let _lock = Lock(lock_path);
    let preflight = inspect(root);
    if !preflight.problems.is_empty() {
        return Err(preflight.problems.join("\n").into());
    }
    for dir in DIRECTORIES {
        let path = root.join(dir);
        if !path.exists() {
            fs::create_dir(&path)?;
            created.push(path);
        }
    }
    for (file, contents) in FILES {
        let path = root.join(file);
        // symlink_metadata detecta inclusive links quebrados; nunca os substitui.
        match fs::symlink_metadata(&path) {
            Ok(_) => continue,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let mut temp = tempfile::NamedTempFile::new_in(path.parent().ok_or("Caminho inválido")?)?;
        temp.write_all(contents.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(&path)?;
        created.push(path);
    }
    let after = inspect(root);
    if !after.problems.is_empty() {
        return Err(after.problems.join("\n").into());
    }
    Ok(created)
}

#[derive(Debug)]
pub struct Tool {
    pub label: &'static str,
    pub required: bool,
    pub executable: Option<PathBuf>,
    pub outside_path: Option<PathBuf>,
}

pub fn tools(path: &OsStr, home: Option<&OsStr>) -> Vec<Tool> {
    let specs: &[(&str, &[&str], bool)] = &[
        ("Just", &["just"], true),
        ("Python 3.11+", &["python3"], true),
        ("Python / uv", &["uv"], false),
        ("Vite+", &["vp"], false),
        ("Rust / Cargo", &["cargo"], false),
        ("PlatformIO", &["pio", "platformio"], false),
        ("Zed", &["zed"], false),
        ("Docker (db)", &["docker"], false),
        ("SQLite (db)", &["sqlite3"], false),
    ];
    let mut candidates = vec![PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/bin")];
    if let Some(home) = home {
        candidates.insert(0, PathBuf::from(home).join(".local/bin"));
        candidates.insert(1, PathBuf::from(home).join(".cargo/bin"));
        candidates.insert(2, PathBuf::from(home).join(".local/share/vite-plus/bin"));
        candidates.insert(3, PathBuf::from(home).join(".platformio/penv/bin"));
    }
    let candidate_path = std::env::join_paths(candidates).unwrap_or_default();
    specs
        .iter()
        .map(|(label, names, required)| {
            let executable = names.iter().find_map(|name| processes::find_in(name, path));
            let outside_path = if executable.is_none() {
                names
                    .iter()
                    .find_map(|name| processes::find_in(name, &candidate_path))
            } else {
                None
            };
            Tool {
                label,
                required: *required,
                executable,
                outside_path,
            }
        })
        .collect()
}

pub fn python_supported(python: &Path) -> bool {
    let Ok(output) = Command::new(python).arg("--version").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let version = String::from_utf8_lossy(&output.stdout);
    let mut parts = version
        .trim()
        .strip_prefix("Python ")
        .unwrap_or("")
        .split('.');
    matches!((parts.next().and_then(|s| s.parse::<u32>().ok()), parts.next().and_then(|s| s.parse::<u32>().ok())), (Some(3), Some(minor)) if minor >= 11)
}
