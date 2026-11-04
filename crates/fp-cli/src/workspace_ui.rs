use crate::{Result, workspace::*};
use fp_workspace::root;
use std::path::Path;
pub fn run(root: &Path, check: bool) -> Result<i32> {
    println!("FP · Workspace acadêmico\nRaiz: {}", root.display());
    let initial = inspect(root);
    for missing in &initial.missing {
        println!("[AUSENTE] {}", missing.display());
    }
    for problem in &initial.problems {
        eprintln!("[ERRO] {problem}");
    }
    if !initial.problems.is_empty() {
        return Ok(2);
    }
    if !check {
        let created = initialize(root)?;
        for path in &created {
            println!("[CRIADO] {}", path.display());
        }
        if created.is_empty() {
            println!("[OK] Estrutura completa; arquivos existentes preservados");
        } else {
            println!("[OK] Estrutura preparada; arquivos existentes preservados");
        }
    } else if initial.missing.is_empty() {
        println!("[OK] Estrutura completa");
    }
    println!("\nFerramentas no PATH:");
    let mut required_missing = false;
    for tool in tools(
        &std::env::var_os("PATH").unwrap_or_default(),
        root::home().as_deref().map(std::path::Path::as_os_str),
    ) {
        if let Some(path) = &tool.executable {
            if tool.label == "Python 3.14+" && !python_supported(path) {
                eprintln!(
                    "[ERRO] Python: versão incompatível ou não identificada em {} (necessário 3.14+)",
                    path.display()
                );
                required_missing = true;
            } else {
                println!("[OK] {}: {}", tool.label, path.display());
            }
        } else {
            required_missing |= tool.required;
            println!(
                "[{}] {}: ausente no PATH ({})",
                if tool.required { "FALTA" } else { "OPCIONAL" },
                tool.label,
                if tool.required {
                    "necessário para just run"
                } else {
                    "apenas para seu ecossistema"
                }
            );
            if let Some(found) = &tool.outside_path {
                let directory = found.parent().ok_or("Caminho de ferramenta inválido")?;
                #[cfg(windows)]
                println!(
                    "  Encontrado em {}. Adicione {} ao PATH do usuário e reabra o terminal.",
                    found.display(),
                    directory.display()
                );
                #[cfg(unix)]
                println!(
                    "  Encontrado em {}. No Fish: fish_add_path '{}'",
                    found.display(),
                    directory
                        .display()
                        .to_string()
                        .replace('\\', "\\\\")
                        .replace('\'', "\\'")
                );
            } else {
                println!("  Instale a ferramenta e disponibilize seu executável no PATH.");
            }
        }
    }
    if check {
        println!("\nVerificação sem alterações. Use fp init para criar os arquivos ausentes.");
    } else {
        println!("\nUse fp create para criar fases e capítulos; fp list para listar.");
    }
    Ok(i32::from(
        required_missing || (check && !initial.missing.is_empty()),
    ))
}
