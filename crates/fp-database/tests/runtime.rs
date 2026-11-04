#![cfg(target_os = "linux")]
use fp_database::{runtime::*, *};
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Mutex,
};
struct Fake {
    calls: Mutex<Vec<String>>,
    images: Mutex<HashSet<String>>,
    prerequisite: bool,
}
impl Fake {
    fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
            images: Mutex::new(HashSet::new()),
            prerequisite: true,
        }
    }
}
impl SetupHost for Fake {
    fn prerequisites(&self, _: &RuntimeLocation) -> Result<()> {
        if self.prerequisite {
            Ok(())
        } else {
            Err("E_PREREQUISITE: fixture".into())
        }
    }
    fn download(&self, _: &str) -> Result<Vec<u8>> {
        Ok(b"corrupted download".to_vec())
    }
    fn command(&self, program: &Path, args: &[String], _: &[(String, String)]) -> Result<String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{} {}", program.display(), args.join(" ")));
        if let Some(i) = args.iter().position(|s| s == "inspect") {
            let name = &args[i + 1];
            if !self.images.lock().unwrap().contains(name) {
                return Err("image missing".into());
            }
        }
        if let Some(i) = args.iter().position(|s| s == "pull") {
            self.images.lock().unwrap().insert(args[i + 1].clone());
        }
        Ok(String::new())
    }
}
fn fixture() -> (tempfile::TempDir, Manager, RuntimeLocation) {
    let t = tempfile::tempdir().unwrap();
    let p = t.path();
    let m = Manager::new(Paths {
        config: p.join("config"),
        data: p.join("data"),
        state: p.join("state"),
    });
    let location = RuntimeLocation {
        runtime_dir: p.join("run"),
        unit_dir: p.join("units"),
    };
    (t, m, location)
}
fn seed(m: &Manager) {
    let bin = m
        .paths
        .data
        .join(format!("runtime/docker-{DOCKER_VERSION}"));
    fs::create_dir_all(&bin).unwrap();
    for name in [
        "docker",
        "dockerd",
        "containerd",
        "runc",
        "rootlesskit",
        "dockerd-rootless.sh",
        "slirp4netns",
    ] {
        let p = bin.join(name);
        fs::write(&p, "fixture").unwrap();
        fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
    }
}
#[test]
fn preflight_and_check_never_write_or_download() {
    let (_t, m, loc) = fixture();
    let fake = Fake::new();
    assert_eq!(m.setup_runtime(&loc, true, &fake).unwrap()["ready"], false);
    assert!(!m.paths.config.exists());
    let mut fail = Fake::new();
    fail.prerequisite = false;
    assert!(m.setup_runtime(&loc, false, &fail).is_err());
    assert!(!m.paths.config.exists());
    assert!(fake.calls.lock().unwrap().is_empty());
}
#[test]
fn checksum_failure_never_publishes_binaries_or_service() {
    let (_t, m, loc) = fixture();
    let error = m
        .setup_runtime(&loc, false, &Fake::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("E_CHECKSUM"));
    assert!(!loc.unit_dir.exists());
    assert!(m.installed_runtime().unwrap().is_none());
    assert!(
        !m.paths
            .data
            .join(format!("runtime/docker-{DOCKER_VERSION}"))
            .exists()
    );
    assert!(
        verify(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )
        .is_ok()
    );
}
#[test]
fn dedicated_service_downloads_images_once_without_sudo_or_login() {
    let (_t, m, loc) = fixture();
    seed(&m);
    let fake = Fake::new();
    m.setup_runtime(&loc, false, &fake).unwrap();
    m.setup_runtime(&loc, false, &fake).unwrap();
    let calls = fake.calls.lock().unwrap().join("\n");
    assert_eq!(calls.matches(" pull ").count(), 2);
    assert!(calls.contains("--user enable --now fp-docker.service"));
    assert!(!calls.contains("sudo"));
    assert!(!calls.contains(" login"));
    assert!(!calls.contains("context use"));
    let config = fs::read_to_string(loc.unit_dir.join("fp-docker.service")).unwrap();
    assert!(config.contains("--data-root="));
    assert!(config.contains("RuntimeDirectory=fp-docker"));
    assert!(config.contains("--config-file="));
    assert!(m.installed_runtime().unwrap().is_some());
}
#[test]
fn foreign_service_is_not_overwritten() {
    let (_t, m, loc) = fixture();
    seed(&m);
    fs::create_dir_all(&loc.unit_dir).unwrap();
    let p = loc.unit_dir.join("fp-docker.service");
    fs::write(&p, "custom service").unwrap();
    assert!(
        m.setup_runtime(&loc, false, &Fake::new())
            .unwrap_err()
            .to_string()
            .contains("E_CONFLICT")
    );
    assert_eq!(fs::read_to_string(p).unwrap(), "custom service");
}
#[test]
fn rootless_routing_and_generated_auth_do_not_change_the_legacy_client() {
    let (t, mut m, loc) = fixture();
    m.init(false).unwrap();
    let bin = t.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let docker = bin.join("docker");
    let socket = loc.runtime_dir.join("fp-docker/docker.sock");
    fs::write(
        m.paths.config.join("runtime.json"),
        serde_json::to_vec(&Runtime {
            bin: bin.clone(),
            socket: socket.clone(),
            version: DOCKER_VERSION.into(),
        })
        .unwrap(),
    )
    .unwrap();
    let body = format!(
        r#"#!/bin/sh
test "$DOCKER_HOST" = 'unix://{}' || exit 12
test -z "$DOCKER_CONTEXT" || exit 13
case "$1 $2" in
'info '|'ps -a'|'volume ls') exit 0;;
'volume create') echo volume;;
'create --name') test "${{#MONGO_INITDB_ROOT_PASSWORD}}" = 64 || exit 14; test "$MONGO_INITDB_ROOT_USERNAME" = fp || exit 15; echo mongo-id;;
'container inspect') echo '[{{"Id":"mongo-id","State":{{"Running":true}}}}]';;
'exec -i') test "${{#FP_DB_PASSWORD}}" = 64 || exit 16; cat >/dev/null;;
*) exit 1;;
esac
"#,
        socket.display()
    );
    fs::write(&docker, body).unwrap();
    fs::set_permissions(&docker, fs::Permissions::from_mode(0o755)).unwrap();
    m.docker = PathBuf::from("/never/use/system/docker");
    let result = m
        .create(Resource::new("mongo".into(), Engine::Mongodb), false, false)
        .unwrap();
    assert!(
        result["docker_host"]
            .as_str()
            .unwrap()
            .contains("fp-docker")
    );
    m.exec("mongo", b"db.version();", false).unwrap();
    let secret = m.generated_password("mongo", false).unwrap();
    assert!(
        !fs::read_to_string(m.paths.config.join("databases.json"))
            .unwrap()
            .contains(&secret)
    );
    assert_eq!(m.docker, PathBuf::from("/never/use/system/docker"));
}
