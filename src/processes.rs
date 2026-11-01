use crate::Result;
use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

pub fn find_in(tool: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .map(|p| p.join(tool))
        .find(|p| fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
}
pub fn find(tool: &str) -> Result<PathBuf> {
    find_in(tool, &env::var_os("PATH").unwrap_or_default()).ok_or_else(|| {
        format!("Ferramenta '{tool}' ausente no PATH; instale-a para continuar").into()
    })
}
pub fn code(status: ExitStatus) -> i32 {
    status
        .code()
        .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
}
pub fn run_just(
    root: &Path,
    just: &Path,
    phase: u32,
    chapter: u32,
    args: &[String],
) -> Result<i32> {
    Ok(code(
        Command::new(just)
            .current_dir(root)
            .arg("run")
            .arg(phase.to_string())
            .arg(chapter.to_string())
            .args(args)
            .status()?,
    ))
}
pub fn initialize(tool: &Path, args: &[String], cwd: &Path) -> Result<()> {
    let status = Command::new(tool).args(args).current_dir(cwd).status()?;
    if !status.success() {
        return Err(format!("{} falhou (código {})", tool.display(), code(status)).into());
    }
    Ok(())
}
pub fn open_zed(path: &Path) -> Result<()> {
    Command::new(find("zed")?).arg(path).spawn()?;
    Ok(())
}

/// Permite apontar uma instalação local sem depender de alias do shell.
pub fn find_db() -> Result<PathBuf> {
    if let Some(path) = env::var_os("FP_DB_BIN") {
        let path = PathBuf::from(path);
        let metadata = fs::metadata(&path)?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err("FP_DB_BIN deve apontar para um executável db".into());
        }
        return Ok(path);
    }
    find("db")
}

pub fn run_db(db: &Path, args: &[String]) -> Result<i32> {
    Ok(code(
        Command::new(db)
            .args(args)
            .env("DB_EXECUTION_PROFILE", "user")
            .status()?,
    ))
}
