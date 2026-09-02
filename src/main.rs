use crate::config::{Project, load_project};
use clap::Parser;
use color_eyre::eyre::{self, Result};

mod commands;
mod config;
mod fs;
mod modrinth;

#[derive(clap::Parser)]
#[clap(name = "cari", about, version)]
enum Cli {
    #[clap(about = "Open all content on modrinth")]
    Open {
        #[clap(short, long)]
        project: Option<Vec<String>>,
    },
    #[clap(about = "List outdated content")]
    Outdated {
        #[clap(short, long)]
        project: Option<Vec<String>>,
    },
    #[clap(about = "Update content to latest version(s)")]
    Update {
        #[clap(short, long)]
        project: Option<Vec<String>>,
        #[clap(long)]
        dry_run: bool,
        #[clap(long)]
        open: bool,
    },
    #[clap(about = "Add new content to your config")]
    Add {
        #[clap(short, long)]
        project: Option<Vec<String>>,
        #[clap(required = true)]
        items: Vec<String>,
    },
    #[clap(about = "Install all content")]
    Install {
        #[clap(long)]
        dry_run: bool,
    },
    #[clap(about = "Initialize a new cari project")]
    Init,
    #[clap(about = "Import content from a modpack")]
    Import {
        #[clap(short, long)]
        project: Option<Vec<String>>,
        #[clap(required = true)]
        item: String,
    },
}

#[allow(clippy::needless_pass_by_value)]
fn load_projects(filter: Option<Vec<String>>) -> Result<Vec<Project>> {
    let projects = std::fs::read_dir("./servers")?
        .map(|ent| load_project(ent?.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter(|s| filter.as_deref().is_none_or(|f| f.contains(&s.name)))
        .collect::<Vec<_>>();

    if projects.is_empty() {
        eyre::bail!("no projects found :((")
    }

    Ok(projects)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    #[cfg(debug_assertions)]
    config::write_schema()?;

    let cli = Cli::parse();

    match cli {
        Cli::Init => {
            commands::init::run(&std::env::current_dir()?)?;
        }
        Cli::Open { project } => {
            commands::open::run(load_projects(project)?)?;
        }
        Cli::Outdated { project } => {
            commands::outdated::run(load_projects(project)?).await?;
        }
        Cli::Update {
            project,
            dry_run,
            open,
        } => {
            commands::update::run(load_projects(project)?, dry_run, open).await?;
        }
        Cli::Add { project, items } => {
            commands::add::run(load_projects(project)?, items).await?;
        }
        Cli::Install { dry_run } => {
            commands::install::run(load_projects(None)?, dry_run).await?;
        }
        Cli::Import { project, item } => {
            commands::import::run(load_projects(project)?, item).await?;
        }
    }

    Ok(())
}
