use crate::config::load_server;
use clap::Parser;
use color_eyre::eyre::Result;
use heck::ToTitleCase;
use yansi::Paint;

mod config;
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
        .filter_map(|c| c)
        .collect::<Vec<_>>();

    match cli {
        Cli::Open { server } => {
            let servers = match server {
                Some(server) => servers
                    .into_iter()
                    .filter(|s| s.name == server)
                    .collect::<Vec<_>>(),
                None => servers,
            };

            for server in servers {
                println!("Opening content for {}", server.name);

                for item in server.cfg.content {
                    if !item.skip {
                        println!("  Opening {}", item.id);
                        open::that(format!("https://modrinth.com/mod/{}", item.id))?;
                    }
                }
            }
        }
        Cli::Outdated { server } => {
            let servers = match server {
                Some(server) => servers
                    .into_iter()
                    .filter(|s| s.name == server)
                    .collect::<Vec<_>>(),
                None => servers,
            };

            let server_count = servers.len();

            for (index, server) in servers.into_iter().enumerate() {
                println!(
                    "   {}",
                    server.name.to_title_case().blue().bold().underline(),
                );

                for item in server.cfg.content {
                    if item.skip {
                        continue;
                    }

                    let latest_version = modrinth::get_latest_project_version(
                        &item.id,
                        &server.cfg.loader,
                        &server.cfg.game_version,
                    )
                    .await?;

                    if latest_version.id == item.version {
                        println!(
                            " {} {} {}",
                            "✔".green(),
                            item.id.blue().dim(),
                            latest_version.name
                        )
                    } else {
                        let current_version = modrinth::get_project_version(
                            &item.id,
                            &item.version,
                            &server.cfg.loader,
                            &server.cfg.game_version,
                        )
                        .await?;

                        println!(
                            " {} {} {} -> {}",
                            "🠙".yellow(),
                            item.id.blue().dim(),
                            current_version.name.dim(),
                            latest_version.name.green()
                        )
                    }
                }

                if index != server_count - 1 {
                    println!("");
                }
            }
        }
    }

    Ok(())
}
