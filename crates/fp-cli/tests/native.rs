use fiap_projects::{
    create::{self, Plan, Technology},
    discovery, processes, registry, root, workspace,
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

fn tool(dir: &Path, name: &str) -> PathBuf {
    static BUILT: OnceLock<tempfile::TempDir> = OnceLock::new();
    let built = BUILT.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/tool.rs");
        let status = Command::new("rustc")
            .arg(source)
            .arg("-o")
            .arg(
                dir.path()
                    .join(format!("tool{}", std::env::consts::EXE_SUFFIX)),
            )
            .status()
            .unwrap();
        assert!(status.success());
        dir
    });
    let path = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    fs::copy(
        built
            .path()
            .join(format!("tool{}", std::env::consts::EXE_SUFFIX)),
        &path,
    )
    .unwrap();
    path
}

#[test]
fn native_initializers_preserve_domain_commands_and_register_entries() {
    for (technology, name) in [
        (Technology::Python, "uv"),
        (Technology::Rust, "cargo"),
        (Technology::Web, "vp"),
        (Technology::Iot, "pio"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("acadêmico com espaços");
        workspace::initialize(&root).unwrap();
        fs::create_dir(root.join("fase-01")).unwrap();
        let plan = Plan {
            target: root.join("fase-01/cap-01"),
            chapter: Some((1, 1)),
            technology,
            tool: Some(tool(dir.path(), name)),
            board: Some("esp32dev".into()),
            framework: None,
        };
        create::create(&root, &plan, true).unwrap();
        let doc = registry::load(&root).unwrap();
        assert_eq!(
            doc["chapters"]["1.1"]["path"].as_str(),
            Some("fase-01/cap-01")
        );
        if technology == Technology::Python {
            let manifest = fs::read_to_string(root.join("pyproject.toml"))
                .unwrap()
                .parse::<toml_edit::DocumentMut>()
                .unwrap();
            assert_eq!(
                manifest["tool"]["uv"]["workspace"]["members"][0].as_str(),
                Some("fase-01/cap-01")
            );
        }
        assert_eq!(discovery::discover(&root, "fase-").unwrap().len(), 1);
        assert!(!root.join(".fp-init.lock").exists());
    }
}
#[test]
fn native_initializer_failures_and_registry_conflicts_roll_back() {
    for name in ["fail", "cargo"] {
        let dir = tempfile::tempdir().unwrap();
        workspace::initialize(dir.path()).unwrap();
        fs::create_dir(dir.path().join("fase-01")).unwrap();
        let executable = tool(dir.path(), name);
        if name == "cargo" {
            fs::write(
                executable.with_extension("registry"),
                dir.path().join("chapters.toml").to_str().unwrap(),
            )
            .unwrap();
        }
        let plan = Plan {
            target: dir.path().join("fase-01/cap-01"),
            chapter: Some((1, 1)),
            technology: Technology::Rust,
            tool: Some(executable),
            board: None,
            framework: None,
        };
        assert!(create::create(dir.path(), &plan, true).is_err());
        assert!(!plan.target.exists());
        assert!(!dir.path().join(".fp-init.lock").exists());
    }
}
#[test]
fn native_process_arguments_streams_cwd_and_exit_codes() {
    let dir = tempfile::tempdir().unwrap();
    let executable = tool(dir.path(), "probe");
    let args = [
        "a b",
        "",
        "ação",
        "$(touch unexpected)",
        "--stdin",
        "exit37",
    ];
    let mut child = Command::new(executable)
        .args(args)
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"literal stdin")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(processes::code(output.status), 37);
    let text = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(text.contains(&format!("{}\nliteral stdin", args.join("\n"))));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixture-stderr"));
    assert!(!dir.path().join("unexpected").exists());
    let just = tool(dir.path(), "just");
    assert_eq!(
        processes::run_just(dir.path(), &just, 1, 2, &["exit37".into()]).unwrap(),
        37
    );
    assert!(processes::initialize(&just, &["exit37".into()], dir.path()).is_err());
}
#[test]
fn native_executable_discovery_uses_path_with_spaces_and_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin ação");
    fs::create_dir(&bin).unwrap();
    let exe = tool(&bin, "cargo");
    assert_eq!(
        processes::find_in("cargo", &std::env::join_paths([&bin]).unwrap()),
        Some(exe)
    );
    assert!(processes::find_in("missing", bin.as_os_str()).is_none());
}
#[test]
fn root_resolution_preserves_unicode_and_explicit_precedence() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().canonicalize().unwrap();
    let expected = home.join("ação com espaços");
    assert_eq!(
        root::locate(
            Some(&expected),
            Some(Path::new("ignored").as_os_str()),
            Some(home.as_os_str())
        )
        .unwrap(),
        expected
    );
    assert_eq!(
        root::locate(None, None, Some(home.as_os_str())).unwrap(),
        home.join("fiap")
    );
}

