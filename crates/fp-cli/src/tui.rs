use crate::{
    Result,
    create::{self, Plan, Technology},
    discovery, processes, registry, technology_menu,
};
use std::{
    io::{self, IsTerminal},
    path::Path,
};

fn number(prompt: &str, suggested: u32, entries: &[discovery::Entry]) -> Result<u32> {
    let entries = entries.to_vec();
    let answer: String = cliclack::input(prompt)
        .default_input(&suggested.to_string())
        .validate(move |s: &String| match s.parse::<u32>() {
            Ok(n) if n > 0 && !entries.iter().any(|e| e.number == n) => Ok(()),
            _ => Err("Informe um inteiro positivo ainda disponível"),
        })
        .interact()?;
    Ok(answer.parse()?)
}

pub fn run(root: &Path) -> Result<()> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err("fp create requer um terminal interativo (TUI inline)".into());
    }
    cliclack::intro("FP · FIAP Projects")?;
    let chapter = cliclack::select("O que deseja criar?")
        .item(false, "Fase", "Somente diretório")
        .item(true, "Capítulo", "Projeto dentro de uma fase")
        .interact()?;
    let phases = discovery::discover(root, "fase-")?;
    let (parent, phase_number, entries, prefix) = if chapter {
        if phases.is_empty() {
            return Err(
                "Nenhuma fase disponível. Execute fp create para criar uma Fase primeiro".into(),
            );
        }
        let mut select = cliclack::select("Em qual fase?");
        for phase in &phases {
            select = select.item(phase.number, &phase.label, "");
        }
        let chosen = select.interact()?;
        let phase = phases
            .iter()
            .find(|p| p.number == chosen)
            .ok_or("Fase inválida")?;
        (
            phase.path.clone(),
            chosen,
            discovery::discover(&phase.path, "cap-")?,
            "cap-",
        )
    } else {
        (root.to_path_buf(), 0, phases, "fase-")
    };
    let n = number(
        if chapter {
            "Número do capítulo"
        } else {
            "Número da fase"
        },
        discovery::next(&entries)?,
        &entries,
    )?;
    let name = discovery::dirname(prefix, n, &entries);
    let target = parent.join(&name);
    if target.exists() {
        return Err("Diretório já existe".into());
    }
    let (technology, tool) = if chapter {
        let choice = technology_menu::select()?;
        (choice.technology, choice.tool)
    } else {
        (Technology::Folder, None)
    };
    let mut board = None;
    let mut framework = None;
    if technology == Technology::Iot {
        board = Some(
            cliclack::input("Qual é o board ID do PlatformIO? (consulte pio boards)")
                .validate(|s: &String| {
                    if s.is_empty() || s.starts_with('-') || s.chars().any(char::is_whitespace) {
                        Err("Informe um board ID válido")
                    } else {
                        Ok(())
                    }
                })
                .interact()?,
        );
        let answer: String = cliclack::input("Framework (vazio para não configurar)")
            .required(false)
            .interact()?;
        if !answer.trim().is_empty() {
            framework = Some(answer.trim().to_string());
        }
    }
    if matches!(technology, Technology::Web | Technology::Iot) {
        cliclack::log::warning(
            "O inicializador oficial pode baixar pacotes e executar código desses pacotes. Seus prompts e escolhas serão preservados.",
        )?;
    }
    let phase = if chapter {
        format!("Fase: {phase_number}\n")
    } else {
        String::new()
    };
    cliclack::note(
        "Resumo",
        format!(
            "Nome: {name}\nTipo: {}\n{phase}Tecnologia: {}\nDiretório: {}",
            if chapter { "Capítulo" } else { "Fase" },
            technology.label(),
            target.display()
        ),
    )?;
    if !cliclack::confirm("Deseja continuar?")
        .initial_value(false)
        .interact()?
    {
        cliclack::outro_cancel("Criação cancelada; nenhum arquivo alterado")?;
        return Ok(());
    }
    let plan = Plan {
        target: target.clone(),
        chapter: chapter.then_some((phase_number, n)),
        technology,
        tool,
        board,
        framework,
    };
    let spinner = cliclack::spinner();
    if plan.tool.is_none() {
        spinner.start("Criando estrutura e registrando capítulo");
    } else {
        cliclack::log::step("Executando inicializador oficial; siga os prompts abaixo")?;
    }
    let command = match create::create(root, &plan, true) {
        Ok(command) => {
            if plan.tool.is_none() {
                spinner.stop("Estrutura pronta");
            }
            command
        }
        Err(error) => {
            if plan.tool.is_none() {
                spinner.error("Falha ao criar estrutura");
            }
            cliclack::log::error(error.to_string())?;
            return Err(error);
        }
    };
    cliclack::log::success("Estrutura criada com sucesso")?;
    let open = cliclack::select("Deseja abrir o projeto no Zed?")
        .item(true, "Sim", "Abrir pasta criada")
        .item(false, "Não", "Continuar no terminal")
        .initial_value(false)
        .interact()?;
    if open && let Err(error) = processes::open_zed(&target) {
        cliclack::log::warning(error.to_string())?;
    }
    let run = if command.is_some() {
        format!(
            "\nExecução: fp --root {} run {phase_number} {n}\nOu, na raiz acadêmica: just run {phase_number} {n}",
            shell_quote(root)
        )
    } else {
        "\nSem entrypoint registrado.".into()
    };
    cliclack::outro(format!("{} criado.{run}", target.display()))?;
    Ok(())
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

