#![allow(dead_code)]

use crate::config::{Server, load_server};
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
        server: Option<Vec<String>>,
    },
    Outdated {
        #[clap(short, long)]
        server: Option<Vec<String>>,
    },
    Update {
        #[clap(short, long)]
        server: Option<Vec<String>>,
        #[clap(long)]
        dry_run: bool,
        #[clap(long)]
        open: bool,
    },
    Add {
        #[clap(short, long)]
        server: Option<Vec<String>>,
        #[clap(required = true)]
        projects: Vec<String>,
    },
    Install {
        #[clap(long)]
        dry_run: bool,
    },
    Init,
}

#[allow(clippy::needless_pass_by_value)]
fn load_servers(filter: Option<Vec<String>>) -> Result<Vec<Server>> {
    let servers = std::fs::read_dir("./servers")?
        .map(|ent| load_server(ent?.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter(|s| filter.as_deref().is_none_or(|f| f.contains(&s.name)))
        .collect::<Vec<_>>();

    if servers.is_empty() {
        eyre::bail!("no servers found :((")
    }

    Ok(servers)
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
        Cli::Open { server } => {
            commands::open::run(load_servers(server)?)?;
        }
        Cli::Outdated { server } => {
            commands::outdated::run(load_servers(server)?).await?;
        }
        Cli::Update {
            server,
            dry_run,
            open,
        } => {
            commands::update::run(load_servers(server)?, dry_run, open).await?;
        }
        Cli::Add { server, projects } => {
            commands::add::run(load_servers(server)?, projects).await?;
        }
        Cli::Install { dry_run } => {
            commands::install::run(load_servers(None)?, dry_run).await?;
        }
    }

    Ok(())
}
