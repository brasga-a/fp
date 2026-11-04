use crate::Result;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use toml_edit::{Array, DocumentMut, value};

/// Alteração pontual no manifesto, com publicação atômica e rollback verificado.
pub(crate) struct Change {
    before: Option<String>,
    after: String,
}

fn read(root: &Path) -> Result<Option<String>> {
    let path = root.join("pyproject.toml");
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && !crate::identity::is_link(&metadata) => {
            Ok(Some(fs::read_to_string(path)?))
        }
        Ok(_) => Err("pyproject.toml da raiz deve ser um arquivo regular, sem links".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

impl Change {
    pub(crate) fn add(root: &Path, target: &Path) -> Result<Self> {
        let before = read(root)?;
        let mut doc = before
            .as_deref()
            .unwrap_or(include_str!("../../../assets/workspace/pyproject.toml"))
            .parse::<DocumentMut>()?;
        // Configurações existentes não podem ser substituídas por tabelas novas.
        for keys in [&["tool"][..], &["tool", "uv"], &["tool", "uv", "workspace"]] {
            let mut item = doc.as_item();
            for key in keys {
                let Some(next) = item.get(key) else { break };
                if !next.is_table_like() {
                    return Err(format!("{} deve ser uma tabela", keys.join(".")).into());
                }
                item = next;
            }
        }
        if let Some(exclude) = doc
            .get("tool")
            .and_then(|t| t.get("uv"))
            .and_then(|u| u.get("workspace"))
            .and_then(|w| w.get("exclude"))
            && exclude.as_array().is_none_or(|a| !a.is_empty())
        {
            return Err("Workspace uv possui exclusões personalizadas; revise tool.uv.workspace.exclude antes de adicionar capítulos pelo FP".into());
        }
        let relative = target
            .strip_prefix(root)?
            .to_str()
            .ok_or("Caminho não UTF-8")?
            .replace('\\', "/");
        let members = &mut doc["tool"]["uv"]["workspace"]["members"];
        if members.is_none() {
            *members = value(Array::new());
        }
        let members = members
            .as_array_mut()
            .ok_or("tool.uv.workspace.members deve ser uma lista")?;
        if members.iter().any(|v| v.as_str().is_none()) {
            return Err("tool.uv.workspace.members deve conter somente caminhos".into());
        }
        if !members
            .iter()
            .any(|v| v.as_str() == Some(relative.as_str()))
        {
            members.push(relative.as_str());
        }
        configure_analysis(&mut doc, ".");
        Ok(Self {
            before,
            after: doc.to_string(),
        })
    }

    pub(crate) fn remove(root: &Path, target: &Path) -> Result<Option<Self>> {
        let Some(before) = read(root)? else {
            return Ok(None);
        };
        let mut doc = before.parse::<DocumentMut>()?;
        let Some(members) = doc
            .get_mut("tool")
            .and_then(|t| t.get_mut("uv"))
            .and_then(|u| u.get_mut("workspace"))
            .and_then(|w| w.get_mut("members"))
        else {
            return Ok(None);
        };
        let members = members
            .as_array_mut()
            .ok_or("tool.uv.workspace.members deve ser uma lista")?;
        let relative = target.strip_prefix(root)?;
        let mut changed = false;
        for index in (0..members.len()).rev() {
            let member = members
                .get(index)
                .and_then(|v| v.as_str())
                .ok_or("tool.uv.workspace.members deve conter somente caminhos")?;
            if Path::new(member).starts_with(relative) {
                members.remove(index);
                changed = true;
            }
        }
        Ok(changed.then(|| Self {
            before: Some(before),
            after: doc.to_string(),
        }))
    }

    pub(crate) fn apply(&self, root: &Path) -> Result<()> {
        if read(root)? != self.before {
            return Err("pyproject.toml alterado por outro processo; tente novamente".into());
        }
        self.write(root, &self.after, self.before.is_none())
    }

    pub(crate) fn rollback(&self, root: &Path) -> Result<()> {
        if read(root)?.as_deref() != Some(&self.after) {
            return Err("pyproject.toml alterado durante rollback; revise o workspace uv".into());
        }
        if let Some(before) = &self.before {
            self.write(root, before, false)
        } else {
            fs::remove_file(root.join("pyproject.toml"))?;
            Ok(())
        }
    }

    fn write(&self, root: &Path, contents: &str, new: bool) -> Result<()> {
        let path = root.join("pyproject.toml");
        let mut temp = tempfile::NamedTempFile::new_in(root)?;
        if !new {
            temp.as_file()
                .set_permissions(fs::metadata(&path)?.permissions())?;
        }
        temp.write_all(contents.as_bytes())?;
        temp.as_file().sync_all()?;
        if new {
            temp.persist_noclobber(path)?;
        } else {
            temp.persist(path)?;
        }
        Ok(())
    }
}

/// Os nomes dos diretórios se repetem entre fases; nomes uv precisam ser únicos.
pub(crate) fn name(target: &Path) -> Result<String> {
    let chapter = target
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Capítulo inválido")?;
    let phase = target
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .ok_or("Fase inválida")?;
    Ok(format!("{phase}-{chapter}"))
}

fn configure_analysis(doc: &mut DocumentMut, venv_path: &str) {
    // Uma configuração personalizada do analisador pertence ao usuário.
    if doc.get("tool").and_then(|t| t.get("pyright")).is_none() {
        doc["tool"]["pyright"]["venvPath"] = value(venv_path);
        doc["tool"]["pyright"]["venv"] = value(".venv");
    }
}

pub(crate) fn configure(root: &Path, project: &Path, target: &Path) -> Result<()> {
    let path = project.join("pyproject.toml");
    let mut doc = fs::read_to_string(&path)?.parse::<DocumentMut>()?;
    doc["project"]["name"] = value(name(target)?);
    doc["tool"]["uv"]["package"] = value(false);
    let venv_path = target
        .strip_prefix(root)?
        .components()
        .map(|_| "..")
        .collect::<PathBuf>();
    configure_analysis(&mut doc, venv_path.to_str().ok_or("Caminho não UTF-8")?);
    fs::write(path, doc.to_string())?;
    Ok(())
}
