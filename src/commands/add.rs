use crate::{
    config::{Content, Project},
    modrinth,
};
use color_eyre::eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use yansi::Paint;

async fn run_item(project: &Project, id: &str) -> Result<Option<Content>> {
    let cfg = &project.cfg;

    if cfg.content.iter().any(|item| item.id == id) {
        println!(
            " {} {} {}",
            "↻".yellow(),
            id.blue().dim(),
            "(already added)".dim()
        );
        return Ok(None);
    }

    let latest_version = modrinth::get_latest_project_version(id, &cfg.loader, &cfg.game_version)
        .await?
        .assert_type(&modrinth::ProjectType::Mod)
        .await?;

    println!(
        " {} {} {}",
        "+".green(),
        id.blue().dim(),
        latest_version.name.green()
    );

    Ok(Some(Content::new(id.to_string(), latest_version.id)))
}

pub async fn run(projects: Vec<Project>, items: Vec<String>) -> Result<()> {
    for mut project in projects {
        println!(
            "   {}",
            project.name.to_title_case().blue().bold().underline(),
        );

        let new_content = stream::iter(&items)
            .map(|item| run_item(&project, item))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !new_content.is_empty() {
            project.cfg.content.extend(new_content);
            project.save()?;
        }
    }

    Ok(())
}
