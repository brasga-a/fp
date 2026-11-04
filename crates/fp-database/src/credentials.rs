//! Credenciais independentes do bootstrap Docker. Nenhum segredo no registro/argv.
use crate::{Manager, Result};
use std::{fs, path::Path};
impl Manager {
    pub fn generated_password(&self, id: &str, create: bool) -> Result<String> {
        crate::valid_name(id)?;
        let dir = self.paths.config.join("secrets");
        secure::directory(&dir, create)?;
        let path = dir.join(id);
        if create
            && fs::symlink_metadata(&path).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            if self.list()?.iter().any(|r| r.id == id) {
                return Err("E_CREDENTIALS: segredo local ausente; restaure o backup, sem redefinir senhas do banco".into());
            }
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes)
                .map_err(|e| format!("E_CREDENTIALS: fonte aleatória indisponível: {e}"))?;
            let password = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            secure::write(&path, id, password.as_bytes())?;
        }
        let password = secure::read(&path, id, &self.paths.config)?;
        if password.len() != 64 || !password.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("E_CREDENTIALS: segredo local inválido".into());
        }
        Ok(password)
    }
    pub(crate) fn password(&self, r: &crate::Resource, create: bool) -> Result<String> {
        if let Some(key) = &r.password_env {
            let p = std::env::var(key).map_err(|_| "E_CREDENTIALS: variável de senha ausente")?;
            if p.is_empty() {
                return Err("E_CREDENTIALS: senha vazia".into());
            }
            Ok(p)
        } else if create || r.owned {
            self.generated_password(&r.id, create)
        } else {
            Err("E_CREDENTIALS: recurso importado exige a variável da senha existente".into())
        }
    }
}

#[cfg(unix)]
mod secure {
    use super::*;
    use std::{
        io::{Read, Write},
        os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    };
    pub fn directory(dir: &Path, create: bool) -> Result<()> {
        if create && !dir.try_exists()? {
            fs::DirBuilder::new().mode(0o700).create(dir)?;
        }
        let meta = fs::symlink_metadata(dir)?;
        if !meta.is_dir() || meta.mode() & 0o077 != 0 {
            return Err(
                "E_CREDENTIALS: diretório de segredos deve ser privado e não pode ser symlink"
                    .into(),
            );
        }
        Ok(())
    }
    pub fn write(path: &Path, _: &str, password: &[u8]) -> Result<()> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(password)?;
        f.sync_all()?;
        Ok(())
    }
    pub fn read(path: &Path, _: &str, config: &Path) -> Result<String> {
        let meta=fs::symlink_metadata(path).map_err(|_|"E_CREDENTIALS: segredo local ausente; restaure o backup, sem redefinir senhas do banco")?;
        if !meta.is_file()
            || meta.mode() & 0o077 != 0
            || meta.nlink() != 1
            || meta.uid() != fs::metadata(config)?.uid()
        {
            return Err("E_CREDENTIALS: arquivo de senha deve ser privado (0600), regular e pertencente ao usuário".into());
        }
        let mut file = fs::File::open(path)?;
        let opened = file.metadata()?;
        if (opened.dev(), opened.ino()) != (meta.dev(), meta.ino()) {
            return Err("E_CREDENTIALS: segredo alterado durante leitura".into());
        }
        let mut password = String::new();
        file.read_to_string(&mut password)?;
        Ok(password)
    }
}

#[cfg(windows)]
#[path = "credentials/windows.rs"]
mod secure;
