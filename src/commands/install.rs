use crate::{
    config::Server,
    fs::{Downloader, sha512sum},
    modrinth,
};
use color_eyre::eyre::{self, Result};

pub async fn run(servers: Vec<Server>) -> Result<()> {
    for server in servers {
        let content_dir = server.content_dir();

        if !content_dir.exists() {
            std::fs::create_dir_all(&content_dir)?;
        }

        let mut downloader = Downloader::new();

        for item in &server.cfg.content {
            let mut version = modrinth::get_project_version(
                &item.id,
                &item.version,
                &server.cfg.loader,
                &server.cfg.game_version,
            )
            .await?;

            if version.files.len() != 1 {
                eyre::bail!("version does not have a single file {:#?}", item)
            }

            let file_data = version.files.remove(0);
            let file_path = content_dir.join(&file_data.filename);

            if file_path.exists() {
                let sum = sha512sum(&file_path)?;

                if sum == file_data.hashes.sha512 {
                    println!("Skipping {} (sha512 sum matches)", file_data.filename);
                    continue;
                } else {
                    println!(
                        "Trashing {} as sha512 sum doesn't match",
                        file_data.filename
                    );
                    trash::delete(&file_path)?;
                }
            }

            downloader.add(file_data.url, file_path, file_data.hashes.sha512)
        }

        downloader.download().await?;
    }

    Ok(())
}
