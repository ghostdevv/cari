use crate::{
    config::{Content, Project},
    modrinth,
};
use color_eyre::eyre::Result;
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use yansi::Paint;

async fn run_item(project: &Project, item: &Content) -> Result<Option<Content>> {
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

        Ok(Some(Content::new(item.id.clone(), latest_version.id)))
    }
}

pub async fn run(projects: Vec<Project>, dry_run: bool, open_versions: bool) -> Result<()> {
    let project_count = projects.len();

    for (index, mut project) in projects.into_iter().enumerate() {
        println!(
            "   {}",
            project.name.to_title_case().blue().bold().underline(),
        );

        let changes = stream::iter(&project.cfg.content)
            .map(|item| run_item(&project, item))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if !changes.is_empty() {
            if open_versions {
                for update in &changes {
                    open::that(format!(
                        "https://modrinth.com/mod/{}/changelog?g={}&l={}",
                        update.id, project.cfg.game_version, project.cfg.loader
                    ))?;
                }
            }

            if dry_run {
                println!("   {} {}", "→".yellow(), "would update cari.json".dim());
            } else {
                for update in changes {
                    let found = project
                        .cfg
                        .content
                        .iter_mut()
                        .find(|item| item.id == update.id);

                    if let Some(found) = found {
                        found.version = update.version;
                    } else {
                        project.cfg.content.push(update);
                    }
                }

                project.save()?;
            }
        }

        if index != project_count - 1 {
            println!();
        }
    }

    Ok(())
}
