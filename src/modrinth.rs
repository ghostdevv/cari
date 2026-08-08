use crate::config::Loader;
use color_eyre::eyre::{self, Result, eyre};
use serde::{Deserialize, Serialize, Serializer};

const MODRINTH_BASE_URL: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str = "bedrocko (+https://bedrocko.com)";

fn as_json_array<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    let json = serde_json::to_string(&[value]).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&json)
}

#[derive(Debug, Serialize)]
struct VersionQuery<'a> {
    #[serde(serialize_with = "as_json_array")]
    loaders: &'a Loader,
    #[serde(serialize_with = "as_json_array")]
    game_versions: &'a str,
    include_changelog: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionStatus {
    Listed,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionType {
    Release,
    Beta, // todo discount beta versions when finding latest
}

#[derive(Debug, Deserialize)]
pub struct VersionFileHashes {
    pub sha512: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionFile {
    pub id: String,
    pub hashes: VersionFileHashes,
    pub url: String,
    pub filename: String,
    pub primary: bool,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<Loader>,
    pub date_published: chrono::DateTime<chrono::Utc>,
    pub status: VersionStatus,
    pub version_type: VersionType,
    pub files: Vec<VersionFile>,
}

pub async fn get_latest_project_version(
    project: &str,
    loader: &Loader,
    game_version: &str,
) -> Result<Version> {
    let version = reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()?
        .get(format!("{}/project/{}/version", MODRINTH_BASE_URL, project))
        .query(&VersionQuery {
            loaders: loader,
            game_versions: game_version,
            include_changelog: false,
        })
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Version>>()
        .await?
        .into_iter()
        .max_by_key(|item| item.date_published)
        .ok_or_else(|| eyre!("no versions returned from api for {}", project))?;

    Ok(version)
}

pub async fn get_project_version(
    project: &str,
    version: &str,
    loader: &Loader,
    game_version: &str,
) -> Result<Version> {
    let version = reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()?
        .get(format!(
            "{}/project/{}/version/{}",
            MODRINTH_BASE_URL, project, version
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<Version>()
        .await?;

    if !version.loaders.contains(loader) {
        eyre::bail!("version does not support loader");
    } else if !version.game_versions.contains(&game_version.to_string()) {
        eyre::bail!("version does not support game version");
    }

    Ok(version)
}
