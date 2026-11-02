use crate::*;
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::MetadataExt,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
#[derive(Clone)]
pub struct Manager {
    pub paths: Paths,
    pub docker: PathBuf,
    pub timeout: Duration,
    pub docker_host: Option<String>,
}
pub(crate) struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
impl Manager {
    pub fn new(paths: Paths) -> Self {
        Self {
            paths,
            docker: "docker".into(),
            timeout: Duration::from_secs(60),
            docker_host: None,
        }
    }
    fn registry(&self) -> PathBuf {
        self.paths.config.join("databases.json")
    }
    fn load(&self) -> Result<Vec<Resource>> {
        match fs::read(self.registry()) {
            Ok(b) => Ok(serde_json::from_slice(&b)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }
    fn save(&self, rs: &[Resource]) -> Result<()> {
        let mut f = tempfile::NamedTempFile::new_in(&self.paths.config)?;
        serde_json::to_writer_pretty(&mut f, rs)?;
        f.as_file().sync_all()?;
        f.persist(self.registry())?;
        Ok(())
    }
    pub(crate) fn lock(&self) -> Result<Lock> {
        if !self.paths.state.is_dir() {
            return Err("E_INIT: execute fp db init primeiro".into());
        }
        let p = self.paths.state.join("database.lock");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)
            .map_err(
                |_| "E_BUSY: operação em andamento ou lock remanescente; inspecione database.lock",
            )?;
        writeln!(f, "{}", std::process::id())?;
        Ok(Lock(p))
    }
    fn log(&self, action: &str, id: &str) -> Result<()> {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.paths.state.join("database.log"))?;
        writeln!(f, "{action} {id}")?;
        Ok(())
    }
    pub fn init(&self, dry: bool) -> Result<Value> {
        if !dry {
            for p in [&self.paths.config, &self.paths.data, &self.paths.state] {
                fs::create_dir_all(p)?;
            }
            let _lock = self.lock()?;
            if !self.registry().exists() {
                self.save(&[])?;
            }
        }
        Ok(
            json!({"initialized":true,"dry_run":dry,"config":self.paths.config,"data":self.paths.data,"state":self.paths.state}),
        )
    }
    pub fn list(&self) -> Result<Vec<Resource>> {
        self.load()
    }
    pub fn get(&self, id: &str) -> Result<Resource> {
        self.load()?
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| "E_NOT_FOUND: recurso não registrado; use list".into())
    }
    fn docker(&self, args: &[&str]) -> Result<String> {
        self.call(
            &self.docker,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            None,
            &[],
        )
    }
    fn inspect(&self, r: &Resource) -> Result<Value> {
        let out = self.docker(&[
            "container",
            "inspect",
            r.container
                .as_deref()
                .ok_or("E_CONFIG: container ausente")?,
        ])?;
        let v: Vec<Value> = serde_json::from_str(&out)?;
        v.into_iter()
            .next()
            .ok_or_else(|| "E_DOCKER: resposta vazia".into())
    }
    fn identity(&self, r: &Resource) -> Result<Value> {
        let v = self.inspect(r)?;
        if v["Id"].as_str() != r.docker_id.as_deref() {
            return Err("E_CONFLICT: identidade do container mudou; operação recusada".into());
        }
        Ok(v)
    }
    pub fn discover(&self) -> Result<Value> {
        let out = self.docker(&[
            "ps",
            "-a",
            "--filter",
            "name=^/fiap-(oracle|mongodb)$",
            "--format",
            "{{json .}}",
        ])?;
        let values = out
            .lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<Vec<Value>, _>>()?;
        Ok(json!(values))
    }
    pub fn create(&self, mut r: Resource, existing: bool, dry: bool) -> Result<Value> {
        if r.engine != Engine::Sqlite
            && !existing
            && r.docker_host.is_none()
            && let Some(runtime) = self.installed_runtime()?
        {
            r.docker_host = Some(format!("unix://{}", runtime.socket.display()));
        }
        let scoped = self.scoped(&r);
        let this = &scoped;
        valid_name(&r.id)?;
        if this.load()?.iter().any(|x| x.id == r.id) {
            return Err("E_CONFLICT: identificador já registrado".into());
        }
        if r.engine == Engine::Sqlite {
            r.file = Some(
                r.file
                    .unwrap_or_else(|| this.paths.data.join(format!("{}.db", r.id))),
            );
            let p = r.file.as_ref().unwrap();
            if !p.is_absolute() {
                return Err("E_CONFIG: informe caminho absoluto".into());
            }
            if existing {
                if !fs::symlink_metadata(p)?.is_file() {
                    return Err("E_CONFIG: SQLite deve ser arquivo regular".into());
                }
            } else if p.exists() || fs::symlink_metadata(p).is_ok() {
                return Err("E_CONFLICT: arquivo existente; use --existing para registrar".into());
            }
        } else {
            let default_port = if r.engine == Engine::Oracle {
                1521
            } else {
                27017
            };
            r.port = Some(r.port.unwrap_or(default_port));
            r.container = Some(r.container.unwrap_or_else(|| format!("fp-{}", r.id)));
            r.volume = Some(r.volume.unwrap_or_else(|| format!("fp-{}-data", r.id)));
            r.image = Some(r.image.unwrap_or_else(|| {
                if r.engine == Engine::Oracle {
                    crate::runtime::ORACLE_IMAGE.into()
                } else {
                    crate::runtime::MONGO_IMAGE.into()
                }
            }));
            valid_name(r.container.as_ref().unwrap())?;
            valid_name(r.volume.as_ref().unwrap())?;
            if r.host != "127.0.0.1" && r.host != "::1" {
                return Err("E_CONFIG: bindings devem usar loopback".into());
            }
            if !existing {
                if let Some(key) = &r.password_env {
                    valid_env(key)?;
                    this.password(&r, false)?;
                }
                if r.engine == Engine::Mongodb {
                    if r.username.as_deref() == Some("") {
                        return Err("E_CREDENTIALS: usuário MongoDB não pode ser vazio".into());
                    }
                    if r.username.is_none() {
                        r.username = Some("fp".into());
                    }
                }
            }
        }

        if dry {
            return Ok(json!({"dry_run":true,"resource":r,"existing":existing}));
        }
        let _lock = this.lock()?;
        let mut resources = this.load()?;
        if resources.iter().any(|x| {
            x.id == r.id
                || (r.file.is_some() && x.file == r.file)
                || (r.container.is_some() && x.container == r.container)
        }) {
            return Err("E_CONFLICT: recurso já registrado".into());
        }
        if r.engine == Engine::Sqlite {
            let p = r.file.as_ref().unwrap();
            if existing {
                this.sqlite_execute(p, "PRAGMA schema_version;", true)?;
            } else {
                let _file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(p)?;
                if let Err(e) = this.sqlite_execute(p, "PRAGMA user_version=0;", false) {
                    let _ = fs::remove_file(p);
                    return Err(e);
                }
                r.owned = true;
            }
        } else {
            this.docker(&["info"])?;
            let container = r.container.as_ref().unwrap();
            let volume = r.volume.as_ref().unwrap();
            if existing {
                let v = this.inspect(&r)?;
                if v["Config"]["Image"].as_str() != r.image.as_deref()
                    || !v["Mounts"]
                        .as_array()
                        .is_some_and(|ms| ms.iter().any(|m| m["Name"].as_str() == Some(volume)))
                {
                    return Err("E_CONFLICT: imagem ou volume difere do recurso existente".into());
                }
                r.docker_id = Some(v["Id"].as_str().ok_or("E_DOCKER: ID ausente")?.into());
            } else {
                let containers = this.docker(&["ps", "-a", "--format", "{{.Names}}"])?;
                let volumes = this.docker(&["volume", "ls", "--format", "{{.Name}}"])?;
                if containers.lines().any(|n| n == container)
                    || volumes.lines().any(|n| n == volume)
                {
                    return Err("E_CONFLICT: container/volume existente; adote explicitamente com --existing".into());
                }
                this.docker(&[
                    "volume",
                    "create",
                    "--label",
                    &format!("fp.resource={}", r.id),
                    volume,
                ])?;
                let password = this.password(&r, true)?;
                let mut env = vec![];
                let mut args = vec![
                    "create".into(),
                    "--name".into(),
                    container.clone(),
                    "--label".into(),
                    format!("fp.resource={}", r.id),
                    "-p".into(),
                    format!(
                        "{}:{}:{}",
                        if r.host == "::1" { "[::1]" } else { &r.host },
                        r.port.unwrap(),
                        if r.engine == Engine::Oracle {
                            1521
                        } else {
                            27017
                        }
                    ),
                    "-v".into(),
                    format!(
                        "{volume}:{}",
                        if r.engine == Engine::Oracle {
                            "/opt/oracle/oradata"
                        } else {
                            "/data/db"
                        }
                    ),
                ];
                if r.engine == Engine::Oracle {
                    let key = if r
                        .image
                        .as_deref()
                        .is_some_and(|s| s.starts_with("gvenzl/oracle-free:"))
                    {
                        "ORACLE_PASSWORD"
                    } else {
                        "ORACLE_PWD"
                    };
                    args.extend(["-e".into(), key.into()]);
                    env.push((key.into(), password));
                    if key == "ORACLE_PASSWORD" {
                        args.extend([
                            "--health-cmd".into(),
                            "healthcheck.sh".into(),
                            "--health-interval".into(),
                            "10s".into(),
                            "--health-start-period".into(),
                            "120s".into(),
                        ]);
                    }
                } else {
                    args.extend([
                        "-e".into(),
                        "MONGO_INITDB_ROOT_PASSWORD".into(),
                        "-e".into(),
                        "MONGO_INITDB_ROOT_USERNAME".into(),
                    ]);
                    env.extend([
                        ("MONGO_INITDB_ROOT_PASSWORD".into(), password),
                        (
                            "MONGO_INITDB_ROOT_USERNAME".into(),
                            r.username.clone().unwrap(),
                        ),
                    ]);
                }
                args.push(r.image.clone().unwrap());
                r.docker_id = Some(
                    this.call(&this.docker, &args, None, &env)?
                        .trim()
                        .to_string(),
                );
                r.owned = true;
            }
        }
        if let Some(file) = &r.file {
            let meta = fs::symlink_metadata(file)?;
            r.file_identity = Some((meta.dev(), meta.ino()));
        }
        resources.push(r.clone());
        this.save(&resources)?;
        this.log("create", &r.id)?;
        Ok(json!(r))
    }
    pub fn status(&self, id: &str) -> Result<Value> {
        let r = self.get(id)?;
        let scoped = self.scoped(&r);
        let this = &scoped;
        if r.engine == Engine::Sqlite {
            return Ok(
                json!({"id":id,"exists":r.file.as_ref().is_some_and(|p|p.is_file()),"engine":"sqlite"}),
            );
        }
        let v = this.identity(&r)?;
        Ok(json!({"id":id,"running":v["State"]["Running"],"health":v["State"]["Health"]["Status"]}))
    }
    pub fn run(&self, id: &str, stop: bool, wait: bool, dry: bool) -> Result<Value> {
        let r = self.get(id)?;
        let scoped = self.scoped(&r);
        let this = &scoped;
        if dry {
            return Ok(json!({"id":id,"dry_run":true,"stop":stop}));
        }
        let _lock = this.lock()?;
        this.log("operation:start", id)?;
        if r.engine == Engine::Sqlite {
            if !r.file.as_ref().is_some_and(|p| p.is_file()) {
                return Err("E_NOT_FOUND: arquivo SQLite ausente".into());
            }
        } else {
            let v = this.identity(&r)?;
            if v["State"]["Running"].as_bool() != Some(!stop) {
                this.docker(&[
                    if stop { "stop" } else { "start" },
                    r.container.as_ref().unwrap(),
                ])?;
            }
            if wait && !stop {
                let start = Instant::now();
                loop {
                    let state = this.identity(&r)?;
                    if state["State"]["Running"] != true {
                        return Err("E_START: container parou; consulte logs".into());
                    }
                    if state["State"]["Health"]["Status"] == "healthy" {
                        break;
                    }
                    if r.engine == Engine::Mongodb
                        && self
                            .docker(&[
                                "exec",
                                r.container.as_ref().unwrap(),
                                "mongosh",
                                "--quiet",
                                "--eval",
                                "quit(db.runCommand({ping:1}).ok ? 0 : 1)",
                            ])
                            .is_ok()
                    {
                        break;
                    }
                    if start.elapsed() >= this.timeout {
                        return Err(
                            "E_TIMEOUT: banco ainda não está disponível; consulte status/logs"
                                .into(),
                        );
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
        this.log(if stop { "stop" } else { "run" }, id)?;
        let status = this.status(id)?;
        if r.engine != Engine::Sqlite && status["running"].as_bool() != Some(!stop) {
            return Err("E_START: container não atingiu o estado solicitado; consulte logs".into());
        }
        Ok(status)
    }
    pub fn logs(&self, id: &str) -> Result<Value> {
        let r = self.get(id)?;
        let scoped = self.scoped(&r);
        let this = &scoped;
        if r.engine == Engine::Sqlite {
            return Ok(
                json!({"id":id,"logs":fs::read_to_string(this.paths.state.join("database.log")).unwrap_or_default()}),
            );
        }
        this.identity(&r)?;
        Ok(
            json!({"id":id,"logs":this.docker(&["logs","--tail","100",r.container.as_ref().unwrap()])?}),
        )
    }
    fn sqlite_identity(&self, r: &Resource) -> Result<()> {
        let p = r.file.as_ref().ok_or("E_CONFIG: arquivo ausente")?;
        let meta = fs::symlink_metadata(p)?;
        if !meta.is_file() || Some((meta.dev(), meta.ino())) != r.file_identity {
            return Err("E_CONFLICT: identidade do arquivo SQLite mudou".into());
        }
        Ok(())
    }
    pub fn exec(&self, id: &str, script: &[u8], dry: bool) -> Result<Value> {
        let r = self.get(id)?;
        let scoped = self.scoped(&r);
        let this = &scoped;
        if dry {
            return Ok(json!({"id":id,"dry_run":true}));
        }
        let _lock = this.lock()?;
        this.log("exec:start", id)?;
        let output = if r.engine == Engine::Sqlite {
            this.sqlite_identity(&r)?;
            this.sqlite_execute(
                r.file.as_ref().unwrap(),
                std::str::from_utf8(script)?,
                false,
            )?
        } else {
            this.identity(&r)?;
            let mut args = vec!["exec".into(), "-i".into()];
            let mut env = vec![];
            let input = if r.engine == Engine::Oracle {
                args.extend([
                    r.container.clone().unwrap(),
                    "sqlplus".into(),
                    "-s".into(),
                    "/".into(),
                    "as".into(),
                    "sysdba".into(),
                ]);
                [b"SET ECHO OFF\nSET VERIFY OFF\nSET DEFINE OFF\nWHENEVER SQLERROR EXIT FAILURE\nWHENEVER OSERROR EXIT FAILURE\nALTER SESSION SET CONTAINER = FREEPDB1;\n".as_slice(),script,b"\nEXIT\n"].concat()
            } else {
                let password = this.password(&r, false)?;
                let user = r.username.ok_or("E_CREDENTIALS: usuário MongoDB ausente")?;
                args.extend([
                    "-e".into(),
                    "FP_DB_PASSWORD".into(),
                    r.container.clone().unwrap(),
                    "mongosh".into(),
                    "--quiet".into(),
                    "--file".into(),
                    "/dev/stdin".into(),
                ]);
                env.push(("FP_DB_PASSWORD".into(), password));
                let prefix = format!(
                    "if (!db.getSiblingDB('admin').auth({}, process.env.FP_DB_PASSWORD)) {{ quit(1); }}\n",
                    serde_json::to_string(&user)?
                );
                [prefix.as_bytes(), script].concat()
            };
            this.call(&this.docker, &args, Some(&input), &env)?
        };
        this.log("exec:ok", id)?;
        Ok(json!({"id":id,"output":output}))
    }
    /// Provisionamento solicitado: cria e concede permissões somente se o usuário não existe.
    pub fn provision_user(
        &self,
        id: &str,
        user: &str,
        password_env: &str,
        dry: bool,
    ) -> Result<Value> {
        let r = self.get(id)?;
        if r.engine != Engine::Oracle {
            return Err("E_ENGINE: provision-user requer Oracle".into());
        }
        if user.is_empty()
            || user.len() > 30
            || !user.bytes().all(|b| b.is_ascii_alphabetic() || b == b'_')
        {
            return Err("E_NAME: usuário Oracle inválido".into());
        }
        valid_env(password_env)?;
        let password =
            std::env::var(password_env).map_err(|_| "E_CREDENTIALS: variável de senha ausente")?;
        if password.is_empty() || password.chars().any(char::is_control) {
            return Err("E_CREDENTIALS: senha inválida".into());
        }
        let password = password.replace('"', "\"\"").replace('\'', "''");
        let user = user.to_ascii_uppercase();
        let sql = format!(
            "DECLARE n NUMBER; BEGIN SELECT COUNT(*) INTO n FROM dba_users WHERE username = '{user}'; IF n = 0 THEN EXECUTE IMMEDIATE 'CREATE USER {user} IDENTIFIED BY \"{password}\" DEFAULT TABLESPACE users QUOTA 100M ON users'; EXECUTE IMMEDIATE 'GRANT CREATE SESSION, CREATE TABLE TO {user}'; END IF; END;\n/\n"
        );
        self.exec(id, sql.as_bytes(), dry)
    }
    pub fn delete(
        &self,
        id: &str,
        confirm: &str,
        container: bool,
        data: bool,
        dry: bool,
    ) -> Result<Value> {
        if confirm != id {
            return Err(
                "E_CONFIRM: --confirm deve corresponder exatamente ao identificador do recurso"
                    .into(),
            );
        }
        let r = self.get(id)?;
        let scoped = self.scoped(&r);
        let this = &scoped;
        if data && !r.owned {
            return Err(
                "E_EXTERNAL: dados importados não podem ser apagados; apenas remova o registro"
                    .into(),
            );
        }
        if r.engine != Engine::Sqlite && data && !container {
            return Err("E_CONFIRM: --delete-data exige --remove-container".into());
        }
        if dry {
            return Ok(
                json!({"id":id,"dry_run":true,"remove_container":container,"delete_data":data}),
            );
        }
        let _lock = this.lock()?;
        this.log("operation:start", id)?;
        if r.engine == Engine::Sqlite && data {
            this.sqlite_identity(&r)?;
            let p = r.file.as_ref().unwrap();
            if !fs::symlink_metadata(p)?.is_file() {
                return Err("E_CONFLICT: tipo de arquivo alterado".into());
            }
            fs::remove_file(p)?;
        } else if r.engine != Engine::Sqlite && container {
            let v = this.identity(&r)?;
            if v["State"]["Running"] == true {
                return Err("E_RUNNING: pare o container antes de remover".into());
            }
            if data {
                let vol = r.volume.as_ref().unwrap();
                let v: Vec<Value> =
                    serde_json::from_str(&this.docker(&["volume", "inspect", vol])?)?;
                if v.first().and_then(|v| v["Labels"]["fp.resource"].as_str()) != Some(id) {
                    return Err("E_EXTERNAL: volume não pertence ao FP".into());
                }
            }
            this.docker(&["rm", r.container.as_ref().unwrap()])?;
            if data {
                this.docker(&["volume", "rm", r.volume.as_ref().unwrap()])?;
            }
        }
        let mut rs = this.load()?;
        rs.retain(|r| r.id != id);
        this.save(&rs)?;
        this.log("delete", id)?;
        Ok(json!({"id":id,"deleted":true,"data_preserved":!data,"container_preserved":!container}))
    }
}
pub fn valid_name(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 64
        || s.starts_with('-')
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err("E_NAME: use nome alfanumérico com _ ou - (até 64 caracteres)".into());
    }
    Ok(())
}
fn valid_env(s: &str) -> Result<()> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
        return Err("E_CREDENTIALS: nome de variável inválido".into());
    }
    Ok(())
}
