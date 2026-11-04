use crate::Result;
use clap::{Args, Subcommand, ValueEnum};
use fp_database::{Engine, Manager, Paths, Resource};
use serde_json::{Value, json};
use std::{io::IsTerminal, path::PathBuf, time::Duration};
#[derive(Clone, Debug, ValueEnum, PartialEq)]
pub enum Output {
    Text,
    Json,
}
#[derive(Clone, Debug, Subcommand)]
pub enum Action {
    /// Sintaxe legada; informa migração para create/run
    Up,
    #[command(long_flag = "init")]
    Init,
    #[command(long_flag = "create")]
    Create,
    #[command(long_flag = "delete")]
    Delete,
    #[command(long_flag = "run")]
    Run,
    Stop,
    List,
    Status,
    Logs,
    Exec,
    ProvisionUser,
    #[command(long_flag = "visual")]
    Visual,
    /// Compatibilidade: consulta logs; SQLite: use exec --file SCRIPT
    Connect {
        id: String,
    },
}
#[derive(Clone, Debug, Args)]
pub struct Database {
    #[command(subcommand)]
    pub action: Option<Action>,
    #[arg(long, global = true)]
    pub name: Option<String>,
    #[arg(long,global=true,value_parser=["sqlite","oracle","mongodb"])]
    pub engine: Option<String>,
    #[arg(long, global = true)]
    pub file: Option<PathBuf>,
    #[arg(long, global = true)]
    pub existing: bool,
    /// Adota explicitamente um recurso no Docker rootless do FP
    #[arg(long, global = true)]
    pub rootless: bool,
    #[arg(long, global = true)]
    pub container: Option<String>,
    #[arg(long, global = true)]
    pub volume: Option<String>,
    #[arg(long, global = true)]
    pub image: Option<String>,
    #[arg(long, global = true)]
    pub port: Option<u16>,
    #[arg(long, global = true, default_value = "127.0.0.1")]
    pub host: String,
    #[arg(long, global = true)]
    pub username: Option<String>,
    #[arg(long, global = true)]
    pub password_env: Option<String>,
    /// Identificador exato autorizado para exclusão
    #[arg(long, global = true)]
    pub confirm: Option<String>,
    #[arg(long, global = true)]
    pub remove_container: bool,
    #[arg(long, global = true)]
    pub delete_data: bool,
    #[arg(long, global = true)]
    pub wait: bool,
    #[arg(long, global = true, default_value_t = 60)]
    pub timeout: u64,
    #[arg(long, global = true)]
    pub discover: bool,
    /// Baixa e prepara o Docker rootless dedicado (init)
    #[arg(long, global = true)]
    pub setup_runtime: bool,
    #[arg(long, global = true)]
    pub dry_run: bool,
    #[arg(long, global = true)]
    pub no_input: bool,
    #[arg(long, global = true, value_enum, default_value = "text")]
    pub output: Output,
    /// Caminhos isolados de configuração, dados e estado
    #[arg(long, global = true)]
    pub config_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    pub state_dir: Option<PathBuf>,
    /// Sintaxe legada: consulte a migração em db --help
    #[arg(long, global = true)]
    pub oracle: bool,
    #[arg(short = 'f', long = "filter", global = true)]
    pub legacy_filter: Option<String>,
    #[arg(long, global = true)]
    pub mongodb: bool,
    #[arg(long, global = true)]
    pub sqlite: bool,
}
fn manager(a: &Database) -> Result<Manager> {
    let mut paths = match (&a.config_dir, &a.data_dir, &a.state_dir) {
        (Some(config), Some(data), Some(state)) => Paths {
            config: config.clone(),
            data: data.clone(),
            state: state.clone(),
        },
        _ => Paths::from_env()?,
    };
    if let Some(p) = &a.config_dir {
        paths.config = p.clone();
    }
    if let Some(p) = &a.data_dir {
        paths.data = p.clone();
    }
    if let Some(p) = &a.state_dir {
        paths.state = p.clone();
    }
    let mut m = Manager::new(paths);
    m.timeout = Duration::from_secs(a.timeout);
    Ok(m)
}
fn name(a: &Database) -> Result<&str> {
    a.name
        .as_deref()
        .ok_or_else(|| "E_ARGUMENT: informe --name".into())
}
pub fn execute(a: &Database, m: &Manager) -> Result<Value> {
    if a.oracle
        || a.mongodb
        || a.sqlite
        || a.legacy_filter.is_some()
        || matches!(a.action, Some(Action::Up))
    {
        return Err("E_MIGRATION: init agora apenas configura; use create --engine ENGINE --name NOME, depois run --name NOME. Para legado, create --existing com container/volume explícitos".into());
    }
    match a
        .action
        .as_ref()
        .ok_or("E_ARGUMENT: informe uma operação; consulte fp db --help")?
    {
        Action::Init => {
            if a.setup_runtime {
                m.setup_native_runtime(a.dry_run)
            } else {
                m.init(a.dry_run)
            }
        }
        Action::Create => {
            let engine = match a.engine.as_deref().ok_or("E_ARGUMENT: informe --engine")? {
                "sqlite" => Engine::Sqlite,
                "oracle" => Engine::Oracle,
                "mongodb" => Engine::Mongodb,
                _ => unreachable!(),
            };
            #[cfg(target_os = "linux")]
            if engine != Engine::Sqlite && !a.existing && m.installed_runtime()?.is_none() {
                return Err("E_INIT: execute fp init ou fp db init --setup-runtime para preparar o Docker rootless".into());
            }
            let mut r = Resource::new(name(a)?.into(), engine);
            if a.rootless {
                let runtime = m
                    .installed_runtime()?
                    .ok_or("E_INIT: runtime rootless não configurado")?;
                r.docker_host = Some(format!("unix://{}", runtime.socket.display()));
            }
            r.file = a.file.clone();
            r.container = a.container.clone();
            r.volume = a.volume.clone();
            r.image = a.image.clone();
            r.port = a.port;
            r.host = a.host.clone();
            r.username = a.username.clone();
            r.password_env = a.password_env.clone();
            m.create(r, a.existing, a.dry_run)
        }
        Action::List => {
            if a.discover {
                m.discover()
            } else {
                Ok(json!({"resources":m.list()?}))
            }
        }
        Action::Status => m.status(name(a)?),
        Action::Logs => m.logs(name(a)?),
        Action::Connect { id } => {
            if m.get(id)?.engine == Engine::Sqlite {
                return Err(
                    "E_MIGRATION: use exec --name NOME --file SCRIPT.sql para SQLite".into(),
                );
            }
            m.logs(id)
        }
        Action::Run | Action::Stop => m.run(
            name(a)?,
            matches!(a.action, Some(Action::Stop)),
            a.wait,
            a.dry_run,
        ),
        Action::Delete => m.delete(
            name(a)?,
            a.confirm
                .as_deref()
                .ok_or("E_CONFIRM: informe --confirm NOME")?,
            a.remove_container,
            a.delete_data,
            a.dry_run,
        ),
        Action::Exec => {
            let file = a.file.as_ref().ok_or("E_ARGUMENT: informe --file SCRIPT")?;
            let script = std::fs::read(file)?;
            m.exec(name(a)?, &script, a.dry_run)
        }
        Action::ProvisionUser => m.provision_user(
            name(a)?,
            a.username
                .as_deref()
                .ok_or("E_ARGUMENT: informe --username")?,
            a.password_env
                .as_deref()
                .ok_or("E_ARGUMENT: informe --password-env")?,
            a.dry_run,
        ),
        Action::Up => unreachable!("legado tratado antes do despacho"),
        Action::Visual => Err("E_ARGUMENT: visual requer um terminal".into()),
    }
}
pub fn run(mut a: Database) -> Result<i32> {
    let visual = a.action.is_none() || matches!(a.action, Some(Action::Visual));
    if visual
        && (a.no_input
            || a.output == Output::Json
            || !std::io::stdin().is_terminal()
            || !std::io::stderr().is_terminal())
    {
        return Err("E_TTY: informe uma operação explícita; use fp db --help".into());
    }
    let m = manager(&a)?;
    if visual {
        return visual_menu(&mut a, &m);
    }
    let result = execute(&a, &m).map_err(|e| -> Box<dyn std::error::Error> {
        if e.to_string().starts_with("E_") {
            e
        } else {
            format!("E_DATABASE: {e}").into()
        }
    })?;
    print_result(&result, &a.output)?;
    Ok(0)
}
fn print_result(v: &Value, output: &Output) -> Result<()> {
    if *output == Output::Json {
        println!("{}", serde_json::to_string(v)?);
    } else {
        println!("{}", serde_json::to_string_pretty(v)?);
    }
    Ok(())
}
fn input(prompt: &str) -> Result<String> {
    Ok(cliclack::input(prompt).interact()?)
}
fn optional(prompt: &str) -> Result<Option<String>> {
    let value: String = cliclack::input(prompt).required(false).interact()?;
    Ok((!value.is_empty()).then_some(value))
}
fn visual_menu(a: &mut Database, m: &Manager) -> Result<i32> {
    cliclack::intro("FP · Bancos de dados")?;
    loop {
        let selected = cliclack::select("Operação")
            .item(0, "Criar banco", "")
            .item(1, "Iniciar banco", "")
            .item(2, "Parar banco", "")
            .item(3, "Listar bancos", "")
            .item(4, "Consultar status", "")
            .item(5, "Executar script", "")
            .item(6, "Excluir banco", "")
            .item(7, "Configurações", "Preparar diretórios XDG")
            .item(8, "Consultar logs", "")
            .item(9, "Descobrir bancos legados", "")
            .item(
                10,
                "Provisionar usuário Oracle",
                "Somente criação explícita, idempotente",
            )
            .item(11, "Sair", "")
            .item(
                12,
                "Preparar Docker rootless",
                "Baixa runtime e imagens públicas, sem login",
            )
            .interact()?;
        if selected == 11 {
            break;
        }
        a.action = Some(match selected {
            0 => Action::Create,
            1 => Action::Run,
            2 => Action::Stop,
            3 | 9 => Action::List,
            4 => Action::Status,
            5 => Action::Exec,
            6 => Action::Delete,
            7 | 12 => Action::Init,
            10 => Action::ProvisionUser,
            _ => Action::Logs,
        });
        a.discover = selected == 9;
        a.setup_runtime = selected == 12;
        a.delete_data = false;
        a.remove_container = false;
        a.existing = false;
        a.wait = false;
        if selected == 0 {
            a.rootless = false;
            a.file = None;
            a.container = None;
            a.volume = None;
            a.image = None;
            a.port = None;
            a.username = None;
            a.password_env = None;
            a.engine = Some(
                cliclack::select("Engine")
                    .item("sqlite", "SQLite", "")
                    .item("oracle", "Oracle Free", "")
                    .item("mongodb", "MongoDB", "")
                    .interact()?
                    .into(),
            );
            a.name = Some(input("Nome/identificador")?);
            a.existing = cliclack::confirm("Registrar recurso existente sem recriar?")
                .initial_value(false)
                .interact()?;
            if a.engine.as_deref() == Some("sqlite") {
                a.file =
                    optional("Caminho absoluto SQLite (vazio: diretório XDG)")?.map(PathBuf::from);
            } else {
                a.container = optional("Nome do container (vazio: fp-NOME)")?;
                a.volume = optional("Volume (vazio: fp-NOME-data)")?;
                a.image = optional("Imagem (vazio: padrão da engine)")?;
                a.port = optional("Porta local (vazio: padrão)")?
                    .map(|s| s.parse())
                    .transpose()?;
                a.host = input("Host de loopback: 127.0.0.1 ou ::1")?;
                // Novas instâncias usam credenciais geradas; importações mantêm as existentes.
                if a.existing {
                    a.rootless = cliclack::confirm("Recurso está no Docker rootless do FP?")
                        .initial_value(false)
                        .interact()?;
                    a.username = optional("Usuário MongoDB existente")?;
                    a.password_env = optional("Variável da senha existente")?;
                }
            }
        } else if [1, 2, 4, 5, 6, 8, 10].contains(&selected) {
            let rs = m.list()?;
            if rs.is_empty() {
                cliclack::log::info("Nenhum banco registrado")?;
                continue;
            }
            let mut menu = cliclack::select("Recurso");
            for r in &rs {
                menu = menu.item(r.id.clone(), &r.id, format!("{:?}", r.engine));
            }
            a.name = Some(menu.interact()?);
            if selected == 1 {
                a.wait = cliclack::confirm(
                    "Aguardar o banco ficar disponível? (Não retorna após iniciar o container)",
                )
                .initial_value(false)
                .interact()?;
            }
            if selected == 10 {
                a.username = Some(input("Usuário Oracle")?);
                a.password_env = Some(input("Variável de ambiente da senha")?);
            }
            if selected == 5 {
                a.file = Some(input("Arquivo de script solicitado")?.into());
            }
            if selected == 6 {
                let r = m.get(name(a)?)?;
                cliclack::note(
                    "Persistência",
                    format!(
                        "{:?}\nArquivo: {:?}\nVolume: {:?}\nPor padrão só o registro será removido; os dados serão preservados.",
                        r.engine, r.file, r.volume
                    ),
                )?;
                if r.engine != Engine::Sqlite {
                    a.remove_container = cliclack::confirm("Remover também o container parado?")
                        .initial_value(false)
                        .interact()?;
                }
                if r.owned {
                    a.delete_data = cliclack::confirm("Excluir permanentemente os dados também?")
                        .initial_value(false)
                        .interact()?;
                }
                a.confirm = Some(input("Digite o identificador exato para autorizar")?);
            }
        }
        if matches!(
            a.action,
            Some(
                Action::Create
                    | Action::Delete
                    | Action::Init
                    | Action::Run
                    | Action::Stop
                    | Action::Exec
                    | Action::ProvisionUser
            )
        ) {
            cliclack::note(
                "Resumo",
                format!(
                    "Operação: {:?}\nRecurso: {:?}\nExcluir dados: {}\nRemover container: {}",
                    a.action, a.name, a.delete_data, a.remove_container
                ),
            )?;
            if !cliclack::confirm("Confirmar operação?")
                .initial_value(false)
                .interact()?
            {
                cliclack::log::info("Cancelado; nenhum recurso alterado")?;
                continue;
            }
        }
        match execute(a, m) {
            Ok(result) => cliclack::log::success(serde_json::to_string_pretty(&result)?)?,
            Err(e) => cliclack::log::error(e.to_string())?,
        }
    }
    cliclack::outro("Gerenciador encerrado")?;
    Ok(0)
}
