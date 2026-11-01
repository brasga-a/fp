use crate::Result;
use std::{fs, io::Write, path::Path};
use toml_edit::{Array, DocumentMut, Item, Table, value};

pub fn load(root: &Path) -> Result<DocumentMut> {
    let doc = fs::read_to_string(root.join("chapters.toml"))?.parse::<DocumentMut>()?;
    let chapters = doc
        .get("chapters")
        .and_then(Item::as_table)
        .ok_or("Registro deve conter [chapters]")?;
    for (key, entry) in chapters {
        let parts: Vec<_> = key.split('.').collect();
        if parts.len() != 2
            || parts
                .iter()
                .any(|s| s.parse::<u32>().is_err() || s.starts_with('0'))
        {
            return Err(format!("Identificador inválido no registro: {key}").into());
        }
        let relative = entry
            .get("path")
            .and_then(Item::as_str)
            .ok_or("Capítulo sem path")?;
        let path = root.join(relative).canonicalize()?;
        if !path.starts_with(root.canonicalize()?) || !path.is_dir() {
            return Err("Capítulo fora da raiz".into());
        }
        if let Some(commands) = entry.get("commands") {
            let commands = commands.as_table().ok_or("commands deve ser tabela")?;
            for (_, command) in commands {
                let argv = command
                    .get("argv")
                    .and_then(Item::as_array)
                    .ok_or("Comando sem argv")?;
                if argv.is_empty()
                    || argv
                        .iter()
                        .any(|v| v.as_str().is_none_or(|s| s.contains('\0')))
                    || argv.get(0).and_then(|v| v.as_str()) == Some("")
                {
                    return Err("argv inválido".into());
                }
                let cwd = command.get("cwd").and_then(Item::as_str).unwrap_or(".");
                let cwd = path.join(cwd).canonicalize()?;
                if !cwd.is_dir() || !cwd.starts_with(&path) {
                    return Err("cwd fora do capítulo".into());
                }
            }
            if let Some(default) = entry.get("default")
                && !default.as_str().is_some_and(|s| commands.contains_key(s))
            {
                return Err("default inválido".into());
            }
        } else if entry.get("default").is_some() {
            return Err("default sem commands".into());
        }
    }
    Ok(doc)
}

pub fn executable(doc: &DocumentMut, key: &str) -> bool {
    doc.get("chapters")
        .and_then(|c| c.get(key))
        .and_then(|e| e.get("commands"))
        .and_then(Item::as_table)
        .is_some_and(|t| !t.is_empty())
}

pub fn add(
    doc: &mut DocumentMut,
    key: &str,
    relative: &str,
    name: &str,
    command: Option<&[String]>,
) -> Result<()> {
    let chapters = doc["chapters"].as_table_mut().ok_or("Registro inválido")?;
    if chapters.contains_key(key) {
        return Err(format!("Capítulo {key} já registrado").into());
    }
    let mut entry = Table::new();
    entry["path"] = value(relative);
    entry["name"] = value(name);
    if let Some(command) = command {
        entry["default"] = value("main");
        let mut argv = Array::new();
        for arg in command {
            argv.push(arg.as_str());
        }
        let mut main = Table::new();
        main["argv"] = value(argv);
        let mut commands = Table::new();
        commands["main"] = Item::Table(main);
        entry["commands"] = Item::Table(commands);
    }
    chapters[key] = Item::Table(entry);
    Ok(())
}

pub fn save(root: &Path, before: &str, doc: &DocumentMut) -> Result<()> {
    let path = root.join("chapters.toml");
    if fs::read_to_string(&path)? != before {
        return Err("Registro alterado por outro processo; tente novamente".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(root)?;
    temp.as_file()
        .set_permissions(fs::metadata(&path)?.permissions())?;
    temp.write_all(doc.to_string().as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}

/// Procura manifests também no cwd explícito de cada alvo do dispatcher.
pub fn technology(root: &Path, entry: Option<&Item>, path: &Path) -> &'static str {
    let detected = crate::discovery::technology(path);
    if detected != "Pasta / não identificada" {
        return detected;
    }
    let mut inferred = detected;
    if let Some(commands) = entry
        .and_then(|e| e.get("commands"))
        .and_then(Item::as_table)
    {
        for (_, command) in commands {
            let cwd = command.get("cwd").and_then(Item::as_str).unwrap_or(".");
            let base = entry
                .and_then(|e| e.get("path"))
                .and_then(Item::as_str)
                .map(|p| root.join(p))
                .unwrap_or_else(|| path.to_path_buf());
            let detected = crate::discovery::technology(&base.join(cwd));
            if detected != "Pasta / não identificada" {
                return detected;
            }
            if let Some(tool) = command
                .get("argv")
                .and_then(Item::as_array)
                .and_then(|argv| argv.get(0))
                .and_then(|v| v.as_str())
                .and_then(|s| Path::new(s).file_name())
                .and_then(|s| s.to_str())
            {
                inferred = match tool {
                    "uv" | "python" | "python3" => "Python",
                    "cargo" | "rustc" => "Rust",
                    "vp" | "node" | "bun" | "npm" | "pnpm" | "yarn" => "TypeScript / JavaScript",
                    "pio" | "platformio" => "ESP32 / IoT",
                    "sqlite3" | "psql" | "mysql" => "SQL / Database",
                    _ => inferred,
                };
            }
        }
    }
    inferred
}
