use clap::Parser;
#[cfg(unix)]
use fiap_projects::processes;
use fiap_projects::{
    cli::{Cli, Commands},
    create::{Plan, Technology, create},
    discovery, registry, root, tui,
};
#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn repo() -> TempDir {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Justfile"),
        "run phase chapter *args:\n    @exit 0\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("chapters.toml"),
        "# preservar comentário\n[chapters]\n",
    )
    .unwrap();
    dir
}
#[cfg(unix)]
fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
fn plan(root: &Path, technology: Technology, tool: Option<PathBuf>) -> Plan {
    fs::create_dir_all(root.join("fase-01")).unwrap();
    Plan {
        target: root.join("fase-01/cap-01"),
        chapter: Some((1, 1)),
        technology,
        tool,
        board: None,
        framework: None,
    }
}
fn binary() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_fp"));
    cmd.env_remove("FP_ROOT");
    cmd
}

#[test]
fn clap_commands_and_trailing_arguments() {
    for args in [
        vec!["fp", "create"],
        vec!["fp", "list"],
        vec!["fp", "--root", "/tmp", "run", "1", "2"],
    ] {
        assert!(Cli::try_parse_from(args).is_ok());
    }
    let cli = Cli::try_parse_from(["fp", "run", "1", "2", "--task", "dev", "--", "a b"]).unwrap();
    match cli.command {
        Commands::Run {
            phase,
            chapter,
            args,
        } => {
            assert_eq!((phase, chapter), (1, 2));
            assert_eq!(args, ["--task", "dev", "--", "a b"]);
        }
        _ => panic!(),
    }
    assert!(Cli::try_parse_from(["fp", "run", "0", "2"]).is_err());
    assert!(Cli::try_parse_from(["fp", "run", "x", "2"]).is_err());
}
#[test]
fn root_precedence_home_and_normalization() {
    let a = repo();
    let b = repo();
    assert_eq!(
        root::resolve(Some(a.path()), Some(b.path().as_os_str()), None).unwrap(),
        a.path()
    );
    assert_eq!(
        root::resolve(None, Some(b.path().as_os_str()), None).unwrap(),
        b.path()
    );
    fs::create_dir(a.path().join("fiap")).unwrap();
    fs::write(a.path().join("fiap/Justfile"), "").unwrap();
    fs::write(a.path().join("fiap/chapters.toml"), "[chapters]").unwrap();
    assert_eq!(
        root::resolve(None, None, Some(a.path().as_os_str())).unwrap(),
        a.path().join("fiap")
    );
    assert_eq!(
        root::resolve(
            Some(Path::new("~/fiap/../fiap")),
            None,
            Some(a.path().as_os_str())
        )
        .unwrap(),
        a.path().join("fiap")
    );
    assert!(root::resolve(None, None, None).is_err());
    assert!(root::resolve(Some(Path::new("/fp-nonexistent-root")), None, None).is_err());
    let empty = tempdir().unwrap();
    assert!(root::resolve(Some(empty.path()), None, None).is_err());
}
#[test]
fn cli_environment_and_explicit_override() {
    let a = repo();
    let b = tempdir().unwrap();
    assert!(
        binary()
            .env("FP_ROOT", a.path())
            .arg("list")
            .status()
            .unwrap()
            .success()
    );
    assert!(
        binary()
            .env("FP_ROOT", b.path())
            .arg("--root")
            .arg(a.path())
            .arg("list")
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn names_are_generated_from_kind_and_number_only() {
    assert_eq!(discovery::dirname("fase-", 3, &[]), "fase-03");
    assert_eq!(discovery::dirname("cap-", 2, &[]), "cap-02");
    assert_eq!(discovery::dirname("cap-", 123, &[]), "cap-123");
    assert!(matches!(
        Cli::try_parse_from(["fp", "init"]).unwrap().command,
        Commands::Init { check: false, .. }
    ));
    assert!(Cli::try_parse_from(["fp", "create", "meu-nome"]).is_err());
}
#[test]
fn numbering_and_existing_conventions() {
    let dir = tempdir().unwrap();
    for name in [
        "fase-01-intro",
        "fase-03-ia",
        "fase-0",
        "fase-7oops",
        "docs",
    ] {
        fs::create_dir(dir.path().join(name)).unwrap();
    }
    let phases = discovery::discover(dir.path(), "fase-").unwrap();
    assert_eq!(phases.iter().map(|p| p.number).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(discovery::next(&phases).unwrap(), 4);
    assert_eq!(discovery::dirname("fase-", 4, &phases), "fase-04");
    fs::create_dir(phases[0].path.join("cap-009")).unwrap();
    let chapters = discovery::discover(&phases[0].path, "cap-").unwrap();
    assert_eq!(discovery::next(&chapters).unwrap(), 10);
    assert_eq!(discovery::dirname("cap-", 10, &chapters), "cap-010");
    assert_eq!(discovery::dirname("cap-", 1, &[]), "cap-01");
    #[cfg(unix)]
    symlink(&phases[0].path, dir.path().join("fase-08")).unwrap();
    assert_eq!(discovery::discover(dir.path(), "fase-").unwrap().len(), 2);
    fs::create_dir(dir.path().join("fase-1-duplicate")).unwrap();
    assert!(discovery::discover(dir.path(), "fase-").is_err());
}
#[test]
fn cancellation_creates_nothing() {
    let dir = repo();
    let plan = plan(dir.path(), Technology::Rust, None);
    let before = fs::read_dir(dir.path()).unwrap().count();
    assert!(create(dir.path(), &plan, false).unwrap().is_none());
    assert!(!plan.target.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), before);
    assert_eq!(
        fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
        "# preservar comentário\n[chapters]\n"
    );
}
#[test]
fn conflict_preserves_existing_directory() {
    let dir = repo();
    let plan = plan(dir.path(), Technology::Folder, None);
    fs::create_dir(&plan.target).unwrap();
    fs::write(plan.target.join("original"), "keep").unwrap();
    assert!(create(dir.path(), &plan, true).is_err());
    assert_eq!(
        fs::read_to_string(plan.target.join("original")).unwrap(),
        "keep"
    );
}
#[test]
#[cfg(unix)]
fn failed_external_initializer_rolls_back() {
    let dir = repo();
    let tools = tempdir().unwrap();
    let tool = script(
        tools.path(),
        "cargo",
        "mkdir src\necho partial > src/main.rs\nexit 37",
    );
    let plan = plan(dir.path(), Technology::Rust, Some(tool));
    let error = create(dir.path(), &plan, true).unwrap_err().to_string();
    assert!(error.contains("37"));
    assert!(!plan.target.exists());
    assert!(!dir.path().join(".fp-init.lock").exists());
    assert!(
        registry::load(dir.path()).unwrap()["chapters"]
            .as_table()
            .unwrap()
            .is_empty()
    );
}
#[test]
#[cfg(unix)]
fn successful_native_initializer_registers_real_entrypoint() {
    let dir = repo();
    let tools = tempdir().unwrap();
    let tool = script(
        tools.path(),
        "cargo",
        "test \"$1\" = init || exit 9\nmkdir src\necho 'fn main() {}' > src/main.rs\necho '[package]' > Cargo.toml",
    );
    let plan = plan(dir.path(), Technology::Rust, Some(tool));
    let command = create(dir.path(), &plan, true).unwrap().unwrap();
    assert_eq!(command, ["cargo", "run", "--"]);
    let doc = registry::load(dir.path()).unwrap();
    assert!(registry::executable(&doc, "1.1"));
    assert_eq!(doc["chapters"]["1.1"]["name"].as_str(), Some("cap-01"));
    assert!(doc.to_string().contains("# preservar comentário"));
    assert_eq!(
        doc["chapters"]["1.1"]["path"].as_str(),
        Some("fase-01/cap-01")
    );
    assert!(tui::list(dir.path()).unwrap().contains("Rust [executável]"));
}
#[test]
fn registry_keeps_existing_entries_and_rejects_duplicates() {
    let dir = repo();
    fs::create_dir(dir.path().join("existing")).unwrap();
    let mut doc = registry::load(dir.path()).unwrap();
    registry::add(
        &mut doc,
        "2.3",
        "existing",
        "Original",
        Some(&["echo".into(), "ok".into()]),
    )
    .unwrap();
    let original = doc.to_string();
    registry::add(&mut doc, "2.4", "existing", "Outro", None).unwrap();
    assert_eq!(doc["chapters"]["2.3"]["name"].as_str(), Some("Original"));
    assert!(registry::add(&mut doc, "2.3", "existing", "Substituir", None).is_err());
    fs::write(dir.path().join("chapters.toml"), &original).unwrap();
    assert!(tui::list(dir.path()).unwrap().contains("Registro 2.3"));
    assert!(registry::save(dir.path(), "changed", &doc).is_err());
    assert_eq!(
        fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
        original
    );
}
#[test]
fn phase_and_folder_and_sql_do_not_invent_commands() {
    for tech in [Technology::Folder, Technology::Sql] {
        let dir = repo();
        let plan = plan(dir.path(), tech, None);
        assert!(create(dir.path(), &plan, true).unwrap().is_none());
        assert!(!registry::executable(
            &registry::load(dir.path()).unwrap(),
            "1.1"
        ));
        assert_eq!(
            fs::read_dir(&plan.target).unwrap().count(),
            usize::from(tech == Technology::Sql)
        );
    }
    let dir = repo();
    let plan = Plan {
        target: dir.path().join("fase-01"),
        chapter: None,
        technology: Technology::Folder,
        tool: None,
        board: None,
        framework: None,
    };
    create(dir.path(), &plan, true).unwrap();
    assert_eq!(fs::read_dir(plan.target).unwrap().count(), 0);
    assert!(
        registry::load(dir.path()).unwrap()["chapters"]
            .as_table()
            .unwrap()
            .is_empty()
    );
}
#[test]
#[cfg(unix)]
fn new_python_workspace_passes_minimum_version_to_uv() {
    let dir = repo();
    fiap_projects::workspace::initialize(dir.path()).unwrap();
    let tools = tempdir().unwrap();
    let uv = script(
        tools.path(),
        "uv",
        "printf '%s\\n' \"$@\" > initializer-args\necho '[project]' > pyproject.toml\necho 'print(42)' > main.py",
    );
    let plan = plan(dir.path(), Technology::Python, Some(uv));
    create(dir.path(), &plan, true).unwrap();
    let args = fs::read_to_string(plan.target.join("initializer-args")).unwrap();
    assert!(args.contains("--python\n>=3.14\n"));
    let root = fs::read_to_string(dir.path().join("pyproject.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(root["project"]["requires-python"].as_str(), Some(">=3.14"));
}

#[test]
#[cfg(unix)]
fn python_app_respects_version_and_registers_main() {
    let dir = repo();
    let tools = tempdir().unwrap();
    fs::write(
        dir.path().join("pyproject.toml"),
        "# preservar configuração\n[project]\nname = 'fiap-academic'\nrequires-python = '>=3.11'\ndependencies = ['numpy>=2']\n[tool.uv]\npackage = false\n[tool.custom]\nenabled = true\n",
    )
    .unwrap();
    let tool = script(
        tools.path(),
        "uv",
        "printf '%s\\n' \"$@\" > initializer-args\necho '[project]' > pyproject.toml\necho 'print(42)' > main.py",
    );
    let plan = plan(dir.path(), Technology::Python, Some(tool));
    let command = create(dir.path(), &plan, true).unwrap().unwrap();
    assert_eq!(command, ["uv", "run", "--all-packages", "main.py"]);
    assert!(plan.target.join("main.py").is_file());
    let doc = registry::load(dir.path()).unwrap();
    assert!(registry::executable(&doc, "1.1"));
    assert_eq!(doc["chapters"]["1.1"]["name"].as_str(), Some("cap-01"));
    assert_eq!(doc["chapters"]["1.1"]["default"].as_str(), Some("main"));
    assert!(
        tui::list(dir.path())
            .unwrap()
            .contains("Python [executável]")
    );
    let args = fs::read_to_string(plan.target.join("initializer-args")).unwrap();
    for expected in [
        "--app",
        "--no-package",
        "--no-workspace",
        "--no-python-downloads",
        "--name\nfase-01-cap-01\n",
        ">=3.11",
    ] {
        assert!(args.contains(expected));
    }
    assert!(!plan.target.join(".venv").exists());
    let root = fs::read_to_string(dir.path().join("pyproject.toml")).unwrap();
    assert!(root.contains("# preservar configuração"));
    let root = root.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(
        root["tool"]["uv"]["workspace"]["members"][0].as_str(),
        Some("fase-01/cap-01")
    );
    assert_eq!(
        root["project"]["dependencies"][0].as_str(),
        Some("numpy>=2")
    );
    assert_eq!(root["tool"]["custom"]["enabled"].as_bool(), Some(true));
    assert_eq!(root["tool"]["pyright"]["venvPath"].as_str(), Some("."));
    assert_eq!(root["tool"]["pyright"]["venv"].as_str(), Some(".venv"));
    let member = fs::read_to_string(plan.target.join("pyproject.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(member["project"]["name"].as_str(), Some("fase-01-cap-01"));
    assert_eq!(member["tool"]["uv"]["package"].as_bool(), Some(false));
    assert_eq!(
        member["tool"]["pyright"]["venvPath"].as_str(),
        Some("../..")
    );
    assert_eq!(member["tool"]["pyright"]["venv"].as_str(), Some(".venv"));
}

#[test]
#[cfg(unix)]
fn python_chapters_in_different_phases_join_one_workspace_with_unique_names() {
    let dir = repo();
    fs::write(dir.path().join("pyproject.toml"), "[project]\nname='fiap-academic'\n[tool.pyright]\nvenvPath='custom'\nvenv='environment'\ntypeCheckingMode='strict'\n").unwrap();
    let tools = tempdir().unwrap();
    let uv = script(
        tools.path(),
        "uv",
        "echo '[project]' > pyproject.toml\necho 'print(42)' > main.py",
    );
    let first = plan(dir.path(), Technology::Python, Some(uv.clone()));
    create(dir.path(), &first, true).unwrap();
    fs::create_dir(dir.path().join("fase-02")).unwrap();
    let second = Plan {
        target: dir.path().join("fase-02/cap-01"),
        chapter: Some((2, 1)),
        tool: Some(uv),
        ..first
    };
    create(dir.path(), &second, true).unwrap();
    let root = fs::read_to_string(dir.path().join("pyproject.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let members = root["tool"]["uv"]["workspace"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(
        members
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["fase-01/cap-01", "fase-02/cap-01"]
    );
    let member = fs::read_to_string(second.target.join("pyproject.toml"))
        .unwrap()
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    assert_eq!(member["project"]["name"].as_str(), Some("fase-02-cap-01"));
    assert_eq!(root["tool"]["pyright"]["venvPath"].as_str(), Some("custom"));
    assert_eq!(
        root["tool"]["pyright"]["venv"].as_str(),
        Some("environment")
    );
    assert_eq!(
        root["tool"]["pyright"]["typeCheckingMode"].as_str(),
        Some("strict")
    );
    assert!(!dir.path().join(".venv").exists());
    assert!(!dir.path().join("uv.lock").exists());
}

#[test]
#[cfg(unix)]
fn python_creation_rolls_back_workspace_when_registry_changes() {
    for existing in [false, true] {
        let dir = repo();
        let config = dir.path().join("pyproject.toml");
        let before = "# manter\n[project]\nname='fiap-academic'\ndependencies=[]\n";
        if existing {
            fs::write(&config, before).unwrap();
        }
        let tools = tempdir().unwrap();
        let uv = script(
            tools.path(),
            "uv",
            &format!(
                "echo '[project]' > pyproject.toml\necho 'print(42)' > main.py\necho '# edição concorrente' >> '{}'",
                dir.path().join("chapters.toml").display()
            ),
        );
        let plan = plan(dir.path(), Technology::Python, Some(uv));
        assert!(
            create(dir.path(), &plan, true)
                .unwrap_err()
                .to_string()
                .contains("Registro alterado")
        );
        assert!(!plan.target.exists());
        if existing {
            assert_eq!(fs::read_to_string(&config).unwrap(), before);
        } else {
            assert!(!config.exists());
        }
        assert!(
            fs::read_to_string(dir.path().join("chapters.toml"))
                .unwrap()
                .contains("# edição concorrente")
        );
    }
}

#[test]
#[cfg(unix)]
fn invalid_or_symlinked_python_workspace_is_preserved_before_initializing() {
    for contents in [
        "[tool.uv.workspace]\nmembers = 'invalid'\n",
        "[tool.uv.workspace]\nmembers = [42]\n",
        "[tool.uv.workspace]\nmembers = []\nexclude = ['fase-*']\n",
        "[tool]\nuv = false\n",
    ] {
        let dir = repo();
        fs::write(dir.path().join("pyproject.toml"), contents).unwrap();
        let tools = tempdir().unwrap();
        let uv = script(tools.path(), "uv", "exit 99");
        let plan = plan(dir.path(), Technology::Python, Some(uv));
        assert!(create(dir.path(), &plan, true).is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join("pyproject.toml")).unwrap(),
            contents
        );
        assert!(!plan.target.exists());
    }
    let dir = repo();
    let outside = tempdir().unwrap();
    let external = outside.path().join("pyproject.toml");
    fs::write(&external, "# preservar\n").unwrap();
    symlink(&external, dir.path().join("pyproject.toml")).unwrap();
    let plan = plan(dir.path(), Technology::Python, None);
    assert!(
        create(dir.path(), &plan, true)
            .unwrap_err()
            .to_string()
            .contains("sem links")
    );
    assert_eq!(fs::read_to_string(external).unwrap(), "# preservar\n");
}
#[test]
#[cfg(unix)]
fn web_only_registers_existing_dev_script() {
    for (manifest, runnable) in [
        (
            r#"{"scripts":{"dev":"vite"},"packageManager":"pnpm@10"}"#,
            true,
        ),
        (r#"{"scripts":{"build":"vite build"}}"#, false),
    ] {
        let dir = repo();
        let tools = tempdir().unwrap();
        let tool = script(
            tools.path(),
            "vp",
            &format!("printf '%s' '{manifest}' > package.json"),
        );
        let plan = plan(dir.path(), Technology::Web, Some(tool));
        assert_eq!(create(dir.path(), &plan, true).unwrap().is_some(), runnable);
        assert_eq!(
            fs::read_to_string(plan.target.join("package.json")).unwrap(),
            manifest
        );
    }
}
#[test]
#[cfg(unix)]
fn iot_requires_board_and_never_registers_upload() {
    let dir = repo();
    let tools = tempdir().unwrap();
    let tool = script(
        tools.path(),
        "pio",
        "printf '%s\\n' \"$@\" > args\necho '[env:board]' > platformio.ini",
    );
    let mut plan = plan(dir.path(), Technology::Iot, Some(tool));
    assert!(create(dir.path(), &plan, true).is_err());
    assert!(!plan.target.exists());
    plan.board = Some("esp32dev".into());
    plan.framework = Some("espidf".into());
    assert!(create(dir.path(), &plan, true).unwrap().is_none());
    assert!(
        fs::read_to_string(plan.target.join("args"))
            .unwrap()
            .contains("framework=espidf")
    );
}
#[test]
#[cfg(unix)]
fn absence_of_tools_and_zed() {
    let tools = tempdir().unwrap();
    assert!(processes::find_in("zed", tools.path().as_os_str()).is_none());
    script(tools.path(), "zed", "exit 0");
    assert!(processes::find_in("zed", tools.path().as_os_str()).is_some());
    let dir = repo();
    let plan = plan(dir.path(), Technology::Rust, None);
    assert!(create(dir.path(), &plan, true).is_err());
    assert!(!plan.target.exists());
    let output = binary()
        .env("PATH", tools.path())
        .arg("--root")
        .arg(dir.path())
        .args(["run", "1", "2"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("just"));
}
#[test]
#[cfg(unix)]
fn just_delegation_cwd_literal_args_and_exit_code() {
    let dir = repo();
    let tools = tempdir().unwrap();
    let just = script(
        tools.path(),
        "just",
        "pwd > cwd\nprintf '%s\\n' \"$@\" > args\nexit 37",
    );
    let args = vec![
        "--task".into(),
        "dev".into(),
        "--".into(),
        "a b".into(),
        "$(touch unexpected)".into(),
    ];
    assert_eq!(
        processes::run_just(dir.path(), &just, 2, 3, &args).unwrap(),
        37
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("cwd")).unwrap().trim(),
        dir.path().to_str().unwrap()
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("args")).unwrap(),
        "run\n2\n3\n--task\ndev\n--\na b\n$(touch unexpected)\n"
    );
    let result = binary()
        .current_dir("/tmp")
        .env("PATH", tools.path())
        .arg("--root")
        .arg(dir.path())
        .args(["run", "2", "3"])
        .status()
        .unwrap();
    assert_eq!(result.code(), Some(37));
    assert!(!dir.path().join("unexpected").exists());
}
#[test]
#[cfg(unix)]
fn signal_exit_status() {
    let tools = tempdir().unwrap();
    let tool = script(tools.path(), "signal", "kill -TERM $$");
    let status = Command::new(tool).status().unwrap();
    assert_eq!(processes::code(status), 143);
}
#[test]
#[cfg(unix)]
fn real_just_in_temporary_repository() {
    let just = processes::find("just").expect("Instale Just para o teste de integração");
    let dir = repo();
    fs::write(dir.path().join("Justfile"), "set positional-arguments\nrun phase chapter *args:\n    @test \"$1\" = 1 && test \"$2\" = 2\n    @exit 37\n").unwrap();
    assert_eq!(
        processes::run_just(dir.path(), &just, 1, 2, &[]).unwrap(),
        37
    );
}
#[test]
fn list_and_error_behaviour_through_binary() {
    let dir = repo();
    fs::create_dir_all(dir.path().join("fase-01/cap-02")).unwrap();
    fs::write(dir.path().join("fase-01/cap-02/platformio.ini"), "").unwrap();
    let out = binary()
        .arg("--root")
        .arg(dir.path())
        .arg("list")
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("Fase 1"));
    assert!(text.contains("Capítulo 2"));
    assert!(text.contains("ESP32"));
    assert!(text.contains("sem comando"));
    let out = binary()
        .arg("--root")
        .arg(dir.path())
        .arg("create")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("terminal interativo"));
    fs::write(dir.path().join("chapters.toml"), "[bad").unwrap();
    assert!(
        !binary()
            .arg("--root")
            .arg(dir.path())
            .arg("list")
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn registry_rejects_escape_and_invalid_commands() {
    let dir = repo();
    for text in [
        "[chapters.'1.2']\npath = '..'",
        "[chapters.'0.2']\npath = '.'",
        "[chapters.'1.2']\npath = '.'\n[chapters.'1.2'.commands.main]\nargv = []",
        "[chapters.'1.2']\npath = '.'\ndefault = 'missing'",
    ] {
        fs::write(dir.path().join("chapters.toml"), text).unwrap();
        assert!(registry::load(dir.path()).is_err());
    }
}
#[test]
#[cfg(unix)]
fn failed_registry_publication_removes_only_new_project() {
    let dir = repo();
    let tools = tempdir().unwrap();
    let marker = dir.path().join("chapters.toml");
    let tool = script(
        tools.path(),
        "cargo",
        &format!(
            "mkdir src\necho 'fn main() {{}}' > src/main.rs\necho '[package]' > Cargo.toml\necho '# externally changed' >> '{}'",
            marker.display()
        ),
    );
    let plan = plan(dir.path(), Technology::Rust, Some(tool));
    assert!(create(dir.path(), &plan, true).is_err());
    assert!(!plan.target.exists());
    assert!(
        fs::read_to_string(marker)
            .unwrap()
            .contains("externally changed")
    );
}
#[test]
fn lock_and_outside_targets_are_rejected() {
    let dir = repo();
    let plan = plan(dir.path(), Technology::Folder, None);
    fs::write(dir.path().join(".fp-init.lock"), "other process").unwrap();
    assert!(create(dir.path(), &plan, true).is_err());
    assert!(!plan.target.exists());
    assert_eq!(
        fs::read_to_string(dir.path().join(".fp-init.lock")).unwrap(),
        "other process"
    );
    let outside = tempdir().unwrap();
    let mut plan = plan;
    plan.target = outside.path().join("chapter");
    assert!(create(dir.path(), &plan, true).is_err());
}

#[test]
fn technology_uses_manifests_in_registered_cwd() {
    let dir = repo();
    fs::create_dir_all(dir.path().join("fase-02/cap-03/app")).unwrap();
    fs::write(
        dir.path().join("fase-02/cap-03/app/pyproject.toml"),
        "[project]",
    )
    .unwrap();
    fs::write(dir.path().join("chapters.toml"), "[chapters.'2.3']\npath = 'fase-02/cap-03'\n[chapters.'2.3'.commands.main]\nargv = ['uv', 'run', 'main.py']\ncwd = 'app'\n").unwrap();
    let output = tui::list(dir.path()).unwrap();
    assert!(output.contains("Python [executável]"));
}

#[test]
fn technology_uses_registered_interpreter_without_manifest() {
    let dir = repo();
    fs::create_dir_all(dir.path().join("fase-01/cap-01")).unwrap();
    fs::write(dir.path().join("chapters.toml"), "[chapters.'1.1']\npath = 'fase-01/cap-01'\n[chapters.'1.1'.commands.main]\nargv = ['python3', 'exercicio.py']\n").unwrap();
    assert!(
        tui::list(dir.path())
            .unwrap()
            .contains("Python [executável]")
    );
}

#[test]
#[cfg(unix)]
fn cli_preserves_initial_separator_for_project_task_flag() {
    let dir = repo();
    let tools = tempdir().unwrap();
    script(tools.path(), "just", "printf '%s\\n' \"$@\"");
    for (extra, expected) in [
        (vec!["--", "--task", "app"], "run\n1\n2\n--\n--task\napp\n"),
        (
            vec!["--task", "dev", "--", "--flag"],
            "run\n1\n2\n--task\ndev\n--\n--flag\n",
        ),
        (vec!["--"], "run\n1\n2\n--\n"),
        (vec![], "run\n1\n2\n"),
    ] {
        let output = binary()
            .env("PATH", tools.path())
            .arg("--root")
            .arg(dir.path())
            .args(["run", "1", "2"])
            .args(extra)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
#[cfg(unix)]
fn python_app_without_main_rolls_back_instead_of_registering_empty_chapter() {
    let dir = repo();
    let tools = tempdir().unwrap();
    let uv = script(tools.path(), "uv", "echo '[project]' > pyproject.toml");
    let plan = plan(dir.path(), Technology::Python, Some(uv));
    let before = fs::read_to_string(dir.path().join("chapters.toml")).unwrap();
    let error = create(dir.path(), &plan, true).unwrap_err().to_string();
    assert!(error.contains("não criou main.py"));
    assert!(!plan.target.exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
        before
    );
    assert!(!dir.path().join(".fp-init.lock").exists());
}

fn tempdir() -> std::io::Result<TempDir> {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize()?)
}
