#![allow(dead_code)]

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
    Open {
        #[clap(short, long)]
        project: Option<Vec<String>>,
    },
    Outdated {
        #[clap(short, long)]
        project: Option<Vec<String>>,
    },
    Update {
        #[clap(short, long)]
        project: Option<Vec<String>>,
        #[clap(long)]
        dry_run: bool,
        #[clap(long)]
        open: bool,
    },
    Add {
        #[clap(short, long)]
        project: Option<Vec<String>>,
        #[clap(required = true)]
        items: Vec<String>,
    },
    Install {
        #[clap(long)]
        dry_run: bool,
    },
    Init,
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
    }

    Ok(())
}
