use clap::Parser;
use fiap_projects::{
    cli::{Cli, Commands},
    delete, registry,
};
use std::{fs, os::unix::fs::symlink, process::Command};
use tempfile::{TempDir, tempdir};

fn repo() -> TempDir {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("Justfile"), "# intact\n").unwrap();
    for path in [
        "fase-01/cap-01",
        "fase-01/cap-02",
        "fase-02/cap-01",
        "docs/fase-01",
    ] {
        fs::create_dir_all(dir.path().join(path)).unwrap();
        fs::write(
            dir.path().join(path).join("original"),
            "preserve until confirmed",
        )
        .unwrap();
    }
    fs::write(dir.path().join("chapters.toml"), "# original comment\n[chapters]\n[chapters.'1.1']\npath = 'fase-01/cap-01'\n[chapters.'1.1'.commands.main]\nargv = ['echo', 'ok']\n[chapters.'1.2']\npath = 'fase-01/cap-02'\n[chapters.'2.1']\npath = 'fase-02/cap-01'\n").unwrap();
    dir
}

#[test]
fn parses_delete_and_root() {
    let cli = Cli::try_parse_from(["fp", "--root", "/tmp", "delete"]).unwrap();
    assert!(matches!(cli.command, Commands::Delete));
    assert!(Cli::try_parse_from(["fp", "delete", "1", "2"]).is_err());
}
#[test]
fn deleting_chapter_preserves_siblings_and_comments() {
    let dir = repo();
    let plan = delete::prepare(dir.path(), 1, Some(1)).unwrap();
    assert_eq!(plan.registrations, ["1.1"]);
    delete::execute(&plan, true).unwrap();
    assert!(!dir.path().join("fase-01/cap-01").exists());
    assert!(dir.path().join("fase-01/cap-02/original").is_file());
    let doc = registry::load(dir.path()).unwrap();
    assert!(doc["chapters"].get("1.1").is_none());
    assert!(doc["chapters"].get("1.2").is_some());
    assert!(doc.to_string().contains("# original comment"));
    assert_eq!(
        fs::read_to_string(dir.path().join("Justfile")).unwrap(),
        "# intact\n"
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 5);
}
#[test]
fn deleting_phase_removes_its_contents_and_registrations_only() {
    let dir = repo();
    fs::write(dir.path().join("fase-01/anotacao.md"), "note").unwrap();
    let plan = delete::prepare(dir.path(), 1, None).unwrap();
    assert_eq!(plan.registrations, ["1.1", "1.2"]);
    delete::execute(&plan, true).unwrap();
    assert!(!dir.path().join("fase-01").exists());
    assert!(dir.path().join("fase-02/cap-01/original").is_file());
    assert!(dir.path().join("docs/fase-01/original").is_file());
    assert_eq!(
        registry::load(dir.path()).unwrap()["chapters"]
            .as_table()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn cancellation_does_not_modify_anything() {
    let dir = repo();
    let before = fs::read_to_string(dir.path().join("chapters.toml")).unwrap();
    for chapter in [Some(1), None] {
        let plan = delete::prepare(dir.path(), 1, chapter).unwrap();
        delete::execute(&plan, false).unwrap();
        assert!(dir.path().join("fase-01/cap-01/original").is_file());
        assert_eq!(
            fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
            before
        );
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 5);
    }
}
#[test]
fn unregistered_directory_can_be_deleted() {
    let dir = repo();
    fs::create_dir(dir.path().join("fase-01/cap-03")).unwrap();
    let before = fs::read_to_string(dir.path().join("chapters.toml")).unwrap();
    let plan = delete::prepare(dir.path(), 1, Some(3)).unwrap();
    assert!(plan.registrations.is_empty());
    delete::execute(&plan, true).unwrap();
    assert!(!dir.path().join("fase-01/cap-03").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
        before
    );
}
#[test]
fn changed_registry_and_busy_lock_abort_without_deleting() {
    let dir = repo();
    let plan = delete::prepare(dir.path(), 1, Some(1)).unwrap();
    fs::write(dir.path().join(".fp-init.lock"), "busy").unwrap();
    assert!(delete::execute(&plan, true).is_err());
    assert!(dir.path().join("fase-01/cap-01/original").is_file());
    fs::remove_file(dir.path().join(".fp-init.lock")).unwrap();
    let mut text = fs::read_to_string(dir.path().join("chapters.toml")).unwrap();
    text.push_str("# changed externally\n");
    fs::write(dir.path().join("chapters.toml"), &text).unwrap();
    assert!(
        delete::execute(&plan, true)
            .unwrap_err()
            .to_string()
            .contains("Registro alterado")
    );
    assert!(dir.path().join("fase-01/cap-01/original").is_file());
    assert_eq!(
        fs::read_to_string(dir.path().join("chapters.toml")).unwrap(),
        text
    );
    assert!(!dir.path().join(".fp-init.lock").exists());
}
#[test]
fn replaced_directory_is_not_deleted() {
    let dir = repo();
    let plan = delete::prepare(dir.path(), 1, Some(1)).unwrap();
    fs::rename(
        dir.path().join("fase-01/cap-01"),
        dir.path().join("original-chapter"),
    )
    .unwrap();
    fs::create_dir(dir.path().join("fase-01/cap-01")).unwrap();
    assert!(delete::execute(&plan, true).is_err());
    assert!(dir.path().join("fase-01/cap-01").is_dir());
    assert!(dir.path().join("original-chapter/original").is_file());
}
#[test]
fn symlinks_and_invalid_selections_are_rejected() {
    let dir = repo();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("keep"), "outside").unwrap();
    symlink(outside.path(), dir.path().join("fase-09")).unwrap();
    symlink(outside.path(), dir.path().join("fase-01/cap-09")).unwrap();
    for (phase, chapter) in [(0, None), (9, None), (1, Some(9)), (1, Some(99))] {
        assert!(delete::prepare(dir.path(), phase, chapter).is_err());
    }
    // Links dentro da pasta excluída são removidos sem percorrer seus destinos.
    symlink(outside.path(), dir.path().join("fase-01/cap-01/link")).unwrap();
    delete::execute(&delete::prepare(dir.path(), 1, Some(1)).unwrap(), true).unwrap();
    assert!(outside.path().join("keep").is_file());
}
#[test]
fn surviving_command_cwd_inside_target_blocks_deletion() {
    let dir = repo();
    let path = dir.path().join("chapters.toml");
    let mut text = fs::read_to_string(&path).unwrap();
    text.push_str("[chapters.'1.9']\npath = 'fase-01'\n[chapters.'1.9'.commands.main]\nargv = ['echo', 'ok']\ncwd = 'cap-01'\n");
    fs::write(path, text).unwrap();
    assert!(
        delete::prepare(dir.path(), 1, Some(1))
            .err()
            .unwrap()
            .to_string()
            .contains("referencia")
    );
}

#[test]
fn delete_without_terminal_reports_error() {
    let dir = repo();
    let result = Command::new(env!("CARGO_BIN_EXE_fp"))
        .args(["--root"])
        .arg(dir.path())
        .arg("delete")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("terminal interativo"));
    assert!(dir.path().join("fase-01/cap-01").is_dir());
}
#[test]
fn registry_aliases_inside_deleted_directory_are_removed() {
    let dir = repo();
    let path = dir.path().join("chapters.toml");
    let text = fs::read_to_string(&path).unwrap() + "[chapters.'8.8']\npath = 'fase-01/cap-01'\n";
    fs::write(path, text).unwrap();
    let plan = delete::prepare(dir.path(), 1, Some(1)).unwrap();
    assert_eq!(plan.registrations, ["1.1", "8.8"]);
    delete::execute(&plan, true).unwrap();
    assert!(
        registry::load(dir.path()).unwrap()["chapters"]
            .get("8.8")
            .is_none()
    );
}
