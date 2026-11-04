use clap::Parser;
use fiap_projects::{
    Result,
    cli::{Cli, Commands},
    database, delete, processes, root, tui, update, workspace,
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
    if let Commands::Update { check } = &cli.command {
        return update::run(*check);
    }
    // Bancos são independentes da raiz acadêmica e funcionam fora dela.
    if let Commands::Db(args) = &cli.command {
        return database::run((**args).clone());
    }
    if let Commands::Init {
        check,
        academic_only,
    } = &cli.command
    {
        let target = root::locate(
            cli.root.as_deref(),
            None,
            root::home().as_deref().map(std::path::Path::as_os_str),
        )?;
        let code = workspace::run(&target, *check)?;
        if code == 2 || *academic_only {
            return Ok(code);
        }
        return init_runtime(code, *check);
    }
    let root = root::resolve(
        cli.root.as_deref(),
        None,
        root::home().as_deref().map(std::path::Path::as_os_str),
    )?;
    match cli.command {
        Commands::Update { .. } => unreachable!("update é independente da raiz acadêmica"),
        Commands::Init { .. } => unreachable!("init é tratado antes da validação da raiz"),
        Commands::Create => tui::run(&root)?,
        Commands::Delete => delete::run(&root)?,
        Commands::Db(..) => unreachable!("db é tratado antes da raiz acadêmica"),
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

#[cfg(windows)]
fn init_runtime(code: i32, _: bool) -> Result<i32> {
    eprintln!(
        "FP · Workspace acadêmico no Windows. Bootstrap rootless exclusivo do Linux; bancos Docker podem usar Docker Desktop com Linux containers (fp db init)."
    );
    Ok(code)
}
#[cfg(target_os = "linux")]
fn init_runtime(code: i32, check: bool) -> Result<i32> {
    eprintln!(
        "FP · Docker rootless dedicado: pacotes oficiais verificados e imagens públicas Oracle/MongoDB. Nenhum sudo ou login será solicitado."
    );
    let m = fp_database::Manager::new(fp_database::Paths::from_env()?);
    let location = fp_database::runtime::RuntimeLocation::detect()?;
    let spinner = if !check {
        let s = cliclack::spinner();
        s.start("Preparando Docker rootless e imagens de bancos…");
        Some(s)
    } else {
        None
    };
    let outcome = m.setup_runtime(&location, check, &fp_database::runtime::NativeSetup(&m));
    if let Some(s) = spinner {
        if outcome.is_ok() {
            s.stop("Runtime de bancos preparado");
        } else {
            s.error("Preparação incompleta; os dados existentes foram preservados");
        }
    }
    let result = outcome?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(if check && result["ready"] != true {
        1
    } else {
        code
    })
}
