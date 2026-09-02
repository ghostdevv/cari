use crate::{config::Loader, fs::USER_AGENT};
use color_eyre::eyre::{self, Result, eyre};
use serde::{Deserialize, Serialize, Serializer};

const MODRINTH_BASE_URL: &str = "https://api.modrinth.com/v2";

fn as_json_array<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    let json = serde_json::to_string(&[value]).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&json)
}

#[derive(Debug, Deserialize, PartialEq, Eq, strum_macros::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ProjectType {
    Mod,
    ResourcePack,
    Modpack,
    Shader,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    pub project_type: ProjectType,
}

async fn get_project(project_id: &str) -> Result<Project> {
    Ok(reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()?
        .get(format!("{MODRINTH_BASE_URL}/project/{project_id}"))
        .send()
        .await?
        .error_for_status()?
        .json::<Project>()
        .await?)
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
pub enum VersionType {
    Release,
    Beta,
    Alpha,
}

impl VersionType {
    /// Higher values are preferred when picking the "latest" version, so
    /// that release versions are chosen over beta/alpha ones published
    /// more recently.
    const fn priority(&self) -> u8 {
        match self {
            Self::Release => 2,
            Self::Beta => 1,
            Self::Alpha => 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct VersionFileHashes {
    pub sha512: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionFile {
    // pub id: String,
    pub hashes: VersionFileHashes,
    pub url: String,
    pub filename: String,
    // pub primary: bool,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<Loader>,
    pub date_published: chrono::DateTime<chrono::Utc>,
    #[allow(clippy::struct_field_names)]
    pub version_type: VersionType,
    pub files: Vec<VersionFile>,
}

impl Version {
    pub async fn assert_type(self, project_type: &ProjectType) -> Result<Self> {
        let project = get_project(&self.project_id).await?;

        if project.project_type != *project_type {
            eyre::bail!(
                "{} ({}) is not a {} (found a {})",
                self.project_id,
                self.id,
                project_type,
                project.project_type
            );
        }

        Ok(self)
    }
}

pub async fn get_latest_project_version(
    project: &str,
    loader: &Loader,
    game_version: &str,
) -> Result<Version> {
    let version = reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()?
        .get(format!("{MODRINTH_BASE_URL}/project/{project}/version"))
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
        .max_by_key(|item| (item.version_type.priority(), item.date_published))
        .ok_or_else(|| {
            eyre!(
                "no versions of {} found for game version {} using {} loader (check your cari.json)",
                project,
                game_version,
                loader
            )
        })?;

    Ok(version)
}

pub async fn get_project_version(project: &str, version: &str, loader: &Loader) -> Result<Version> {
    let version_data = reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()?
        .get(format!(
            "{MODRINTH_BASE_URL}/project/{project}/version/{version}"
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<Version>()
        .await?;

    if !version_data.loaders.contains(loader) {
        eyre::bail!(
            "{} ({}) does not support loader {}",
            project,
            version,
            loader
        );
    }

    Ok(version_data)
}
