use crate::config::Server;
use color_eyre::eyre::Result;

pub fn run(servers: Vec<Server>, server: Option<String>) -> Result<()> {
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
            println!("  Opening {}", item.id);
            open::that(format!("https://modrinth.com/mod/{}", item.id))?;
        }
    }

    Ok(())
}
