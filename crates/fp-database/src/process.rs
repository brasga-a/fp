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
        let mut cmd = Command::new(tool);
        cmd.args(args)
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if tool.file_name().is_some_and(|n| n == "docker") {
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
