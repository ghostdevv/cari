use crate::{
    config::{Content, Server},
    fs::{Download, Downloader, sha512sum},
    modrinth,
};
use color_eyre::eyre::{self, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use std::path::{Path, PathBuf};
use yansi::Paint;

async fn should_download(
    name: &str,
    file_path: &PathBuf,
    sha512: &str,
    dry_run: bool,
) -> Result<bool> {
    if file_path.exists() {
        let sum = sha512sum(file_path).await?;

        if sum == sha512 {
            println!(" {} {}", "✔".green(), name.blue().dim());
            return Ok(false);
        } else {
            println!(
                " {} {} {}",
                "↻".yellow(),
                name.blue().dim(),
                "(hash mismatch, redownloading)".dim()
            );
            if !dry_run {
                trash::delete(file_path)?;
            } else {
                println!("   {} {}", "→".yellow(), "would move to trash".dim());
            }
        }
    } else {
        println!(" {} {}", "+".green(), name.blue().dim());
    }

    Ok(true)
}

async fn run_item(
    server: &Server,
    item: &Content,
    content_dir: &Path,
    dry_run: bool,
) -> Result<Vec<Download>> {
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

    if should_download(&item.id, &file_path, &file_data.hashes.sha512, dry_run).await? {
        downloads.push(Download {
            url: file_data.url,
            dest: file_path,
            sha512: file_data.hashes.sha512,
        });
    }

    Ok(downloads)
}

pub async fn run(servers: Vec<Server>, dry_run: bool) -> Result<()> {
    if dry_run {
        println!(
            "   {}",
            "Dry run (no files will be downloaded)".yellow().bold()
        );
        println!();
    }

    let mut downloader = Downloader::new();
    let server_count = servers.len();

    for (index, server) in servers.into_iter().enumerate() {
        println!(
            "   {}",
            server.name.to_title_case().blue().bold().underline(),
        );

        let runtime_str = server.cfg.runtime.to_string();
        let runtime_path = server.path.join(format!("{}.jar", runtime_str));

        if should_download(
            &runtime_str,
            &runtime_path,
            server.cfg.runtime.sha512(),
            dry_run,
        )
        .await?
        {
            downloader.add(
                server.cfg.runtime.to_download_url(&server.cfg.game_version),
                runtime_path,
                server.cfg.runtime.sha512().to_owned(),
            );
        }

        let content_dir = server.content_dir();

        if !content_dir.exists() && !dry_run {
            std::fs::create_dir_all(&content_dir)?;
        }

        let mut downloads = stream::iter(&server.cfg.content)
            .map(|item| run_item(&server, item, &content_dir, dry_run))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        downloader.add_from(&mut downloads);

        if index != server_count - 1 {
            println!();
        }
    }

    if dry_run {
        let count = downloader.len();
        println!();
        if count == 0 {
            println!("   {}", "Nothing to download".green().bold());
        } else {
            println!(
                "   {} {}",
                count.to_string().blue().bold(),
                if count == 1 {
                    "file would be downloaded"
                } else {
                    "files would be downloaded"
                }
            );
        }
    } else {
        downloader.download().await?;
    }

    Ok(())
}
