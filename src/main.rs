#![allow(dead_code)]

use crate::config::load_server;
use clap::Parser;
use color_eyre::eyre::Result;

mod commands;
mod config;
mod fs;
mod modrinth;

#[derive(clap::Parser)]
enum Cli {
    Open {
        #[clap(short, long)]
        server: Option<String>,
    },
    Outdated {
        #[clap(short, long)]
        server: Option<String>,
    },
    Install,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
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
        Cli::Install => {
            commands::install::run(servers).await?;
        }
    }

    Ok(())
}
