#![allow(dead_code)]

use crate::config::load_server;
use clap::Parser;
use color_eyre::eyre::Result;

mod commands;
mod config;
mod fs;
mod modrinth;

#[derive(clap::Parser)]
#[clap(name = "cari", about, version)]
enum Cli {
    Open {
        #[clap(short, long)]
        server: Option<String>,
    },
    Outdated {
        #[clap(short, long)]
        server: Option<String>,
    },
    Update {
        #[clap(short, long)]
        server: Option<String>,
        #[clap(long)]
        dry_run: bool,
        #[clap(long)]
        open: bool,
    },
    Install {
        #[clap(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    #[cfg(debug_assertions)]
    config::write_schema()?;

    let cli = Cli::parse();

    let servers = std::fs::read_dir("./servers")?
        .map(|ent| load_server(ent?.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    match cli {
        Cli::Open { server } => {
            commands::open::run(servers, server)?;
        }
        Cli::Outdated { server } => {
            commands::outdated::run(servers, server).await?;
        }
        Cli::Update {
            server,
            dry_run,
            open,
        } => {
            commands::update::run(servers, server, dry_run, open).await?;
        }
        Cli::Install { dry_run } => {
            commands::install::run(servers, dry_run).await?;
        }
    }

    Ok(())
}
