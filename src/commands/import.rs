use crate::{
    config::{self, Project},
    modrinth,
};
use color_eyre::eyre::{OptionExt, Result, bail};
use yansi::Paint;

async fn run_project(project: &mut Project, item: &str) -> Result<()> {
    let item = modrinth::get_project(item)
        .await?
        .assert_type(&modrinth::ProjectType::Modpack)?;

    let version = modrinth::get_latest_project_version(
        &item.id,
        &project.cfg.loader,
        &project.cfg.game_version,
    )
    .await?;

    println!(
        "Importing {} {} to {}",
        item.title.blue(),
        format!("({})", version.id).dim(),
        project.name.blue()
    );

    for data in version.dependencies {
        if data.dependency_type == Some(modrinth::DependencyType::Incompatible) {
            bail!("unclear how to handle \"incompatible\" dependency type");
        }

        if let Some(file_name) = &data.file_name {
            println!(
                " {} skipped file \"{}\" as unable to handle yet",
                "━".dim(),
                file_name
            );
            continue;
        }

        // the err shouldn't happen as when the `file_name` is None, these are set
        let project_id = data.project_id.ok_or_eyre("somehow missing id")?;
        let version_id = data.version_id.ok_or_eyre("somehow missing id")?;

        let m_project = modrinth::get_project(&project_id).await?;
        let project_id = m_project.slug.as_deref().unwrap_or(&project_id);

        if m_project.project_type != modrinth::ProjectType::Mod {
            println!(
                " {} skipped {} as we can't handle {}s yet",
                "━".dim(),
                m_project.title,
                m_project.project_type
            );
            continue;
        }

        let version =
            modrinth::get_project_version(project_id, &version_id, &project.cfg.loader).await?;

        // todo this fails if not using slug id
        if project.cfg.content.iter().any(|item| item.id == project_id) {
            println!(
                " {} {} {}",
                "↻".yellow(),
                project_id.blue().dim(),
                "(already added)".dim()
            );
            continue;
        }

        println!(
            " {} {} {}",
            "+".green(),
            project_id.blue().dim(),
            version.name.green()
        );

        project
            .cfg
            .content
            .push(config::Content::new(project_id.to_string(), version_id));
    }

    project.save()?;
    Ok(())
}

pub async fn run(projects: Vec<Project>, item: String) -> Result<()> {
    for mut project in projects {
        run_project(&mut project, &item).await?;
    }

    Ok(())
}
