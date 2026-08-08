use std::path::PathBuf;

use color_eyre::eyre::{OptionExt, Result};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
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
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Content {
    pub id: String,
    pub version: String,
    #[serde(default = "default_false")]
    external: bool,
}

#[derive(strum_macros::Display, Debug, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Runtime {
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

impl Runtime {
    pub fn to_download_url(&self, game_version: &str) -> String {
        match self {
            Runtime::Fabric {
                loader_version,
                installer_version,
                ..
            } => {
                format!(
                    "https://meta.fabricmc.net/v2/versions/loader/{}/{}/{}/server/jar",
                    game_version, loader_version, installer_version
                )
            }
            Runtime::Velocity {
                version, sha256, ..
            } => {
                format!("{}/{}/velocity-{}.jar", PAPERMC_BASE_URL, sha256, version)
            }
            Runtime::Paper {
                version, sha256, ..
            } => {
                format!("{}/{}/paper-{}.jar", PAPERMC_BASE_URL, sha256, version)
            }
        }
    }

    pub fn sha512(&self) -> &str {
        match self {
            Runtime::Fabric { sha512, .. } => sha512,
            Runtime::Velocity { sha512, .. } => sha512,
            Runtime::Paper { sha512, .. } => sha512,
        }
    }
}

impl From<Runtime> for String {
    fn from(val: Runtime) -> Self {
        match val {
            Runtime::Fabric { .. } => "fabric".into(),
            Runtime::Velocity { .. } => "velocity".into(),
            Runtime::Paper { .. } => "paper".into(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub runtime: Runtime,
    pub loader: Loader,
    pub game_version: String,
    pub content: Vec<Content>,
}

#[derive(Debug)]
pub struct Server {
    pub name: String,
    pub path: PathBuf,
    pub cfg: Config,
}

impl Server {
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
            cfg.content.retain(|c| !c.external);
            Ok(Some(cfg))
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(e.into()),
        },
    }
}

pub fn load_server(path: PathBuf) -> Result<Option<Server>> {
    let cfg = load_config(&path.join("./cari.json"))?;
    let name = path
        .file_name()
        .ok_or_eyre("failed to find server name")?
        .to_string_lossy()
        .to_string();

    Ok(cfg.map(|cfg| Server { name, path, cfg }))
}

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
