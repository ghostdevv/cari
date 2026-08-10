use crate::{
    config::{Content, Server},
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

async fn run_item(server: &Server, item: &Content) -> Result<Option<Update>> {
    let latest_version = modrinth::get_latest_project_version(
        &item.id,
        &server.cfg.loader,
        &server.cfg.game_version,
    )
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
            modrinth::get_project_version(&item.id, &item.version, &server.cfg.loader).await?;

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

fn apply_updates(server: &Server, updates: &[Update]) -> Result<()> {
    let path = server.path.join("cari.json");
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

pub async fn run(
    servers: Vec<Server>,
    server: Option<String>,
    dry_run: bool,
    open_versions: bool,
) -> Result<()> {
    let servers = match server {
        Some(server) => servers
            .into_iter()
            .filter(|s| s.name == server)
            .collect::<Vec<_>>(),
        None => servers,
    };

    let server_count = servers.len();

    for (index, server) in servers.into_iter().enumerate() {
        println!(
            "   {}",
            server.name.to_title_case().blue().bold().underline(),
        );

        let updates = stream::iter(&server.cfg.content)
            .map(|item| run_item(&server, item))
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
                apply_updates(&server, &updates)?;
            }

            if open_versions {
                for update in &updates {
                    open::that(format!(
                        "https://modrinth.com/mod/{}/changelog?g={}&l={}",
                        update.id, server.cfg.game_version, server.cfg.loader
                    ))?;
                }
            }
        }

        if index != server_count - 1 {
            println!();
        }
    }

    Ok(())
}
