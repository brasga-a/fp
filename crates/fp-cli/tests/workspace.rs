use clap::Parser;
use fiap_projects::{
    cli::{Cli, Commands},
    root, workspace,
};
#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut result = BTreeMap::new();
    if !root.exists() {
        return result;
    }
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        result.insert(
            entry.path(),
            if entry.file_type().unwrap().is_file() {
                fs::read(entry.path()).unwrap()
            } else {
                Vec::new()
            },
        );
        if entry.file_type().unwrap().is_dir() {
            result.extend(snapshot(&entry.path()));
        }
    }
    result
}
#[cfg(unix)]
fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_fp"));
    c.env_remove("FP_ROOT");
    c
}

#[test]
fn parses_workspace_init_and_check_separately_from_create() {
    assert!(matches!(
        Cli::try_parse_from(["fp", "init"]).unwrap().command,
        Commands::Init { check: false, .. }
    ));
    assert!(matches!(
        Cli::try_parse_from(["fp", "init", "--check"])
            .unwrap()
            .command,
        Commands::Init { check: true, .. }
    ));
    assert!(matches!(
        Cli::try_parse_from(["fp", "create"]).unwrap().command,
        Commands::Create
    ));
}
#[test]
fn root_location_supports_nonexistent_home_root_and_precedence() {
    let dir = tempdir().unwrap();
    let expected = dir.path().join("fiap");
    assert_eq!(
        root::locate(None, None, Some(dir.path().as_os_str())).unwrap(),
        expected
    );
    assert!(!expected.exists());
    let env = dir.path().join("env");
    assert_eq!(
        root::locate(Some(&expected), Some(env.as_os_str()), None).unwrap(),
        expected
    );
    assert_eq!(
        root::locate(None, Some(env.as_os_str()), None).unwrap(),
        env
    );
    assert_eq!(
        root::locate(
            Some(Path::new("~/a/../fiap")),
            None,
            Some(dir.path().as_os_str())
        )
        .unwrap(),
        expected
    );
    assert!(root::resolve(Some(&expected), None, None).is_err());
}
#[test]
#[cfg(unix)]
fn root_location_respects_existing_symlink_before_parent_component() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("real/sub")).unwrap();
    symlink(dir.path().join("real/sub"), dir.path().join("alias")).unwrap();
    assert_eq!(
        root::locate(Some(&dir.path().join("alias/../fiap")), None, None).unwrap(),
        dir.path().join("real/fiap")
    );
}
#[test]
fn initializes_complete_workspace_without_creating_phases_or_environments() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("new/fiap");
    let created = workspace::initialize(&root).unwrap();
    assert!(!created.is_empty());
    for (file, contents) in workspace::FILES {
        assert_eq!(fs::read_to_string(root.join(file)).unwrap(), *contents);
    }
    for path in [
        "fase-01",
        ".venv",
        ".git",
        "uv.lock",
        "Cargo.toml",
        "package.json",
        ".fp-init.lock",
    ] {
        assert!(!root.join(path).exists(), "{path}");
    }
    assert!(workspace::inspect(&root).missing.is_empty());
    assert!(workspace::inspect(&root).problems.is_empty());
    assert_eq!(root::resolve(Some(&root), None, None).unwrap(), root);
}
#[test]
fn rerun_preserves_every_existing_byte_and_creates_nothing() {
    let dir = tempdir().unwrap();
    workspace::initialize(dir.path()).unwrap();
    fs::write(dir.path().join("AGENTS.md"), "# Custom instructions\n").unwrap();
    fs::write(dir.path().join("README.md"), "Student's notes\n").unwrap();
    fs::write(dir.path().join("Justfile"), "# Custom orchestration\n").unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname='custom'\ndependencies=['numpy']\n",
    )
    .unwrap();
    let before = snapshot(dir.path());
    assert!(workspace::initialize(dir.path()).unwrap().is_empty());
    assert_eq!(snapshot(dir.path()), before);
}
#[test]
fn repairs_only_missing_support_file_and_preserves_registered_project() {
    let dir = tempdir().unwrap();
    workspace::initialize(dir.path()).unwrap();
    fs::create_dir_all(dir.path().join("fase-03/cap-02")).unwrap();
    fs::write(
        dir.path().join("fase-03/cap-02/main.py"),
        "print('original')",
    )
    .unwrap();
    let registry = "# keep\n[chapters.'3.2']\npath='fase-03/cap-02'\n[chapters.'3.2'.commands.main]\nargv=['python3','main.py']\n";
    fs::write(dir.path().join("chapters.toml"), registry).unwrap();
    fs::remove_file(dir.path().join("docs/README.md")).unwrap();
    let created = workspace::initialize(dir.path()).unwrap();
    assert_eq!(created, [dir.path().join("docs/README.md")]);
    assert_eq!(
        fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
        registry
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("fase-03/cap-02/main.py")).unwrap(),
        "print('original')"
    );
}
#[test]
fn path_conflicts_block_all_writes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("scripts"), "not a directory").unwrap();
    let before = snapshot(dir.path());
    assert!(workspace::initialize(dir.path()).is_err());
    assert_eq!(snapshot(dir.path()), before);
}
#[test]
#[cfg(unix)]
fn symlinked_managed_paths_are_preserved_without_outside_writes() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    symlink(outside.path(), dir.path().join("scripts")).unwrap();
    assert!(workspace::initialize(dir.path()).is_err());
    assert_eq!(
        fs::read_link(dir.path().join("scripts")).unwrap(),
        outside.path()
    );
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    assert!(!dir.path().join("AGENTS.md").exists());
}
#[test]
fn invalid_existing_registry_and_python_config_are_reported_without_repair() {
    for (file, contents) in [
        ("chapters.toml", "[bad"),
        ("pyproject.toml", "[bad"),
        ("chapters.toml", "[chapters.'1.2']\npath='missing'\n"),
    ] {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(file), contents).unwrap();
        let before = snapshot(dir.path());
        assert!(!workspace::inspect(dir.path()).problems.is_empty());
        assert!(workspace::initialize(dir.path()).is_err());
        assert_eq!(snapshot(dir.path()), before);
    }
}
#[test]
fn check_is_read_only_for_missing_and_complete_workspaces() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fiap");
    let output = bin()
        .arg("--root")
        .arg(&root)
        .args(["init", "--academic-only", "--check"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("AGENTS.md"));
    assert!(!root.exists());
    workspace::initialize(&root).unwrap();
    let before = snapshot(&root);
    let output = bin()
        .arg("--root")
        .arg(&root)
        .args(["init", "--academic-only", "--check"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(snapshot(&root), before);
}
#[test]
fn cli_uses_default_home_and_env_root_without_requiring_existing_structure() {
    for use_env in [false, true] {
        let dir = tempdir().unwrap();
        let root = dir
            .path()
            .join(if use_env { "custom-root" } else { "fiap" });
        let mut cmd = bin();
        cmd.env("HOME", dir.path()).env("USERPROFILE", dir.path());
        if use_env {
            cmd.env("FP_ROOT", &root);
        }
        let output = cmd.args(["init", "--academic-only"]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(root.join("AGENTS.md").is_file());
    }
}
#[test]
fn missing_required_tools_report_failure_after_creating_structure() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("fiap");
    let empty = dir.path().join("bin");
    fs::create_dir(&empty).unwrap();
    let output = bin()
        .env("PATH", &empty)
        .env("HOME", dir.path())
        .arg("--root")
        .arg(&root)
        .args(["init", "--academic-only"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(root.join("Justfile").is_file());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("[FALTA] Just"));
    assert!(text.contains("[FALTA] Python 3.14+"));
    assert!(text.contains("Instale a ferramenta"));
}
#[test]
#[cfg(unix)]
fn finds_tools_outside_path_and_accepts_platformio_alias() {
    let dir = tempdir().unwrap();
    let bin = dir.path().join(".local/bin");
    fs::create_dir_all(&bin).unwrap();
    let just = script(&bin, "just", "exit 0");
    let vite = dir.path().join(".local/share/vite-plus/bin");
    fs::create_dir_all(&vite).unwrap();
    let vp = script(&vite, "vp", "exit 0");
    script(&bin, "platformio", "exit 0");
    let empty = dir.path().join("empty");
    fs::create_dir(&empty).unwrap();
    let tools = workspace::tools(empty.as_os_str(), Some(dir.path().as_os_str()));
    let found = tools.iter().find(|t| t.label == "Just").unwrap();
    assert_eq!(found.outside_path.as_deref(), Some(just.as_path()));
    assert!(found.executable.is_none());
    assert_eq!(
        tools
            .iter()
            .find(|t| t.label == "Vite+")
            .unwrap()
            .outside_path
            .as_deref(),
        Some(vp.as_path())
    );
    let tools = workspace::tools(bin.as_os_str(), None);
    assert!(
        tools
            .iter()
            .find(|t| t.label == "PlatformIO")
            .unwrap()
            .executable
            .is_some()
    );
}
#[test]
#[cfg(unix)]
fn validates_required_python_version() {
    let dir = tempdir().unwrap();
    for (version, supported) in [
        ("Python 3.10.9", false),
        ("Python 3.11.0", false),
        ("Python 3.12.9", false),
        ("Python 3.13.9", false),
        ("Python 3.14.0", true),
        ("Python 3.15.0", true),
        ("invalid", false),
    ] {
        let python = script(dir.path(), "python", &format!("echo '{version}'"));
        assert_eq!(workspace::python_supported(&python), supported);
    }
}
#[test]
fn shared_lock_prevents_bootstrap_writes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join(".fp-init.lock"), "busy").unwrap();
    let before = snapshot(dir.path());
    assert!(workspace::initialize(dir.path()).is_err());
    assert_eq!(snapshot(dir.path()), before);
}
#[test]
fn generated_dispatcher_runs_without_fp_installed() {
    let dir = tempdir().unwrap();
    workspace::initialize(dir.path()).unwrap();
    let just = fiap_projects::processes::find("just").unwrap();
    let output = Command::new(just)
        .current_dir(dir.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Nenhum capítulo"));
    let python = workspace::tools(&std::env::var_os("PATH").unwrap_or_default(), None)
        .into_iter()
        .find(|t| t.label == "Python 3.14+")
        .unwrap()
        .executable
        .unwrap();
    let status = Command::new(python)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .current_dir(dir.path())
        .args(["-m", "unittest", "discover", "-s", "tests", "-q"])
        .status()
        .unwrap();
    assert!(status.success());
}

fn tempdir() -> std::io::Result<TempDir> {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
}
