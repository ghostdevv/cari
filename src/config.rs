use color_eyre::eyre::{OptionExt, Result};
#[cfg(debug_assertions)]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg_attr(debug_assertions, derive(JsonSchema))]
#[derive(strum_macros::Display, Debug, Serialize, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Loader {
    Fabric,
    Velocity,
    Paper,
    Quilt,
    Bukkit,
    Folia,
    Spigot,
    Purpur,
    Neoforge,
    Forge,
    Sponge,
    Bungeecord,
    Waterfall,
}

#[cfg_attr(debug_assertions, derive(JsonSchema))]
#[derive(Debug, Serialize, Deserialize)]
pub struct Content {
    pub id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    external: Option<bool>,
}

impl Content {
    pub const fn new(id: String, version: String) -> Self {
        Self {
            id,
            version,
            external: None,
        }
    }
}

#[cfg_attr(debug_assertions, derive(JsonSchema))]
#[derive(strum_macros::Display, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Server {
    #[serde(rename_all = "camelCase")]
    Fabric {
        loader_version: String,
        installer_version: String,
        sha512: String,
    },
    #[serde(rename_all = "camelCase")]
    Velocity {
        version: String,
        sha256: String,
        sha512: String,
    },
    #[serde(rename_all = "camelCase")]
    Paper {
        version: String,
        sha256: String,
        sha512: String,
    },
}

const PAPERMC_BASE_URL: &str = "https://fill-data.papermc.io/v1/objects";

impl Server {
    pub fn to_download_url(&self, game_version: &str) -> String {
        match self {
            Self::Fabric {
                loader_version,
                installer_version,
                ..
            } => {
                format!(
                    "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/{installer_version}/server/jar"
                )
            }
            Self::Velocity {
                version, sha256, ..
            } => {
                format!("{PAPERMC_BASE_URL}/{sha256}/velocity-{version}.jar")
            }
            Self::Paper {
                version, sha256, ..
            } => {
                format!("{PAPERMC_BASE_URL}/{sha256}/paper-{version}.jar")
            }
        }
    }

    pub fn sha512(&self) -> &str {
        match self {
            Self::Fabric { sha512, .. }
            | Self::Velocity { sha512, .. }
            | Self::Paper { sha512, .. } => sha512,
        }
    }
}

impl From<Server> for String {
    fn from(val: Server) -> Self {
        match val {
            Server::Fabric { .. } => "fabric".into(),
            Server::Velocity { .. } => "velocity".into(),
            Server::Paper { .. } => "paper".into(),
        }
    }
}

fn default_config_schema_url() -> String {
    "https://raw.githubusercontent.com/ghostdevv/cari/refs/heads/main/cari.schema.json".into()
}

#[cfg_attr(debug_assertions, derive(JsonSchema))]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "$schema", default = "default_config_schema_url")]
    schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<Server>,
    pub loader: Loader,
    pub game_version: String,
    pub content: Vec<Content>,
}

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub cfg: Config,
}

impl Project {
    pub fn save(&self) -> Result<()> {
        std::fs::write(
            self.path.join("cari.json"),
            format!("{}\n", serde_json::to_string_pretty(&self.cfg)?),
        )?;
        Ok(())
    }

    pub fn content_dir(&self) -> PathBuf {
        match self.cfg.loader {
            Loader::Fabric | Loader::Quilt | Loader::Neoforge => self.path.join("./mods"),
            _ => self.path.join("./plugins"),
        }
    }
}

fn load_config(path: &PathBuf) -> Result<Option<Config>> {
    match std::fs::File::open(path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            let mut cfg: Config = serde_json::from_reader(reader)?;
            cfg.content.retain(|c| !c.external.is_some_and(|b| b));
            Ok(Some(cfg))
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(e.into()),
        },
    }
}

pub fn load_project(path: &PathBuf) -> Result<Option<Project>> {
    let cfg = load_config(path)?;

    let path = path
        .parent()
        .ok_or_eyre("no project parent dir found")?
        .to_path_buf();

    let name = path
        .file_name()
        .ok_or_eyre("failed to find project name")?
        .to_string_lossy()
        .to_string();

    Ok(cfg.map(|cfg| Project { name, path, cfg }))
}

#[cfg(debug_assertions)]
pub fn write_schema() -> Result<()> {
    let schema_path = std::env::current_dir()?.join("./cari.schema.json");

    if !schema_path.exists() {
        std::fs::write(
            schema_path,
            serde_json::to_string_pretty(&schema_for!(Config))?,
        )?;
    }

    Ok(())
}
