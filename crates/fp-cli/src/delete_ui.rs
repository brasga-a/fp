use crate::{Result, delete::*, discovery};
use std::path::Path;
pub fn run(root: &Path) -> Result<()> {
    use std::io::{self, IsTerminal};
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err("fp delete requer um terminal interativo (TUI inline)".into());
    }
    cliclack::intro("FP · Excluir projetos")?;
    let entire_phase = cliclack::select("O que deseja excluir?")
        .item(false, "Capítulo", "Excluir um capítulo de uma fase")
        .item(true, "Fase inteira", "Excluir a fase e todo o seu conteúdo")
        .interact()?;
    let phases = discovery::discover(root, "fase-")?;
    if phases.is_empty() {
        cliclack::outro("Nenhuma fase disponível para excluir")?;
        return Ok(());
    }
    let mut select = cliclack::select("Qual fase?");
    for phase in &phases {
        select = select.item(phase.number, &phase.label, "");
    }
    let phase = select.interact()?;
    let selected = phases
        .iter()
        .find(|p| p.number == phase)
        .ok_or("Fase inválida")?;
    let chapter = if entire_phase {
        None
    } else {
        let chapters = discovery::discover(&selected.path, "cap-")?;
        if chapters.is_empty() {
            cliclack::outro("Essa fase não possui capítulos para excluir")?;
            return Ok(());
        }
        let mut select = cliclack::select("Qual capítulo?");
        for chapter in chapters {
            select = select.item(chapter.number, chapter.label, "");
        }
        Some(select.interact()?)
    };
    let plan = prepare(root, phase, chapter)?;
    let registrations = if plan.registrations.is_empty() {
        "Nenhum".to_string()
    } else {
        plan.registrations.join(", ")
    };
    cliclack::note(
        "Resumo da exclusão",
        format!(
            "Tipo: {}\nDiretório: {}\nRegistros removidos: {registrations}\nTodos os arquivos e subpastas serão excluídos permanentemente.",
            if entire_phase {
                "Fase inteira"
            } else {
                "Capítulo"
            },
            plan.target().display()
        ),
    )?;
    if !cliclack::confirm("Excluir permanentemente?")
        .initial_value(false)
        .interact()?
    {
        cliclack::outro_cancel("Exclusão cancelada; nenhum arquivo alterado")?;
        return Ok(());
    }
    let spinner = cliclack::spinner();
    spinner.start("Excluindo arquivos e atualizando registro");
    if let Err(error) = execute(&plan, true) {
        spinner.error("Falha ao excluir");
        return Err(error);
    }
    spinner.stop("Exclusão concluída");
    cliclack::outro(format!("{} excluído", plan.target().display()))?;
    Ok(())
}
