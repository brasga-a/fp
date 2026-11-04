use crate::Result;
use std::io::IsTerminal;
pub fn run(check: bool) -> Result<i32> {
    if !check {
        fp_update::installation_available()?;
    }
    let info = fp_update::check()?;
    println!(
        "Versão instalada: {}\nÚltima versão estável: {}",
        env!("CARGO_PKG_VERSION"),
        info.version
    );
    if !info.newer {
        println!("O FP já está atualizado.");
        return Ok(0);
    }
    println!("Atualização disponível: v{}", info.version);
    if check {
        return Ok(0);
    }
    let plan = fp_update::prepare_install(&info.version)?;
    if !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return Err("Execute fp update em um terminal para confirmar".into());
    }
    cliclack::intro("FP · Atualizar")?;
    if !cliclack::confirm(format!("Atualizar para v{}?", info.version))
        .initial_value(false)
        .interact()?
    {
        cliclack::outro_cancel("Atualização cancelada; nenhum arquivo alterado")?;
        return Ok(0);
    }
    let spinner = cliclack::spinner();
    spinner.start("Baixando, verificando SHA-256 e instalando…");
    if let Err(e) = fp_update::install(plan) {
        spinner.error("Atualização não concluída");
        return Err(e);
    }
    spinner.stop("Instalação concluída");
    cliclack::outro(format!("FP atualizado para v{}!", info.version))?;
    Ok(0)
}
