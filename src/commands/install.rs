use crate::{
    config::{Content, Server},
    fs::{Download, Downloader, sha512sum},
    modrinth,
};
use color_eyre::eyre::{self, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use std::path::Path;

async fn run_item(server: &Server, item: &Content, content_dir: &Path) -> Result<Vec<Download>> {
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
    let mut downloads = Vec::new();

    if file_path.exists() {
        let sum = sha512sum(&file_path).await?;

        if sum == file_data.hashes.sha512 {
            println!("Skipping {} (sha512 sum matches)", file_data.filename);
            return Ok(Vec::with_capacity(0));
        } else {
            println!(
                "Trashing {} as sha512 sum doesn't match",
                file_data.filename
            );
            trash::delete(&file_path)?;
        }
    }

    downloads.push(Download {
        url: file_data.url,
        dest: file_path,
        sha512: file_data.hashes.sha512,
    });

    Ok(downloads)
}

pub async fn run(servers: Vec<Server>) -> Result<()> {
    let mut downloader = Downloader::new();

    for server in servers {
        let content_dir = server.content_dir();

        if !content_dir.exists() {
            std::fs::create_dir_all(&content_dir)?;
        }

        let mut downloads = stream::iter(&server.cfg.content)
            .map(|item| run_item(&server, item, &content_dir))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        downloader.add_from(&mut downloads);
    }

    downloader.download().await?;

    Ok(())
}
