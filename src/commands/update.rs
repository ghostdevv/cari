use crate::{
    config::{Content, Project},
    modrinth,
};
use color_eyre::eyre::{OptionExt, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use yansi::Paint;

struct Update {
    id: String,
    version: String,
}

async fn run_item(project: &Project, item: &Content) -> Result<Option<Update>> {
    let latest_version = modrinth::get_latest_project_version(
        &item.id,
        &project.cfg.loader,
        &project.cfg.game_version,
    )
    .await?
    .assert_type(&modrinth::ProjectType::Mod)
    .await?;

    if latest_version.id == item.version {
        println!(
            " {} {} {}",
            "✔".green(),
            item.id.blue().dim(),
            latest_version.name
        );

        Ok(None)
    } else {
        let current_version =
            modrinth::get_project_version(&item.id, &item.version, &project.cfg.loader)
                .await?
                .assert_type(&modrinth::ProjectType::Mod)
                .await?;

        println!(
            " {} {} {} -> {}",
            "🠙".yellow(),
            item.id.blue().dim(),
            current_version.name.dim(),
            latest_version.name.green()
        );

        Ok(Some(Update {
            id: item.id.clone(),
            version: latest_version.id,
        }))
    }
}

fn apply_updates(project: &Project, updates: &[Update]) -> Result<()> {
    let path = project.path.join("cari.json");
    let raw = std::fs::read_to_string(&path)?;
    let mut value: serde_json::Value = serde_json::from_str(&raw)?;

    let content = value
        .get_mut("content")
        .and_then(|c| c.as_array_mut())
        .ok_or_eyre("cari.json missing content array")?;

    for item in content {
        let Some(id) = item.get("id").and_then(|v| v.as_str()) else {
            continue;
        };

        if let Some(update) = updates.iter().find(|u| u.id == id) {
            item["version"] = serde_json::Value::String(update.version.clone());
        }
    }

    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&value)?),
    )?;

    Ok(())
}

pub async fn run(projects: Vec<Project>, dry_run: bool, open_versions: bool) -> Result<()> {
    let project_count = projects.len();

    for (index, project) in projects.into_iter().enumerate() {
        println!(
            "   {}",
            project.name.to_title_case().blue().bold().underline(),
        );

        let updates = stream::iter(&project.cfg.content)
            .map(|item| run_item(&project, item))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !updates.is_empty() {
            if dry_run {
                println!("   {} {}", "→".yellow(), "would update cari.json".dim());
            } else {
                apply_updates(&project, &updates)?;
            }

            if open_versions {
                for update in &updates {
                    open::that(format!(
                        "https://modrinth.com/mod/{}/changelog?g={}&l={}",
                        update.id, project.cfg.game_version, project.cfg.loader
                    ))?;
                }
            }
        }

        if index != project_count - 1 {
            println!();
        }
    }

    Ok(())
}
