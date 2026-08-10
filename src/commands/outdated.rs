use crate::{
    config::{Content, Server},
    modrinth,
};
use color_eyre::eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use yansi::Paint;

async fn run_item(server: &Server, item: &Content) -> Result<()> {
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
    }

    Ok(())
}

pub async fn run(servers: Vec<Server>) -> Result<()> {
    let server_count = servers.len();

    for (index, server) in servers.into_iter().enumerate() {
        println!(
            "   {}",
            server.name.to_title_case().blue().bold().underline(),
        );

        println!(
            " {} {} {}",
            "━".dim(),
            server.cfg.runtime.to_string().to_title_case().blue().dim(),
            "(todo, can't compare versions)".dim()
        );

        stream::iter(&server.cfg.content)
            .map(|item| run_item(&server, item))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?;

        if index != server_count - 1 {
            println!();
        }
    }

    Ok(())
}
