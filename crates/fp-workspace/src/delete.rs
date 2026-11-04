use crate::{Result, discovery, python_workspace, registry};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item};

/// Um plano é construído pela descoberta; não aceita caminhos arbitrários.
pub struct Plan {
    target: PathBuf,
    pub registrations: Vec<String>,
    root: PathBuf,
    before: String,
    after: DocumentMut,
    identity: crate::identity::Identity,
    python: Option<python_workspace::Change>,
}

impl Plan {
    pub fn target(&self) -> &Path {
        &self.target
    }
}

pub fn prepare(root: &Path, phase: u32, chapter: Option<u32>) -> Result<Plan> {
    let root = root.canonicalize()?;
    let phase = discovery::discover(&root, "fase-")?
        .into_iter()
        .find(|p| p.number == phase)
        .ok_or("Fase não encontrada")?;
    let target = if let Some(number) = chapter {
        discovery::discover(&phase.path, "cap-")?
            .into_iter()
            .find(|c| c.number == number)
            .ok_or("Capítulo não encontrado")?
            .path
    } else {
        phase.path
    };
    let before = fs::read_to_string(root.join("chapters.toml"))?;
    let mut after = registry::load(&root)?;
    let mut registrations = Vec::new();
    let chapters = after["chapters"].as_table().ok_or("Registro inválido")?;
    for (key, entry) in chapters {
        let path = root
            .join(entry["path"].as_str().ok_or("path inválido")?)
            .canonicalize()?;
        if path.starts_with(&target) {
            registrations.push(key.to_string());
        } else if let Some(commands) = entry.get("commands").and_then(Item::as_table) {
            for (_, command) in commands {
                let cwd = command.get("cwd").and_then(Item::as_str).unwrap_or(".");
                if path.join(cwd).canonicalize()?.starts_with(&target) {
                    return Err(format!("O alvo de {key} referencia a pasta selecionada. Ajuste esse registro antes de excluir").into());
                }
            }
        }
    }
    let chapters = after["chapters"]
        .as_table_mut()
        .ok_or("Registro inválido")?;
    for key in &registrations {
        chapters.remove(key);
    }
    let python = python_workspace::Change::remove(&root, &target)?;
    Ok(Plan {
        target: target.clone(),
        registrations,
        root,
        before,
        after,
        identity: crate::identity::identify(&target)?,
        python,
    })
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn execute(plan: &Plan, confirmed: bool) -> Result<()> {
    if !confirmed {
        return Ok(());
    }
    // Compartilhado com fp create para serializar edições de chapters.toml.
    let lock_path = plan.root.join(".fp-init.lock");
    let _file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| format!("Não foi possível obter .fp-init.lock: {e}"))?;
    let _lock = Lock(lock_path);
    drop(_file);
    if fs::read_to_string(plan.root.join("chapters.toml"))? != plan.before {
        return Err("Registro alterado desde a confirmação; execute fp delete novamente".into());
    }
    let metadata = fs::symlink_metadata(&plan.target)?;
    if !metadata.is_dir()
        || crate::identity::is_link(&metadata)
        || crate::identity::identify(&plan.target)? != plan.identity
        || plan.target.canonicalize()? != plan.target
    {
        return Err("O diretório selecionado mudou; execute fp delete novamente".into());
    }
    // keep impede que uma falha ou rollback remova dados por Drop.
    let staging = tempfile::Builder::new()
        .prefix(".fp-delete-")
        .tempdir_in(&plan.root)?
        .keep();
    let moved = staging.join("project");
    if let Err(error) = fs::rename(&plan.target, &moved) {
        let _ = fs::remove_dir(&staging);
        return Err(error.into());
    }
    finish(plan, &staging)
}

fn finish(plan: &Plan, staging: &Path) -> Result<()> {
    let moved = staging.join("project");
    let publish = (|| -> Result<()> {
        if let Some(change) = &plan.python {
            change.apply(&plan.root)?;
        }
        if let Err(error) = registry::save(&plan.root, &plan.before, &plan.after) {
            if let Some(change) = &plan.python {
                change
                    .rollback(&plan.root)
                    .map_err(|rollback| format!("{error}; {rollback}"))?;
            }
            return Err(error);
        }
        Ok(())
    })();
    if let Err(error) = publish {
        // Não sobrescrever uma pasta criada por outro processo durante a operação.
        if fs::symlink_metadata(&plan.target).is_ok() {
            return Err(format!(
                "{error}; destino ocupado no rollback. Dados preservados em {}",
                moved.display()
            )
            .into());
        }
        if let Err(rollback) = fs::rename(&moved, &plan.target) {
            return Err(format!(
                "{error}; rollback falhou: {rollback}. Dados preservados em {}",
                moved.display()
            )
            .into());
        }
        let _ = fs::remove_dir(staging);
        return Err(error);
    }
    if let Err(error) = fs::remove_dir_all(staging) {
        return Err(format!(
            "Registro atualizado, mas a limpeza falhou: {error}. Arquivos restantes em {}",
            staging.display()
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_failure_restores_staged_directory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("fase-01")).unwrap();
        fs::write(root.path().join("fase-01/original"), "keep").unwrap();
        fs::write(root.path().join("chapters.toml"), "[chapters]\n").unwrap();
        let config = "# preservar dependências\n[tool.uv.workspace]\nmembers=['fase-01']\n";
        fs::write(root.path().join("pyproject.toml"), config).unwrap();
        let plan = prepare(root.path(), 1, None).unwrap();
        let staging = tempfile::tempdir_in(root.path()).unwrap().keep();
        fs::rename(&plan.target, staging.join("project")).unwrap();
        fs::write(root.path().join("chapters.toml"), "# edit\n[chapters]\n").unwrap();
        assert!(finish(&plan, &staging).is_err());
        assert_eq!(
            fs::read_to_string(plan.target.join("original")).unwrap(),
            "keep"
        );
        assert!(!staging.exists());
        assert_eq!(
            fs::read_to_string(root.path().join("pyproject.toml")).unwrap(),
            config
        );
    }

    #[test]
    fn occupied_rollback_keeps_staged_files() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("fase-01")).unwrap();
        fs::write(root.path().join("fase-01/original"), "keep").unwrap();
        fs::write(root.path().join("chapters.toml"), "[chapters]\n").unwrap();
        let plan = prepare(root.path(), 1, None).unwrap();
        let staging = tempfile::tempdir_in(root.path()).unwrap();
        fs::rename(&plan.target, staging.path().join("project")).unwrap();
        fs::create_dir(&plan.target).unwrap();
        fs::write(plan.target.join("new"), "other process").unwrap();
        fs::write(root.path().join("chapters.toml"), "# edit\n[chapters]\n").unwrap();
        assert!(
            finish(&plan, staging.path())
                .unwrap_err()
                .to_string()
                .contains("Dados preservados")
        );
        assert_eq!(
            fs::read_to_string(staging.path().join("project/original")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(plan.target.join("new")).unwrap(),
            "other process"
        );
    }
}
