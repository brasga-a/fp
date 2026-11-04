//! Bootstrap dedicado apenas no Linux; Docker do usuário permanece independente.
use crate::{Manager, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const ORACLE_IMAGE: &str = "gvenzl/oracle-free:23-slim";
pub const MONGO_IMAGE: &str = "mongo:8.0";
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Runtime {
    pub bin: PathBuf,
    pub socket: PathBuf,
    pub version: String,
}
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

impl Manager {
    pub fn setup_native_runtime(&self, check: bool) -> Result<serde_json::Value> {
        #[cfg(target_os = "linux")]
        {
            self.setup_runtime(&RuntimeLocation::detect()?, check, &NativeSetup(self))
        }
        #[cfg(windows)]
        {
            let _ = check;
            Err("E_PLATFORM: bootstrap Docker rootless dedicado é exclusivo do Linux; no Windows utilize Docker Desktop com Linux containers e fp db init".into())
        }
    }
    #[cfg(windows)]
    pub fn installed_runtime(&self) -> Result<Option<Runtime>> {
        if self.paths.config.join("runtime.json").try_exists()? {
            return Err("E_PLATFORM: configuração de runtime Linux incompatível com Windows; utilize diretórios próprios e Docker Desktop".into());
        }
        Ok(None)
    }
    pub(crate) fn scoped(&self, r: &crate::Resource) -> Self {
        let mut m = self.clone();
        m.docker_host = r.docker_host.clone();
        if r.docker_host.is_some()
            && let Ok(Some(runtime)) = self.installed_runtime()
        {
            m.docker = runtime.bin.join("docker");
        }
        m
    }
}
