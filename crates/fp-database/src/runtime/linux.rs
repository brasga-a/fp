//! Runtime rootless próprio: downloads fixos, sem instalador remoto ou alteração de contexto.
use super::{MONGO_IMAGE, ORACLE_IMAGE, Runtime};
use crate::{Manager, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
pub const DOCKER_VERSION: &str = "29.9.2";
const DOWNLOADS: [(&str, &str, &str); 3] = [
    (
        "docker",
        "https://download.docker.com/linux/static/stable/x86_64/docker-29.9.2.tgz",
        "9ac225999fd5e863f7694c728ef23d480616b1e5639328fa309befd4243d69eb",
    ),
    (
        "extras",
        "https://download.docker.com/linux/static/stable/x86_64/docker-rootless-extras-29.9.2.tgz",
        "f2663b230a7afc13dea88fc589685331d01f5f3fc458b92ab40e2388db452ed2",
    ),
    (
        "slirp4netns",
        "https://github.com/rootless-containers/slirp4netns/releases/download/v1.3.7/slirp4netns-x86_64",
        "1c177694fe6bf299debfd8ff601a2ee3c4f7554d9983acdf1f854a6b34d53e70",
    ),
];
#[derive(Clone)]
pub struct RuntimeLocation {
    pub runtime_dir: PathBuf,
    pub unit_dir: PathBuf,
}
impl RuntimeLocation {
    pub fn detect() -> Result<Self> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or(
                "E_PREREQUISITE: XDG_RUNTIME_DIR ausente; use uma sessão de usuário com systemd",
            )?;
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("E_CONFIG: HOME ausente")?);
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        if !runtime_dir.is_absolute() || runtime_dir.starts_with("/tmp") || !config.is_absolute() {
            return Err("E_PREREQUISITE: diretórios de runtime/configuração inválidos".into());
        }
        Ok(Self {
            runtime_dir,
            unit_dir: config.join("systemd/user"),
        })
    }
}
pub trait SetupHost {
    fn prerequisites(&self, location: &RuntimeLocation) -> Result<()>;
    fn download(&self, url: &str) -> Result<Vec<u8>>;
    fn command(&self, program: &Path, args: &[String], env: &[(String, String)]) -> Result<String>;
}
pub struct NativeSetup<'a>(pub &'a Manager);
impl SetupHost for NativeSetup<'_> {
    fn prerequisites(&self, location: &RuntimeLocation) -> Result<()> {
        if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
            return Err("E_PLATFORM: bootstrap rootless disponível para Linux x86_64".into());
        }
        let uid = self
            .command(Path::new("id"), &["-u".into()], &[])?
            .trim()
            .parse::<u32>()?;
        let user = self.command(Path::new("id"), &["-un".into()], &[])?;
        let user = user.trim();
        if uid == 0 {
            return Err("E_PREREQUISITE: execute fp init como usuário comum, sem sudo".into());
        }
        for tool in ["newuidmap", "newgidmap", "iptables", "systemctl"] {
            if !std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).any(|p| {
                fs::metadata(p.join(tool)).is_ok_and(|m| m.is_file() && m.mode() & 0o111 != 0)
            }) {
                return Err(format!("E_PREREQUISITE: instale {tool} uma vez pelo gerenciador do sistema; FP não executa sudo").into());
            }
        }
        for file in ["/etc/subuid", "/etc/subgid"] {
            let data = fs::read_to_string(file).unwrap_or_default();
            if !data.lines().any(|l| {
                let p = l.split(':').collect::<Vec<_>>();
                p.len() == 3
                    && (p[0] == user || p[0] == uid.to_string())
                    && p[2].parse::<u64>().is_ok_and(|n| n >= 65536)
            }) {
                return Err(format!("E_PREREQUISITE: configure ao menos 65536 IDs para {user} em {file}, uma única vez com o administrador").into());
            }
        }
        let meta = fs::symlink_metadata(&location.runtime_dir)?;
        if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o077 != 0 {
            return Err(
                "E_PREREQUISITE: XDG_RUNTIME_DIR deve ser privado e pertencer ao usuário".into(),
            );
        }
        if fs::read_to_string("/proc/sys/kernel/unprivileged_userns_clone")
            .is_ok_and(|s| s.trim() == "0")
        {
            return Err("E_PREREQUISITE: user namespaces estão desabilitados pelo sistema".into());
        }
        self.command(
            Path::new("systemctl"),
            &["--user".into(), "show-environment".into()],
            &[],
        )?;
        Ok(())
    }
    fn download(&self, url: &str) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        self_update::Download::from_url(url)
            .timeout(Duration::from_secs(300))
            .download_to(&mut data)
            .map_err(
                |_| "E_DOWNLOAD: falha ao baixar componente oficial; execute fp init novamente",
            )?;
        Ok(data)
    }
    fn command(&self, program: &Path, args: &[String], env: &[(String, String)]) -> Result<String> {
        let mut m = self.0.clone();
        m.timeout = Duration::from_secs(if args.iter().any(|a| a == "pull") {
            1800
        } else {
            30
        });
        m.call(program, args, None, env)
    }
}
pub fn verify(bytes: &[u8], expected: &str) -> Result<()> {
    if Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        != expected
    {
        return Err("E_CHECKSUM: componente inválido; nenhum binário será instalado".into());
    }
    Ok(())
}
fn unpack(bytes: &[u8], root: &Path) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    for item in archive.entries()? {
        let mut entry = item?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let path = entry.path()?.into_owned();
        let parts = path.components().collect::<Vec<_>>();
        if parts.len() != 2 || !entry.header().entry_type().is_file() {
            return Err("E_ARCHIVE: entrada inesperada no pacote".into());
        }
        let name = path.file_name().ok_or("E_ARCHIVE: nome ausente")?;
        let target = root.join(name);
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        std::io::copy(&mut entry, &mut f)?;
        fs::set_permissions(target, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}
fn quote(s: &str) -> Result<String> {
    if s.chars().any(char::is_control) {
        return Err("E_CONFIG: caminho contém caracteres inválidos".into());
    }
    Ok(format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$")
    ))
}
pub fn unit(runtime: &Runtime, data: &Path, location: &RuntimeLocation) -> Result<String> {
    let path = format!(
        "{}:/usr/local/sbin:/usr/local/bin:/usr/bin:/bin",
        runtime.bin.display()
    );
    let state = location.runtime_dir.join("fp-docker/rootlesskit");
    let launch = quote(
        &runtime
            .bin
            .join("dockerd-rootless.sh")
            .display()
            .to_string(),
    )?;
    Ok(format!(
        "# Managed by FP: dedicated rootless runtime\n[Unit]\nDescription=FP Docker rootless\n[Service]\nType=simple\nEnvironment={}\nEnvironment={}\nEnvironment=DOCKERD_ROOTLESS_ROOTLESSKIT_NET=slirp4netns\nExecStart={launch} --host={} --data-root={} --exec-root={} --pidfile={}\nRestart=on-failure\nRestartSec=2\nTimeoutStartSec=0\nRuntimeDirectory=fp-docker\nRuntimeDirectoryMode=0700\nDelegate=yes\nKillMode=mixed\n[Install]\nWantedBy=default.target\n",
        quote(&format!("PATH={path}"))?,
        quote(&format!(
            "DOCKERD_ROOTLESS_ROOTLESSKIT_STATE_DIR={}",
            state.display()
        ))?,
        quote(&format!("unix://{}", runtime.socket.display()))?,
        quote(&data.join("docker").display().to_string())?,
        quote(
            &location
                .runtime_dir
                .join("fp-docker/exec")
                .display()
                .to_string()
        )?,
        quote(
            &location
                .runtime_dir
                .join("fp-docker/dockerd.pid")
                .display()
                .to_string()
        )?
    ))
}
impl Manager {
    pub fn installed_runtime(&self) -> Result<Option<Runtime>> {
        match fs::read(self.paths.config.join("runtime.json")) {
            Ok(b) => Ok(Some(serde_json::from_slice(&b)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub fn setup_runtime(
        &self,
        location: &RuntimeLocation,
        check: bool,
        host: &dyn SetupHost,
    ) -> Result<Value> {
        host.prerequisites(location)?;
        let runtime = Runtime {
            bin: self
                .paths
                .data
                .join(format!("runtime/docker-{DOCKER_VERSION}")),
            socket: location.runtime_dir.join("fp-docker/docker.sock"),
            version: DOCKER_VERSION.into(),
        };
        if check {
            let mut ready = self.installed_runtime()?.is_some()
                && runtime.bin.join("docker").is_file()
                && runtime.socket.exists();
            if ready {
                let env = vec![(
                    "DOCKER_CONFIG".into(),
                    self.paths
                        .config
                        .join("docker-client")
                        .display()
                        .to_string(),
                )];
                for suffix in [
                    vec!["info".into()],
                    vec!["image".into(), "inspect".into(), ORACLE_IMAGE.into()],
                    vec!["image".into(), "inspect".into(), MONGO_IMAGE.into()],
                ] {
                    let mut args = vec![
                        "--host".into(),
                        format!("unix://{}", runtime.socket.display()),
                    ];
                    args.extend(suffix);
                    ready &= host
                        .command(&runtime.bin.join("docker"), &args, &env)
                        .is_ok();
                }
            }
            return Ok(
                json!({"ready":ready,"version":DOCKER_VERSION,"socket":runtime.socket,"check":true}),
            );
        }
        self.init(false)?;
        let _lock = self.lock()?;
        if !runtime.bin.is_dir() {
            let parent = runtime.bin.parent().unwrap();
            fs::create_dir_all(parent)?;
            let stage = tempfile::tempdir_in(parent)?;
            for (name, url, digest) in DOWNLOADS {
                let bytes = host.download(url)?;
                verify(&bytes, digest)?;
                if name == "slirp4netns" {
                    let p = stage.path().join(name);
                    fs::write(&p, bytes)?;
                    fs::set_permissions(p, fs::Permissions::from_mode(0o755))?;
                } else {
                    unpack(&bytes, stage.path())?;
                }
            }
            for file in [
                "docker",
                "dockerd",
                "containerd",
                "runc",
                "rootlesskit",
                "dockerd-rootless.sh",
                "slirp4netns",
            ] {
                if !stage.path().join(file).is_file() {
                    return Err("E_ARCHIVE: pacote incompleto".into());
                }
            }
            fs::rename(stage.path(), &runtime.bin)?;
        }
        for name in [
            "docker",
            "dockerd",
            "containerd",
            "runc",
            "rootlesskit",
            "dockerd-rootless.sh",
            "slirp4netns",
        ] {
            let m=fs::symlink_metadata(runtime.bin.join(name)).map_err(|_|"E_RUNTIME: runtime incompleto; remova apenas o diretório runtime/docker-VERSAO e execute init novamente")?;
            if !m.is_file() || m.mode() & 0o111 == 0 {
                return Err("E_RUNTIME: componente inválido".into());
            }
        }
        fs::create_dir_all(&location.unit_dir)?;
        let target = location.unit_dir.join("fp-docker.service");
        let config = self.paths.config.join("docker-daemon.json");
        if !config.exists() {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&config)?;
            f.write_all(b"{\"log-driver\":\"local\"}\n")?;
        }
        let contents = unit(&runtime, &self.paths.data, location)?.replace(
            " --host=",
            &format!(
                " --config-file={} --host=",
                quote(&config.display().to_string())?
            ),
        );
        if target.exists() {
            let before = fs::read_to_string(&target)?;
            if !before.starts_with("# Managed by FP:") {
                return Err("E_CONFLICT: fp-docker.service existente não pertence ao FP".into());
            }
            if before != contents {
                return Err(
                    "E_CONFLICT: serviço FP mudou; revise o arquivo antes de reconfigurar".into(),
                );
            }
        } else {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            f.write_all(contents.as_bytes())?;
        }
        host.command(
            Path::new("systemctl"),
            &["--user".into(), "daemon-reload".into()],
            &[],
        )?;
        host.command(
            Path::new("systemctl"),
            &[
                "--user".into(),
                "enable".into(),
                "--now".into(),
                "fp-docker.service".into(),
            ],
            &[],
        )?;
        let env = vec![(
            "DOCKER_CONFIG".into(),
            self.paths
                .config
                .join("docker-client")
                .display()
                .to_string(),
        )];
        let docker = runtime.bin.join("docker");
        let prefix = vec![
            "--host".to_string(),
            format!("unix://{}", runtime.socket.display()),
        ];
        let mut info = prefix.clone();
        info.push("info".into());
        let start = std::time::Instant::now();
        loop {
            if host.command(&docker, &info, &env).is_ok() {
                break;
            }
            if start.elapsed() > Duration::from_secs(60) {
                return Err("E_RUNTIME: Docker rootless não iniciou; consulte journalctl --user -u fp-docker.service".into());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let mut file = tempfile::NamedTempFile::new_in(&self.paths.config)?;
        serde_json::to_writer_pretty(&mut file, &runtime)?;
        file.persist(self.paths.config.join("runtime.json"))?;
        for image in [ORACLE_IMAGE, MONGO_IMAGE] {
            let mut args = prefix.clone();
            let mut inspect = prefix.clone();
            inspect.extend(["image".into(), "inspect".into(), image.into()]);
            if host.command(&docker, &inspect, &env).is_ok() {
                continue;
            }
            args.extend(["pull".into(), image.into()]);
            host.command(&docker,&args,&env).map_err(|_|"E_IMAGE: falha ao baixar imagem pública; execute fp init novamente (sem docker login)")?;
        }
        Ok(
            json!({"ready":true,"version":DOCKER_VERSION,"images":[ORACLE_IMAGE,MONGO_IMAGE],"socket":runtime.socket,"authentication":"generated-local"}),
        )
    }
}
