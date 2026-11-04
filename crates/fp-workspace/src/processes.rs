use crate::Result;
#[cfg(unix)]
use std::os::unix::{fs::PermissionsExt, process::ExitStatusExt};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

pub fn find_in(tool: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .flat_map(|p| {
            executable_names(tool)
                .into_iter()
                .map(move |name| p.join(name))
        })
        .find(|p| executable(p))
        .and_then(|p| {
            if p.is_absolute() {
                Some(p)
            } else {
                Some(env::current_dir().ok()?.join(p))
            }
        })
}
#[cfg(unix)]
fn executable_names(tool: &str) -> Vec<String> {
    vec![tool.into()]
}
#[cfg(windows)]
fn executable_names(tool: &str) -> Vec<String> {
    // Only formats Command can execute. PATHEXT associations such as .py/.vbs
    // must not turn into implicit shell/file-association execution.
    if Path::new(tool).extension().is_some() {
        return vec![tool.into()];
    }
    let extensions = env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.COM;.BAT;.CMD".into());
    let mut names = vec![format!("{tool}.exe")];
    for extension in extensions.split(';') {
        let extension = extension.to_ascii_lowercase();
        if [".exe", ".com", ".bat", ".cmd"].contains(&extension.as_str()) {
            let name = format!("{tool}{extension}");
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}
fn executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| {
        #[cfg(unix)]
        {
            m.is_file() && m.permissions().mode() & 0o111 != 0
        }
        #[cfg(windows)]
        {
            m.is_file()
                && path.extension().is_some_and(|e| {
                    ["exe", "com", "cmd", "bat"]
                        .iter()
                        .any(|s| e.as_encoded_bytes().eq_ignore_ascii_case(s.as_bytes()))
                })
        }
    })
}
pub fn find(tool: &str) -> Result<PathBuf> {
    find_in(tool, &env::var_os("PATH").unwrap_or_default()).ok_or_else(|| {
        format!("Ferramenta '{tool}' ausente no PATH; instale-a para continuar").into()
    })
}
pub fn code(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
    }
    #[cfg(windows)]
    {
        status
            .code()
            .expect("Windows sempre fornece o código do processo")
    }
}
pub fn run_just(
    root: &Path,
    just: &Path,
    phase: u32,
    chapter: u32,
    args: &[String],
) -> Result<i32> {
    let mut forwarded = vec!["run".into(), phase.to_string(), chapter.to_string()];
    forwarded.extend_from_slice(args);
    validate_batch(just, forwarded.iter().map(String::as_str))?;
    Ok(code(
        Command::new(just)
            .current_dir(root)
            .args(&forwarded)
            .status()?,
    ))
}
pub fn initialize(tool: &Path, args: &[String], cwd: &Path) -> Result<()> {
    validate_batch(tool, args.iter().map(String::as_str))?;
    let mut command = Command::new(tool);
    command.args(args).current_dir(cwd);
    let mut retries = 0;
    let status = loop {
        match command.status() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy && retries < 5 => {
                retries += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            result => break result?,
        }
    };
    if !status.success() {
        return Err(format!("{} falhou (código {})", tool.display(), code(status)).into());
    }
    Ok(())
}
pub fn open_zed(path: &Path) -> Result<()> {
    let zed = find("zed")?;
    validate_batch(
        &zed,
        [path.to_str().ok_or("Caminho Zed não UTF-8")?].into_iter(),
    )?;
    Command::new(zed).arg(path).spawn()?;
    Ok(())
}

// Command usa o suporte batch do Rust, sem construir uma string CMD. Rejeitamos
// entradas que a expansão do próprio script poderia reinterpretar.
fn validate_batch<'a>(tool: &Path, args: impl Iterator<Item = &'a str>) -> Result<()> {
    #[cfg(windows)]
    if tool.extension().is_some_and(|e| {
        ["bat", "cmd"]
            .iter()
            .any(|s| e.as_encoded_bytes().eq_ignore_ascii_case(s.as_bytes()))
    }) {
        let mut args = args;
        let unsafe_text = |s: &str| s.chars().any(|c| "\r\n\"%!?^&|<>()".contains(c));
        if unsafe_text(&tool.to_string_lossy()) || args.any(unsafe_text) {
            return Err("Argumento incompatível com execução segura de .cmd/.bat; utilize o executável .exe ou seu interpretador explicitamente".into());
        }
    }
    #[cfg(unix)]
    let _ = (tool, args);
    Ok(())
}
