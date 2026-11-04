use crate::{Result, processes, python_workspace, registry};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Technology {
    Python,
    Web,
    Rust,
    Iot,
    Sql,
    Folder,
}
impl Technology {
    pub fn label(self) -> &'static str {
        match self {
            Self::Python => "Python (uv)",
            Self::Web => "TypeScript / JavaScript (Vite+)",
            Self::Rust => "Rust (Cargo)",
            Self::Iot => "ESP32 / IoT (PlatformIO)",
            Self::Sql => "SQL / Database",
            Self::Folder => "Somente pasta",
        }
    }
    pub fn tool(self) -> Option<&'static str> {
        match self {
            Self::Python => Some("uv"),
            Self::Web => Some("vp"),
            Self::Rust => Some("cargo"),
            Self::Iot => Some("pio"),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub struct Plan {
    pub target: PathBuf,
    pub chapter: Option<(u32, u32)>,
    pub technology: Technology,
    pub tool: Option<PathBuf>,
    pub board: Option<String>,
    pub framework: Option<String>,
}

impl Plan {
    /// O nome visível do capítulo é o nome do diretório gerado pelo FP.
    fn name(&self) -> Result<&str> {
        self.target
            .file_name()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Nome de diretório inválido".into())
    }
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn create(root: &Path, plan: &Plan, confirmed: bool) -> Result<Option<Vec<String>>> {
    if !confirmed {
        return Ok(None);
    }
    let name = plan.name()?;
    let parent = plan
        .target
        .parent()
        .ok_or("Destino inválido")?
        .canonicalize()?;
    if !parent.starts_with(root.canonicalize()?) {
        return Err("Destino fora da raiz".into());
    }
    if fs::symlink_metadata(&plan.target).is_ok() {
        return Err("Diretório de destino já existe".into());
    }
    let lock_path = root.join(".fp-init.lock");
    let _file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| format!("Não foi possível obter .fp-init.lock: {e}"))?;
    let _lock = Lock(lock_path);
    drop(_file);
    let python = if plan.chapter.is_some() && plan.technology == Technology::Python {
        Some(python_workspace::Change::add(root, &plan.target)?)
    } else {
        None
    };
    let before = fs::read_to_string(root.join("chapters.toml"))?;
    let mut doc = registry::load(root)?;
    if let Some((phase, chapter)) = plan.chapter
        && doc["chapters"].get(format!("{phase}.{chapter}")).is_some()
    {
        return Err("Número já registrado".into());
    }
    // Fora da árvore acadêmica: inicializadores não herdam workspaces de pais.
    // Publicação copia quando o diretório temporário está em outro filesystem.
    let stage = tempfile::Builder::new().prefix("fp-stage-").tempdir()?;
    let project = stage.path().join("project");
    fs::create_dir(&project)?;
    if plan.chapter.is_some() {
        scaffold(root, plan, &project)?;
    }
    let command = if plan.chapter.is_some() {
        entrypoint(plan, &project)?
    } else {
        None
    };
    if python.is_some() {
        python_workspace::configure(root, &project, &plan.target)?;
    }
    if let Some((phase, chapter)) = plan.chapter {
        let relative = plan
            .target
            .strip_prefix(root)?
            .to_str()
            .ok_or("Caminho não UTF-8")?
            .replace('\\', "/");
        registry::add(
            &mut doc,
            &format!("{phase}.{chapter}"),
            &relative,
            name,
            command.as_deref(),
        )?;
    }
    validate_tree(&project, &project)?;
    // Reserva exclusiva: rename de uma pasta sobre um destino vazio sobrescreveria terceiros.
    fs::create_dir(&plan.target)?;
    let mut published = Vec::new();
    let publish = (|| -> Result<()> {
        for item in fs::read_dir(&project)? {
            let item = item?;
            publish_item(
                &item.path(),
                &plan.target.join(item.file_name()),
                &mut published,
            )?;
        }
        if plan.chapter.is_some() {
            if let Some(change) = &python {
                change.apply(root)?;
            }
            if let Err(error) = registry::save(root, &before, &doc) {
                if let Some(change) = &python {
                    change
                        .rollback(root)
                        .map_err(|rollback| format!("{error}; {rollback}"))?;
                }
                return Err(error);
            }
        }
        Ok(())
    })();
    if let Err(error) = publish {
        // Rollback só remove entradas criadas por esta chamada cuja identidade
        // permanece igual. Arquivos concorrentes/desconhecidos ficam preservados.
        if let Err(cleanup) = rollback(&plan.target, &published) {
            return Err(format!(
                "{error}; rollback incompleto: {cleanup}; arquivos preservados em {}",
                plan.target.display()
            )
            .into());
        }
        return Err(error);
    }
    Ok(command)
}

fn scaffold(root: &Path, plan: &Plan, path: &Path) -> Result<()> {
    let python_name;
    let name = if plan.technology == Technology::Python {
        python_name = python_workspace::name(&plan.target)?;
        python_name.as_str()
    } else {
        plan.name()?
    };
    let mut args: Vec<String> = match plan.technology {
        Technology::Python => vec![
            "init",
            "--app",
            "--no-package",
            "--no-workspace",
            "--no-pin-python",
            "--no-python-downloads",
            "--author-from",
            "none",
            "--vcs",
            "none",
            "--name",
            name,
            ".",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        Technology::Rust => vec!["init", "--bin", "--vcs", "none", "--name", name, "."]
            .into_iter()
            .map(String::from)
            .collect(),
        Technology::Web => vec!["create", "vite:application", "--directory", ".", "--no-git"]
            .into_iter()
            .map(String::from)
            .collect(),
        Technology::Iot => {
            let board = plan
                .board
                .as_deref()
                .filter(|s| !s.is_empty() && !s.starts_with('-'))
                .ok_or("Informe um board ID válido do PlatformIO")?;
            let mut args = vec![
                "project".into(),
                "init".into(),
                "--board".into(),
                board.into(),
            ];
            if let Some(framework) = &plan.framework {
                args.extend(["--project-option".into(), format!("framework={framework}")]);
            }
            args
        }
        Technology::Sql => {
            fs::create_dir(path.join("sql"))?;
            return Ok(());
        }
        Technology::Folder => return Ok(()),
    };
    if plan.technology == Technology::Python && root.join("pyproject.toml").is_file() {
        let config =
            fs::read_to_string(root.join("pyproject.toml"))?.parse::<toml_edit::DocumentMut>()?;
        if let Some(version) = config
            .get("project")
            .and_then(|p| p.get("requires-python"))
            .and_then(|s| s.as_str())
        {
            args.extend(["--python".into(), version.into()]);
        }
    }
    processes::initialize(
        plan.tool.as_deref().ok_or("Inicializador ausente")?,
        &args,
        path,
    )
}

fn entrypoint(plan: &Plan, path: &Path) -> Result<Option<Vec<String>>> {
    Ok(match plan.technology {
        Technology::Rust
            if path.join("src/main.rs").is_file() && path.join("Cargo.toml").is_file() =>
        {
            Some(vec!["cargo".into(), "run".into(), "--".into()])
        }
        Technology::Python => {
            if !path.join("main.py").is_file() {
                return Err("uv init --app não criou main.py; criação cancelada para evitar capítulo sem comando".into());
            }
            Some(vec![
                "uv".into(),
                "run".into(),
                "--all-packages".into(),
                "main.py".into(),
            ])
        }
        Technology::Web if path.join("package.json").is_file() => {
            let data: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(path.join("package.json"))?)?;
            data.get("scripts")
                .and_then(|s| s.get("dev"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(|_| vec!["vp".into(), "run".into(), "dev".into(), "--".into()])
        }
        _ => None,
    })
}

// Valida antes de publicar. Links relativos internos são preservados, sem ler
// conteúdo por meio deles; links absolutos/externos e tipos especiais são recusados.
fn validate_tree(path: &Path, root: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if crate::identity::is_link(&meta) {
        if !meta.is_symlink()
            || fs::read_link(path)?.is_absolute()
            || !path.canonicalize()?.starts_with(root.canonicalize()?)
        {
            return Err(
                "Inicializador criou link externo ou reparse point; publicação recusada".into(),
            );
        }
    } else if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            validate_tree(&entry?.path(), root)?;
        }
    } else if !meta.is_file() {
        return Err("Inicializador criou arquivo especial; publicação recusada".into());
    }
    Ok(())
}

fn publish_item(
    source: &Path,
    target: &Path,
    published: &mut Vec<(PathBuf, crate::identity::Identity)>,
) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.is_symlink() {
        copy_link(source, target)?;
        published.push((target.into(), crate::identity::identify(target)?));
    } else if metadata.is_dir() {
        fs::create_dir(target)?;
        published.push((target.into(), crate::identity::identify(target)?));
        for item in fs::read_dir(source)? {
            let item = item?;
            publish_item(&item.path(), &target.join(item.file_name()), published)?;
        }
        fs::set_permissions(target, metadata.permissions())?;
    } else if metadata.is_file() {
        // create_new é exclusivo também entre filesystems. fs::copy/rename
        // poderiam sobrescrever um arquivo criado concorrentemente.
        let mut input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        published.push((target.into(), crate::identity::identify(target)?));
        std::io::copy(&mut input, &mut output)?;
        output.set_permissions(metadata.permissions())?;
    } else {
        return Err("Tipo de arquivo não suportado na publicação".into());
    }
    Ok(())
}
#[cfg(unix)]
fn copy_link(source: &Path, target: &Path) -> Result<()> {
    Ok(std::os::unix::fs::symlink(fs::read_link(source)?, target)?)
}
#[cfg(windows)]
fn copy_link(source: &Path, target: &Path) -> Result<()> {
    use std::os::windows::fs::{FileTypeExt, symlink_dir, symlink_file};
    let link = fs::read_link(source)?;
    let result = if fs::symlink_metadata(source)?.file_type().is_symlink_dir() {
        symlink_dir(link, target)
    } else {
        symlink_file(link, target)
    };
    result.map_err(|e| format!("Não foi possível preservar symlink: {e}; habilite Developer Mode ou use um inicializador sem links").into())
}

