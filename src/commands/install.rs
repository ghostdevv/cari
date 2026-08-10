use crate::{
    config::{Content, Server},
    fs::{Download, Downloader, is_managed_filename, managed_filename, sha512sum, unmanaged_path},
    modrinth,
};
use color_eyre::eyre::{self, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use heck::ToTitleCase;
use std::collections::HashSet;
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
        }

        println!(
            " {} {} {}",
            "↻".yellow(),
            name.blue().dim(),
            "(hash mismatch, redownloading)".dim()
        );

        if dry_run {
            println!("   {} {}", "→".yellow(), "would move to trash".dim());
        } else {
            trash::delete(file_path)?;
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
    let mut version =
        modrinth::get_project_version(&item.id, &item.version, &server.cfg.loader).await?;

    if version.files.len() != 1 {
        eyre::bail!("version does not have a single file {:#?}", item)
    }

    if !version.game_versions.contains(&server.cfg.game_version) {
        println!(
            "warn: {} ({}) does not support game version {}",
            version.project_id, version.id, server.cfg.game_version
        );
    }

    let file_data = version.files.remove(0);
    let file_path = content_dir.join(managed_filename(&file_data.filename));

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

async fn clean_stale(content_dir: &Path, expected: &HashSet<PathBuf>, dry_run: bool) -> Result<()> {
    if !content_dir.exists() {
        return Ok(());
    }

    let mut removed = HashSet::new();

    for dest in expected {
        let Some(base) = unmanaged_path(dest) else {
            continue;
        };

        if base.exists() && removed.insert(base.clone()) {
            let name = base
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if dry_run {
                println!(
                    "   {} {}",
                    "→".yellow(),
                    format!("would remove {name}").dim()
                );
            } else {
                trash::delete(&base)?;
                println!(" {} {}", "-".yellow(), name.blue().dim());
            }
        }
    }

    let mut entries = tokio::fs::read_dir(content_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_file() {
            continue;
        }

        let path = entry.path();
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };

        if !is_managed_filename(&name) || expected.contains(&path) {
            continue;
        }

        if removed.insert(path.clone()) {
            if dry_run {
                println!(
                    "   {} {}",
                    "→".yellow(),
                    format!("would remove {name}").dim()
                );
            } else {
                trash::delete(&path)?;
                println!(" {} {}", "-".yellow(), name.blue().dim());
            }
        }
    }

    Ok(())
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
    let mut stale = Vec::new();
    let server_count = servers.len();

    for (index, server) in servers.into_iter().enumerate() {
        println!(
            "   {}",
            server.name.to_title_case().blue().bold().underline(),
        );

        let runtime_str = server.cfg.runtime.to_string();
        let runtime_path = server.path.join(format!("{runtime_str}.jar"));

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

        let expected = downloads
            .iter()
            .map(|download| download.dest.clone())
            .collect::<HashSet<_>>();

        if dry_run {
            clean_stale(&content_dir, &expected, dry_run).await?;
        } else {
            stale.push((content_dir, expected));
        }

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

        for (content_dir, expected) in stale {
            clean_stale(&content_dir, &expected, dry_run).await?;
        }
    }

    Ok(())
}
