use crate::{
    config::{Content, Project},
    modrinth,
};
use color_eyre::eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use yansi::Paint;

async fn run_item(project: &Project, item: &Content) -> Result<()> {
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
    }

    Ok(())
}

pub async fn run(projects: Vec<Project>) -> Result<()> {
    let project_count = projects.len();

    for (index, project) in projects.into_iter().enumerate() {
        println!(
            "   {}",
            project.name.to_title_case().blue().bold().underline(),
        );

        if let Some(server) = &project.cfg.server {
            println!(
                " {} {} {}",
                "━".dim(),
                server.to_string().to_title_case().blue().dim(),
                "(todo, can't compare versions)".dim()
            );
        }

        stream::iter(&project.cfg.content)
            .map(|item| run_item(&project, item))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?;

        if index != project_count - 1 {
            println!();
        }
    }

    Ok(())
}