fn rollback(target: &Path, published: &[(PathBuf, crate::identity::Identity)]) -> Result<()> {
    for (path, identity) in published.iter().rev() {
        if crate::identity::identify(path)? != *identity {
            return Err("Entrada alterada durante publicação".into());
        }
        let meta = fs::symlink_metadata(path)?;
        if meta.is_dir() && !crate::identity::is_link(&meta) {
            fs::remove_dir(path)?;
        } else {
            remove_leaf(path, &meta)?;
        }
    }
    fs::remove_dir(target)?;
    Ok(())
}
fn remove_leaf(path: &Path, meta: &fs::Metadata) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if meta.file_type().is_symlink_dir() {
            return Ok(fs::remove_dir(path)?);
        }
    }
    let _ = meta;
    Ok(fs::remove_file(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_is_exclusive_and_rollback_preserves_unknown_entries() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("one"), "new").unwrap();
        fs::write(target.join("one"), "existing").unwrap();
        let mut published = vec![];
        assert!(publish_item(&source.join("one"), &target.join("one"), &mut published).is_err());
        assert_eq!(fs::read_to_string(target.join("one")).unwrap(), "existing");
        fs::remove_file(target.join("one")).unwrap();
        publish_item(&source.join("one"), &target.join("one"), &mut published).unwrap();
        fs::write(target.join("other"), "concurrent").unwrap();
        assert!(rollback(&target, &published).is_err());
        assert!(!target.join("one").exists());
        assert_eq!(
            fs::read_to_string(target.join("other")).unwrap(),
            "concurrent"
        );
    }
    #[test]
    #[cfg(unix)]
    fn external_links_are_rejected_and_relative_internal_links_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("data"), "original").unwrap();
        std::os::unix::fs::symlink("data", source.join("inside")).unwrap();
        validate_tree(&source, &source).unwrap();
        let mut published = vec![];
        publish_item(
            &source.join("inside"),
            &target.join("inside"),
            &mut published,
        )
        .unwrap();
        assert_eq!(
            fs::read_link(target.join("inside")).unwrap(),
            Path::new("data")
        );
        rollback(&target, &published).unwrap();
        std::os::unix::fs::symlink("../outside", source.join("outside")).unwrap();
        fs::write(temp.path().join("outside"), "keep").unwrap();
        assert!(validate_tree(&source, &source).is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join("outside")).unwrap(),
            "keep"
        );
    }
}
