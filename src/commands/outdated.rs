use crate::{config::Server, modrinth};
use color_eyre::eyre::Result;
use heck::ToTitleCase;
use yansi::Paint;

pub async fn run(servers: Vec<Server>, server: Option<String>) -> Result<()> {
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

        for item in server.cfg.content {
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
                )
            } else {
                let current_version = modrinth::get_project_version(
                    &item.id,
                    &item.version,
                    &server.cfg.loader,
                    &server.cfg.game_version,
                )
                .await?;

                println!(
                    " {} {} {} -> {}",
                    "🠙".yellow(),
                    item.id.blue().dim(),
                    current_version.name.dim(),
                    latest_version.name.green()
                )
            }
        }

        if index != server_count - 1 {
            println!();
        }
    }

    Ok(())
}
