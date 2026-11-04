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
            .ok_or("Caminho não UTF-8")?;
        registry::add(
            &mut doc,
            &format!("{phase}.{chapter}"),
            relative,
            name,
            command.as_deref(),
        )?;
    }
    // Reserva exclusiva: rename de uma pasta sobre um destino vazio sobrescreveria terceiros.
    fs::create_dir(&plan.target)?;
    let publish = (|| -> Result<()> {
        for item in fs::read_dir(&project)? {
            let item = item?;
            publish_item(&item.path(), &plan.target.join(item.file_name()))?;
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
        // Só remove o destino que esta chamada acabou de reservar.
        if let Err(cleanup) = fs::remove_dir_all(&plan.target) {
            return Err(format!(
                "{error}; limpeza falhou em {}: {cleanup}",
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

fn publish_item(source: &Path, target: &Path) -> Result<()> {
    match fs::rename(source, target) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {}
        Err(e) => return Err(e.into()),
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(source)?, target)?;
    } else if metadata.is_dir() {
        fs::create_dir(target)?;
        for item in fs::read_dir(source)? {
            let item = item?;
            publish_item(&item.path(), &target.join(item.file_name()))?;
        }
        fs::set_permissions(target, metadata.permissions())?;
    } else {
        fs::copy(source, target)?;
    }
    Ok(())
}
