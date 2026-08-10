use crate::config::Server;
use color_eyre::eyre::Result;

pub fn run(servers: Vec<Server>) -> Result<()> {
    for server in servers {
        println!("Opening content for {}", server.name);

        for item in server.cfg.content {
            println!("  Opening {}", item.id);
            open::that(format!("https://modrinth.com/mod/{}", item.id))?;
        }
    }

    Ok(())
}
