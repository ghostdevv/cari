use crate::config::Project;
use color_eyre::eyre::Result;

pub fn run(projects: Vec<Project>) -> Result<()> {
    for project in projects {
        println!("Opening content for {}", project.name);

        for item in project.cfg.content {
            println!("  Opening {}", item.id);
            open::that(format!("https://modrinth.com/mod/{}", item.id))?;
        }
    }

    Ok(())
}