pub fn list(root: &Path) -> Result<String> {
    let doc = registry::load(root)?;
    let phases = discovery::discover(root, "fase-")?;
    let mut output = String::new();
    if phases.is_empty() {
        output.push_str("Nenhuma fase de projetos disponível. Use fp create.\n");
    }
    for phase in phases {
        output.push_str(&format!("Fase {} — {}\n", phase.number, phase.label));
        for chapter in discovery::discover(&phase.path, "cap-")? {
            let key = format!("{}.{}", phase.number, chapter.number);
            let entry = doc.get("chapters").and_then(|c| c.get(&key));
            let matches = entry
                .and_then(|e| e.get("path"))
                .and_then(|p| p.as_str())
                .is_some_and(|p| {
                    root.join(p).canonicalize().ok() == chapter.path.canonicalize().ok()
                });
            let runnable = matches && registry::executable(&doc, &key);
            let name = entry
                .filter(|_| matches)
                .and_then(|e| e.get("name"))
                .and_then(|s| s.as_str())
                .unwrap_or(&chapter.label);
            output.push_str(&format!(
                "  └─ Capítulo {} — {} — {} [{}]\n",
                chapter.number,
                name,
                registry::technology(root, entry, &chapter.path),
                if runnable {
                    "executável"
                } else {
                    "sem comando"
                }
            ));
        }
    }
    // Entradas explícitas podem usar subpastas ou convenções fora da descoberta.
    for (key, entry) in doc["chapters"].as_table().ok_or("Registro inválido")? {
        let path = entry["path"].as_str().ok_or("path inválido")?;
        let parts: Vec<_> = key.split('.').collect();
        let phase_number: u32 = parts[0].parse()?;
        let chapter_number: u32 = parts[1].parse()?;
        let found = discovery::discover(root, "fase-")?
            .into_iter()
            .filter(|p| p.number == phase_number)
            .map(|p| discovery::discover(&p.path, "cap-"))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .any(|c| {
                c.number == chapter_number
                    && c.path.canonicalize().ok() == root.join(path).canonicalize().ok()
            });
        if !found {
            output.push_str(&format!(
                "Registro {key} — {path} — {} [{}]\n",
                registry::technology(root, Some(entry), &root.join(path)),
                if registry::executable(&doc, key) {
                    "executável"
                } else {
                    "sem comando"
                }
            ));
        }
    }
    Ok(output)
}
