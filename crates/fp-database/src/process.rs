use crate::{Manager, Result};
use std::{
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
impl Manager {
    pub(crate) fn call(
        &self,
        tool: &Path,
        args: &[String],
        input: Option<&[u8]>,
        env: &[(String, String)],
    ) -> Result<String> {
        #[cfg(windows)]
        if self
            .docker_host
            .as_ref()
            .is_some_and(|h| h.starts_with("unix://"))
        {
            return Err("E_PLATFORM: endpoint Docker unix:// de Linux incompatível com Windows; utilize Docker Desktop e um registro próprio, sem migrar volumes automaticamente".into());
        }
        #[cfg(windows)]
        let program = program_in_path(tool)?;
        #[cfg(unix)]
        let program = tool;
        let mut cmd = Command::new(program);
        cmd.args(args)
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if tool
            .file_name()
            .is_some_and(|n| n == "docker" || n == "docker.exe")
        {
            if let Some(host) = &self.docker_host {
                cmd.env_remove("DOCKER_CONTEXT")
                    .env_remove("DOCKER_TLS_VERIFY")
                    .env_remove("DOCKER_CERT_PATH")
                    .env("DOCKER_HOST", host)
                    .env("DOCKER_CONFIG", self.paths.config.join("docker-client"));
            } else if args.first().is_some_and(|s| s == "--host") {
                cmd.env_remove("DOCKER_CONTEXT")
                    .env_remove("DOCKER_HOST")
                    .env_remove("DOCKER_TLS_VERIFY")
                    .env_remove("DOCKER_CERT_PATH");
            }
        }
        // Um executável recém-escrito pode estar temporariamente ocupado no Linux.
        // Só repetimos spawn recusado: nenhum comando chegou a ser executado.
        let mut retries = 0;
        let mut child = loop {
            match cmd.spawn() {
                Ok(child) => break child,
                Err(error)
                    if error.kind() == std::io::ErrorKind::ExecutableFileBusy && retries < 5 =>
                {
                    retries += 1;
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => {
                    return Err(format!(
                        "E_TOOL: {} indisponível ({error}); verifique instalação e permissões",
                        tool.display()
                    )
                    .into());
                }
            }
        };
        let mut stdin = child.stdin.take().unwrap();
        let data = input.unwrap_or_default().to_vec();
        let writer = thread::spawn(move || {
            let _ = stdin.write_all(&data);
        });
        let mut stdout = child.stdout.take().unwrap();
        let reader = thread::spawn(move || {
            let mut b = String::new();
            let _ = stdout.read_to_string(&mut b);
            b
        });
        let mut stderr = child.stderr.take().unwrap();
        let err_reader = thread::spawn(move || {
            let mut b = String::new();
            let _ = stderr.read_to_string(&mut b);
            b
        });
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                let _ = writer.join();
                let mut out = reader.join().unwrap_or_default();
                let err = err_reader.join().unwrap_or_default();
                if args.first().is_some_and(|s| s == "logs") {
                    out.push_str(&err);
                }
                if !status.success() {
                    return Err(format!(
                        "E_COMMAND: {} falhou (código {:?}); consulte status/logs",
                        tool.display(),
                        status.code()
                    )
                    .into());
                }
                return Ok(out);
            }
            if start.elapsed() > self.timeout {
                let _ = child.kill();
                let _ = child.wait();
                return Err(
                    "E_TIMEOUT: operação excedeu o prazo; consulte status/logs antes de repetir"
                        .into(),
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

#[cfg(windows)]
fn program_in_path(tool: &Path) -> Result<std::path::PathBuf> {
    // Command no Windows também pesquisa diretórios do sistema antes do PATH.
    // Resolva o executável explicitamente para respeitar a escolha do usuário
    // e o isolamento dos executáveis simulados nos testes.
    if tool.is_absolute() || tool.components().count() > 1 {
        return Ok(std::path::absolute(tool)?);
    }
    let name = if tool.extension().is_some() {
        tool.to_path_buf()
    } else {
        tool.with_extension("exe")
    };
    let found = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "E_TOOL: {} ausente no PATH; configure o Docker CLI para continuar",
                tool.display()
            )
        })?;
    Ok(std::path::absolute(found)?)
}
