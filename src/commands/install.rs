use crate::{
    config::{Content, Project},
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
    project: &Project,
    item: &Content,
    content_dir: &Path,
    dry_run: bool,
) -> Result<(PathBuf, Option<Download>)> {
    let mut version = modrinth::get_project_version(&item.id, &item.version, &project.cfg.loader)
        .await?
        .assert_type(&modrinth::ProjectType::Mod)
        .await?;

    if version.files.len() != 1 {
        eyre::bail!("version does not have a single file {:#?}", item)
    }

    if !version.game_versions.contains(&project.cfg.game_version) {
        println!(
            "warn: {} ({}) does not support game version {}",
            version.project_id, version.id, project.cfg.game_version
        );
    }

    let file_data = version.files.remove(0);
    let file_path = content_dir.join(managed_filename(&file_data.filename));

    let download =
        if should_download(&item.id, &file_path, &file_data.hashes.sha512, dry_run).await? {
            Some(Download {
                url: file_data.url,
                dest: file_path.clone(),
                sha512: file_data.hashes.sha512,
            })
        } else {
            None
        };

    Ok((file_path, download))
}

async fn clean_stale(content_dir: &Path, expected: &HashSet<PathBuf>, dry_run: bool) -> Result<()> {
    if !content_dir.exists() {
        return Ok(());
    }

    let mut removed = HashSet::new();

    for dest in expected {
        if !dest.exists() {
            continue;
        }

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

pub async fn run(projects: Vec<Project>, dry_run: bool) -> Result<()> {
    if dry_run {
        println!(
            "   {}",
            "Dry run (no files will be downloaded)".yellow().bold()
        );
        println!();
    }

    let mut downloader = Downloader::new();
    let mut stale = Vec::new();
    let project_count = projects.len();

    for (index, project) in projects.into_iter().enumerate() {
        println!(
            "   {}",
            project.name.to_title_case().blue().bold().underline(),
        );

        if let Some(server) = &project.cfg.server {
            let server_str = server.to_string();
            let server_path = project.path.join(format!("{server_str}.jar"));

            if should_download(&server_str, &server_path, server.sha512(), dry_run).await? {
                downloader.add(
                    server.to_download_url(&project.cfg.game_version),
                    server_path,
                    server.sha512().to_owned(),
                );
            }
        }

        let content_dir = project.content_dir();

        if !content_dir.exists() && !dry_run {
            std::fs::create_dir_all(&content_dir)?;
        }

        let items = stream::iter(&project.cfg.content)
            .map(|item| run_item(&project, item, &content_dir, dry_run))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?;

        let expected = items
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<HashSet<_>>();

        let mut downloads = items
            .into_iter()
            .filter_map(|(_, download)| download)
            .collect::<Vec<_>>();

        downloader.add_from(&mut downloads);

        if dry_run {
            clean_stale(&content_dir, &expected, dry_run).await?;
        } else {
            stale.push((content_dir, expected));
        }

        if index != project_count - 1 {
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
