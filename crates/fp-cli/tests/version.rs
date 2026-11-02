use clap::CommandFactory;
use fiap_projects::cli::Cli;
use std::process::Command;

#[test]
fn clap_uses_the_cargo_package_version() {
    assert_eq!(
        Cli::command().get_version(),
        Some(env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn version_flags_work_without_workspace_or_external_tools() {
    let home = tempfile::tempdir().unwrap();
    for flag in ["--version", "-V"] {
        let result = Command::new(env!("CARGO_BIN_EXE_fp"))
            .arg(flag)
            .env("HOME", home.path())
            .env("FP_ROOT", home.path().join("missing-root"))
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(result.status.success());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            format!("fp {}\n", env!("CARGO_PKG_VERSION"))
        );
        assert!(result.stderr.is_empty());
    }
}
