use crate::{config::Server, modrinth};
use color_eyre::eyre::{OptionExt, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use yansi::Paint;

struct Add {
    id: String,
    version: String,
}

async fn run_item(server: &Server, id: &str) -> Result<Option<Add>> {
    let cfg = &server.cfg;

    if cfg.content.iter().any(|item| item.id == id) {
        println!(
            " {} {} {}",
            "↻".yellow(),
            id.blue().dim(),
            "(already added)".dim()
        );
        return Ok(None);
    }

    let latest_version =
        modrinth::get_latest_project_version(id, &cfg.loader, &cfg.game_version).await?;

    println!(
        " {} {} {}",
        "+".green(),
        id.blue().dim(),
        latest_version.name.green()
    );

    Ok(Some(Add {
        id: id.to_string(),
        version: latest_version.id,
    }))
}

fn apply_adds(server: &Server, adds: &[Add]) -> Result<()> {
    let path = server.path.join("cari.json");
    let raw = std::fs::read_to_string(&path)?;
    let mut value: serde_json::Value = serde_json::from_str(&raw)?;

    let content = value
        .get_mut("content")
        .and_then(|c| c.as_array_mut())
        .ok_or_eyre("cari.json missing content array")?;

    for add in adds {
        content.push(serde_json::json!({
            "id": add.id,
            "version": add.version,
        }));
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
    projects: Vec<String>,
) -> Result<()> {
    let servers = match server {
        Some(server) => servers
            .into_iter()
            .filter(|s| s.name == server)
            .collect::<Vec<_>>(),
        None => servers,
    };

    for server in servers {
        println!(
            "   {}",
            server.name.to_title_case().blue().bold().underline(),
        );

        let adds = stream::iter(&projects)
            .map(|project| run_item(&server, project))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !adds.is_empty() {
            apply_adds(&server, &adds)?;
        }
    }

    Ok(())
}
