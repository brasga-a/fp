use fp_database::*;
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};
fn tool(root: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = root.join(name);
    fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    p
}
fn manager() -> (tempfile::TempDir, Manager) {
    let t = tempfile::tempdir().unwrap();
    let p = t.path();
    let mut m = Manager::new(Paths {
        config: p.join("config"),
        data: p.join("data"),
        state: p.join("state"),
    });
    m.docker = p.join("never-docker");
    (t, m)
}
#[test]
fn init_queries_dryrun_and_lock() {
    let (_t, m) = manager();
    assert!(m.list().unwrap().is_empty());
    assert!(!m.paths.config.exists());
    m.init(true).unwrap();
    assert!(!m.paths.config.exists());
    m.init(false).unwrap();
    m.init(false).unwrap();
    assert!(m.list().unwrap().is_empty());
    fs::write(m.paths.state.join("database.lock"), "busy").unwrap();
    assert!(
        m.create(Resource::new("one".into(), Engine::Sqlite), false, false)
            .unwrap_err()
            .to_string()
            .contains("E_BUSY")
    );
}
#[test]
fn sqlite_lifecycle_preserves_and_requires_explicit_deletion() {
    let (_t, m) = manager();
    m.init(false).unwrap();
    m.create(Resource::new("one".into(), Engine::Sqlite), false, false)
        .unwrap();
    let r = m.get("one").unwrap();
    let p = r.file.unwrap();
    assert!(p.exists());
    m.exec("one", b"select 1;", false).unwrap();
    m.run("one", false, false, false).unwrap();
    assert!(m.delete("one", "wrong", false, true, false).is_err());
    m.delete("one", "one", false, true, true).unwrap();
    assert!(p.exists());
    m.delete("one", "one", false, false, false).unwrap();
    assert!(p.exists());
    let mut r = Resource::new("import".into(), Engine::Sqlite);
    r.file = Some(p.clone());
    m.create(r, true, false).unwrap();
    assert!(m.delete("import", "import", false, true, false).is_err());
    assert!(p.exists());
}
#[test]
fn sqlite_conflicts_failed_create_and_replaced_files() {
    let (_t, m) = manager();
    m.init(false).unwrap();
    let r = Resource::new("one".into(), Engine::Sqlite);
    m.create(r.clone(), false, false).unwrap();
    assert!(m.create(r, false, false).is_err());
    let p = m.get("one").unwrap().file.unwrap();
    let replacement = p.with_extension("new");
    fs::write(&replacement, "other database").unwrap();
    fs::rename(replacement, &p).unwrap();
    assert!(m.exec("one", b"drop table x;", false).is_err());
    assert!(m.delete("one", "one", false, true, false).is_err());
    assert!(
        m.create(
            {
                let mut r = Resource::new("failed".into(), Engine::Sqlite);
                r.file = Some(m.paths.data.join("missing/file.db"));
                r
            },
            false,
            false
        )
        .is_err()
    );
    assert!(!m.paths.data.join("failed.db").exists());
    assert_eq!(m.list().unwrap().len(), 1);
}
fn adopted(m: &Manager) {
    let mut r = Resource::new("legacy".into(), Engine::Oracle);
    r.container = Some("fiap-oracle".into());
    r.volume = Some("fiap-oracle-data".into());
    r.image = Some("container-registry.oracle.com/database/free:latest".into());
    m.create(r, true, false).unwrap();
}
#[test]
fn legacy_reuse_identity_preservation_and_no_implicit_provision() {
    let (t, mut m) = manager();
    m.init(false).unwrap();
    let log = t.path().join("calls");
    m.docker = tool(
        t.path(),
        "docker",
        &format!(
            r#"printf '%s\n' "$*" >> '{}'
case "$1" in
info) exit 0;;
container) printf '%s' '[{{"Id":"legacy-id","Config":{{"Image":"container-registry.oracle.com/database/free:latest"}},"Mounts":[{{"Name":"fiap-oracle-data"}}],"State":{{"Running":true,"Health":{{"Status":"starting"}}}}}}]';;
start|stop) exit 0;;
*) exit 1;;
esac"#,
            log.display()
        ),
    );
    adopted(&m);
    assert!(!m.get("legacy").unwrap().owned);
    m.run("legacy", false, false, false).unwrap();
    assert!(m.delete("legacy", "legacy", true, true, false).is_err());
    let calls = fs::read_to_string(log).unwrap();
    assert!(!calls.contains("exec"));
    assert!(!calls.contains("volume rm"));
    assert!(!calls.contains("create"));
    m.delete("legacy", "legacy", false, false, false).unwrap();
}
#[test]
fn docker_failure_and_timeout_do_not_create_registry_entries() {
    let (t, mut m) = manager();
    m.init(false).unwrap();
    let r = Resource::new("legacy".into(), Engine::Oracle);
    assert!(
        m.create(r.clone(), true, false)
            .unwrap_err()
            .to_string()
            .contains("E_TOOL")
    );
    m.docker = tool(t.path(), "docker", "exec /usr/bin/sleep 2");
    m.timeout = Duration::from_millis(100);
    let error = m.create(r, true, false).unwrap_err().to_string();
    assert!(error.contains("E_TIMEOUT"), "{error}");
    assert!(m.list().unwrap().is_empty());
    assert!(!m.paths.state.join("database.lock").exists());
}
#[test]
fn credentials_validation_and_names() {
    let (_t, m) = manager();
    m.init(false).unwrap();
    let mut r = Resource::new("mongo".into(), Engine::Mongodb);
    let plan = m.create(r.clone(), false, true).unwrap();
    assert_eq!(plan["resource"]["username"], "fp");
    assert!(!m.paths.config.join("secrets").exists());
    r.password_env = Some("INVALID env".into());
    assert!(m.create(r, false, true).is_err());
    for n in ["../other", "-flag", "", "a b"] {
        assert!(valid_name(n).is_err());
    }
    assert!(m.list().unwrap().is_empty());
}
#[test]
fn docker_provision_persistent_volume_no_start_and_explicit_removal() {
    let (t, mut m) = manager();
    m.init(false).unwrap();
    let log = t.path().join("calls");
    m.docker = tool(
        t.path(),
        "docker",
        &format!(
            r#"printf '%s\n' "$*" >> '{}'
case "$1 $2" in
'info '|'ps -a'|'volume ls'|'rm fp-new'|'volume rm') exit 0;;
'volume create') echo fp-new-data;;
'create --name') echo new-id;;
'container inspect') echo '[{{"Id":"new-id","State":{{"Running":false}},"Config":{{"Image":"container-registry.oracle.com/database/free:latest"}}}}]';;
'volume inspect') echo '[{{"Labels":{{"fp.resource":"new"}}}}]';;
'exec -i') cat > /dev/null;;
*) exit 1;;
esac"#,
            log.display()
        ),
    );
    let mut r = Resource::new("new".into(), Engine::Oracle);
    r.password_env = Some("PATH".into());
    m.create(r, false, false).unwrap();
    let calls = fs::read_to_string(&log).unwrap();
    assert!(calls.contains("volume create --label fp.resource=new"));
    assert!(calls.contains("-v fp-new-data:/opt/oracle/oradata"));
    assert!(!calls.contains("start "));
    assert!(!calls.contains(&std::env::var("PATH").unwrap()));
    assert!(m.delete("new", "new", false, true, false).is_err());
    m.provision_user("new", "student", "PATH", false).unwrap();
    m.delete("new", "new", true, true, false).unwrap();
    assert!(m.list().unwrap().is_empty());
    assert!(
        fs::read_to_string(log)
            .unwrap()
            .contains("volume rm fp-new-data")
    );
}
#[test]
fn readiness_timeout_and_stopped_start_are_reported() {
    let (t, mut m) = manager();
    m.init(false).unwrap();
    let make = |running: bool| {
        format!(
            r#"case "$1" in
info|start) exit 0;;
container) echo '[{{"Id":"legacy-id","Config":{{"Image":"container-registry.oracle.com/database/free:latest"}},"Mounts":[{{"Name":"fiap-oracle-data"}}],"State":{{"Running":{running},"Health":{{"Status":"starting"}}}}}}]';;
*) exit 1;;
esac"#
        )
    };
    m.docker = tool(t.path(), "docker", &make(true));
    adopted(&m);
    m.timeout = Duration::from_millis(100);
    assert!(
        m.run("legacy", false, true, false)
            .unwrap_err()
            .to_string()
            .contains("E_TIMEOUT")
    );
    m.docker = tool(t.path(), "docker", &make(false));
    assert!(
        m.run("legacy", false, true, false)
            .unwrap_err()
            .to_string()
            .contains("E_START")
    );
    assert_eq!(m.list().unwrap().len(), 1);
}
#[test]
fn mongodb_authentication_is_forwarded_without_argv_or_registry_secrets() {
    let (t, mut m) = manager();
    m.init(false).unwrap();
    let script = t.path().join("script");
    let calls = t.path().join("calls");
    m.docker = tool(
        t.path(),
        "docker",
        &format!(
            r#"printf '%s\n' "$*" >> '{}'
case "$1 $2" in
'info '|'ps -a'|'volume ls') exit 0;;
'volume create') echo fp-mongo-data;;
'create --name') test -n "$MONGO_INITDB_ROOT_PASSWORD" || exit 19; echo mongo-id;;
'container inspect') echo '[{{"Id":"mongo-id","State":{{"Running":true}}}}]';;
'exec -i') test -n "$FP_DB_PASSWORD" || exit 20; cat > '{}';;
*) exit 1;;
esac"#,
            calls.display(),
            script.display()
        ),
    );
    let mut r = Resource::new("mongo".into(), Engine::Mongodb);
    r.password_env = Some("PATH".into());
    r.username = Some("fixture".into());
    m.create(r, false, false).unwrap();
    m.exec("mongo", b"db.example.findOne();", false).unwrap();
    let input = fs::read_to_string(script).unwrap();
    assert!(input.contains(".auth(\"fixture\", process.env.FP_DB_PASSWORD)"));
    assert!(input.contains("db.example.findOne();"));
    let recorded = fs::read_to_string(calls).unwrap();
    assert!(recorded.contains("--file /dev/stdin"));
    let value = std::env::var("PATH").unwrap();
    assert!(!recorded.contains(&value));
    assert!(
        !fs::read_to_string(m.paths.config.join("databases.json"))
            .unwrap()
            .contains(&value)
    );
}
