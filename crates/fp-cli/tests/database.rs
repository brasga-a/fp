use clap::Parser;
use fiap_projects::cli::{Cli, Commands};
use std::{fs, process::Command};
#[test]
fn aliases_and_conflicting_actions() {
    for alias in ["db", "database"] {
        for (word, flag) in [
            ("init", "--init"),
            ("create", "--create"),
            ("delete", "--delete"),
            ("run", "--run"),
            ("visual", "--visual"),
        ] {
            for action in [word, flag] {
                let c = Cli::try_parse_from(["fp", alias, action, "--name", "demo", "--no-input"])
                    .unwrap();
                assert!(matches!(c.command, Commands::Db(_)));
            }
        }
    }
    assert!(Cli::try_parse_from(["fp", "db", "--create", "--delete"]).is_err());
}
fn call(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fp"))
        .args(["db", "--config-dir"])
        .arg(root.join("config"))
        .arg("--data-dir")
        .arg(root.join("data"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(["--no-input", "--output", "json"])
        .args(args)
        .env("HOME", root)
        .env("FP_ROOT", "/missing-academic-root")
        .env("FP_DB_BIN", "/must-not-execute")
        .env("PATH", root)
        .output()
        .unwrap()
}
#[test]
fn noninteractive_json_and_sqlite_workflow() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path();
    let listed = call(p, &["list"]);
    assert!(listed.status.success());
    let json: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(json["resources"], serde_json::json!([]));
    assert!(!p.join("config").exists());
    for args in [
        vec!["--init"],
        vec!["--create", "--engine", "sqlite", "--name", "test"],
        vec!["status", "--name", "test"],
    ] {
        let out = call(p, &args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap();
    }
    let script = p.join("sql");
    fs::write(&script, "CREATE TABLE example(id INTEGER);").unwrap();
    assert!(
        call(
            p,
            &["exec", "--name", "test", "--file", script.to_str().unwrap()]
        )
        .status
        .success()
    );
    let out = call(p, &["delete", "--name", "test"]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(p.join("data/test.db").exists());
    assert!(
        call(p, &["delete", "--name", "test", "--confirm", "test"])
            .status
            .success()
    );
    assert!(p.join("data/test.db").exists());
    assert!(!call(p, &[]).status.success());
    assert!(!call(p, &["--visual"]).status.success());
}
#[test]
fn missing_inputs_and_legacy_migration_do_not_prompt() {
    let t = tempfile::tempdir().unwrap();
    for args in [vec!["create"], vec!["run"], vec!["init", "--oracle"]] {
        let o = call(t.path(), &args);
        assert!(!o.status.success());
        assert!(o.stdout.is_empty());
        assert!(!o.stderr.contains(&0x1b));
    }
    assert!(!t.path().join("config").exists());
}
