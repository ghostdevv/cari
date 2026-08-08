use std::path::PathBuf;

use color_eyre::eyre::{OptionExt, Result};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

// #[derive(Debug, Deserialize, JsonSchema)]
// pub struct Resource {
//     pub url: String,
//     pub sha256: String,
// }

#[derive(Debug, Serialize, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Fabric,
    Velocity,
    Paper,
    Quilt,
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

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    // pub server: Resource,
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
            Loader::Fabric | Loader::Quilt => self.path.join("./mods"),
            Loader::Velocity | Loader::Paper => self.path.join("./plugins"),
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