#[cfg(windows)]
#[test]
fn windows_batch_extensions_and_exit_status_are_supported() {
    let dir = tempfile::tempdir().unwrap();
    for extension in ["cmd", "bat"] {
        let file = dir.path().join(format!("fixture.{extension}"));
        fs::write(&file, "@echo off\r\nexit /b 37\r\n").unwrap();
        assert_eq!(
            processes::find_in(&format!("fixture.{extension}"), dir.path().as_os_str()),
            Some(file.clone())
        );
        assert_eq!(processes::code(Command::new(&file).status().unwrap()), 37);
        let error =
            processes::initialize(&file, &["& echo unexpected".into()], dir.path()).unwrap_err();
        assert!(error.to_string().contains("execução segura"));
        let error = processes::initialize(
            &file,
            &["argumento com espaços".into(), "ação".into()],
            dir.path(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("37"));
    }
}
#[cfg(windows)]
#[test]
fn windows_default_init_and_linux_only_setup_have_explicit_behavior() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("fiap ação");
    let call = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_fp"))
            .arg("--root")
            .arg(&root)
            .args(args)
            .output()
            .unwrap()
    };
    let out = call(&["init"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(root.join("Justfile").is_file());
    let out = call(&["init", "--check"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = call(&[
        "db",
        "init",
        "--setup-runtime",
        "--no-input",
        "--output",
        "json",
    ]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    let message = String::from_utf8_lossy(&out.stderr);
    assert!(message.contains("exclusivo do Linux") && message.contains("Docker Desktop"));
}

#[test]
fn native_docker_mock_preserves_volumes_and_keeps_secrets_out_of_arguments() {
    use fp_database::{Engine, Manager, Paths, Resource};
    let dir = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(Paths {
        config: dir.path().join("config"),
        data: dir.path().join("data"),
        state: dir.path().join("state"),
    });
    manager.docker = tool(dir.path(), "docker");
    manager.init(false).unwrap();
    let mut resource = Resource::new("fixture".into(), Engine::Mongodb);
    resource.port = Some(29017);
    manager.create(resource, false, false).unwrap();
    manager
        .exec("fixture", b"db.example.findOne();", false)
        .unwrap();
    let secret = manager.generated_password("fixture", false).unwrap();
    let log = fs::read_to_string(manager.docker.with_extension("calls")).unwrap();
    assert!(log.contains("-p 127.0.0.1:29017:27017"));
    assert!(log.contains("fp-fixture-data:/data/db"));
    assert!(!log.contains(&secret));
    assert!(
        !fs::read_to_string(manager.paths.config.join("databases.json"))
            .unwrap()
            .contains(&secret)
    );
    manager
        .delete("fixture", "fixture", true, false, false)
        .unwrap();
    let log = fs::read_to_string(manager.docker.with_extension("calls")).unwrap();
    assert!(log.contains("rm fp-fixture"));
    assert!(!log.contains("volume rm"));
}
#[cfg(windows)]
#[test]
fn windows_docker_uses_user_context_and_rejects_persisted_unix_endpoint() {
    let dir = tempfile::tempdir().unwrap();
    let docker = tool(dir.path(), "docker");
    let path = std::env::join_paths(std::iter::once(dir.path().to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
    ))
    .unwrap();
    let call = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_fp"))
            .arg("db")
            .arg("--config-dir")
            .arg(dir.path().join("c"))
            .arg("--data-dir")
            .arg(dir.path().join("d"))
            .arg("--state-dir")
            .arg(dir.path().join("s"))
            .args(["--no-input", "--output", "json"])
            .args(args)
            .env("DOCKER_CONTEXT", "user-desktop")
            .env("PATH", &path)
            .output()
            .unwrap()
    };
    assert!(call(&["init"]).status.success());
    let out = call(&["create", "--engine", "mongodb", "--name", "fixture"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        fs::read_to_string(docker.with_extension("calls"))
            .unwrap()
            .contains("context=user-desktop")
    );
    let registry = dir.path().join("c/databases.json");
    let mut records: serde_json::Value =
        serde_json::from_slice(&fs::read(&registry).unwrap()).unwrap();
    records[0]["docker_host"] = serde_json::json!("unix:///linux/docker.sock");
    fs::write(registry, serde_json::to_vec(&records).unwrap()).unwrap();
    let before = fs::read(docker.with_extension("calls")).unwrap();
    let out = call(&["run", "--name", "fixture"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unix://"));
    assert_eq!(fs::read(docker.with_extension("calls")).unwrap(), before);
}

#[cfg(windows)]
#[test]
fn windows_configuration_uses_appdata_and_checks_do_not_create_state() {
    let dir = tempfile::tempdir().unwrap();
    let roaming = dir.path().join("roaming ação");
    let local = dir.path().join("local com espaços");
    let out = Command::new(env!("CARGO_BIN_EXE_fp"))
        .args(["db", "init", "--dry-run", "--no-input", "--output", "json"])
        .env("APPDATA", &roaming)
        .env("LOCALAPPDATA", &local)
        .env("HOME", dir.path().join("ignored"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        PathBuf::from(value["config"].as_str().unwrap()),
        roaming.join("fp")
    );
    assert_eq!(
        PathBuf::from(value["data"].as_str().unwrap()),
        local.join("fp/data")
    );
    assert_eq!(
        PathBuf::from(value["state"].as_str().unwrap()),
        local.join("fp/state")
    );
    assert!(!roaming.exists() && !local.exists());
    let root = dir.path().join("missing workspace");
    let out = Command::new(env!("CARGO_BIN_EXE_fp"))
        .arg("--root")
        .arg(&root)
        .args(["init", "--check"])
        .env("APPDATA", &roaming)
        .env("LOCALAPPDATA", &local)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(!root.exists() && !roaming.exists() && !local.exists());
}

#[cfg(windows)]
#[test]
fn windows_update_installation_refuses_without_network_or_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_fp"))
        .args(["update"])
        .env("PATH", dir.path())
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("indisponível no Windows"));
    assert!(out.stdout.is_empty());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}
