use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Sqlite,
    Oracle,
    Mongodb,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub engine: Engine,
    pub file: Option<PathBuf>,
    pub container: Option<String>,
    pub volume: Option<String>,
    pub image: Option<String>,
    pub port: Option<u16>,
    pub host: String,
    pub username: Option<String>,
    pub password_env: Option<String>,
    pub owned: bool,
    pub docker_id: Option<String>,
    #[serde(default)]
    pub docker_host: Option<String>,
    #[serde(default)]
    pub file_identity: Option<(u64, u64)>,
}
impl Resource {
    pub fn new(id: String, engine: Engine) -> Self {
        Self {
            id,
            engine,
            file: None,
            container: None,
            volume: None,
            image: None,
            port: None,
            host: "127.0.0.1".into(),
            username: None,
            password_env: None,
            owned: false,
            docker_id: None,
            docker_host: None,
            file_identity: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Paths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
}
impl Paths {
    pub fn from_env() -> Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("E_CONFIG: HOME ausente")?;
        let xdg = |key, default: &str| {
            std::env::var_os(key)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(default))
                .join("fp")
        };
        Ok(Self {
            config: xdg("XDG_CONFIG_HOME", ".config"),
            data: xdg("XDG_DATA_HOME", ".local/share"),
            state: xdg("XDG_STATE_HOME", ".local/state"),
        })
    }
}
