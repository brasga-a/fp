use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "fp",
    version,
    about = "Crie e execute projetos acadêmicos FIAP"
)]
pub struct Cli {
    /// Raiz acadêmica (padrão: ~/fiap)
    #[arg(long, global = true, env = "FP_ROOT")]
    pub root: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Prepara o workspace acadêmico e diagnostica ferramentas no PATH
    Init {
        /// Verifica sem criar ou modificar arquivos
        #[arg(long)]
        check: bool,
    },
    /// Inicia o assistente inline de criação
    Create,
    /// Seleciona e exclui um capítulo ou fase pela TUI inline
    Delete,
    /// Delega integralmente para just run
    Run {
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        phase: u32,
        #[arg(value_parser = clap::value_parser!(u32).range(1..))]
        chapter: u32,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Lista fases, capítulos, tecnologias e comandos registrados
    List,
    /// Delega para o db local (perfil de usuário, sem sudo automático)
    #[command(disable_help_flag = true)]
    Db {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

impl Cli {
    /// Clap consome o delimitador inicial de opções. O dispatcher Just precisa
    /// recebê-lo para distinguir `--task` do projeto de seu próprio seletor.
    pub fn preserve_run_separator(&mut self, raw: &[std::ffi::OsString]) {
        if let Commands::Run { args, .. } = &mut self.command
            && raw
                .get(raw.len().saturating_sub(args.len() + 1))
                .is_some_and(|s| s == "--")
        {
            args.insert(0, "--".into());
        }
    }
}
