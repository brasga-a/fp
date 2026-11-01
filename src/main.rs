use clap::Parser;
use fiap_projects::{
    Result,
    cli::{Cli, Commands},
    delete, processes, root, tui, workspace,
};

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::Interrupted)
            {
                std::process::exit(130);
            }
            eprintln!("Erro: {error}");
            std::process::exit(2);
        }
    }
}
fn run() -> Result<i32> {
    let raw: Vec<_> = std::env::args_os().collect();
    let mut cli = Cli::parse_from(&raw);
    cli.preserve_run_separator(&raw);
    // Bancos são independentes da raiz acadêmica e funcionam fora dela.
    if let Commands::Db { args } = &cli.command {
        return processes::run_db(&processes::find_db()?, args);
    }
    if let Commands::Init { check } = &cli.command {
        let target = root::locate(
            cli.root.as_deref(),
            None,
            std::env::var_os("HOME").as_deref(),
        )?;
        return workspace::run(&target, *check);
    }
    let root = root::resolve(
        cli.root.as_deref(),
        None,
        std::env::var_os("HOME").as_deref(),
    )?;
    match cli.command {
        Commands::Init { .. } => unreachable!("init é tratado antes da validação da raiz"),
        Commands::Create => tui::run(&root)?,
        Commands::Delete => delete::run(&root)?,
        Commands::Db { .. } => unreachable!("db é tratado antes da raiz acadêmica"),
        Commands::List => print!("{}", tui::list(&root)?),
        Commands::Run {
            phase,
            chapter,
            args,
        } => {
            return processes::run_just(&root, &processes::find("just")?, phase, chapter, &args);
        }
    }
    Ok(0)
}
