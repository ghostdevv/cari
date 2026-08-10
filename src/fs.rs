use color_eyre::eyre::{self, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sha2::{Digest, Sha512};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yansi::Paint;

pub const USER_AGENT: &str = concat!(
    "cari/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/ghostdevv/cari)"
);

pub const MANAGED_SUFFIX: &str = "__cari";

pub fn managed_filename(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}{MANAGED_SUFFIX}.{ext}"),
        None => format!("{filename}{MANAGED_SUFFIX}"),
    }
}

pub fn is_managed_filename(filename: &str) -> bool {
    match filename.rsplit_once('.') {
        Some((stem, _)) => stem.ends_with(MANAGED_SUFFIX),
        None => filename.ends_with(MANAGED_SUFFIX),
    }
}

pub fn unmanaged_path(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy();

    let base = match name.rsplit_once('.') {
        Some((stem, ext)) => stem
            .strip_suffix(MANAGED_SUFFIX)
            .map(|stem| format!("{stem}.{ext}")),
        None => name.strip_suffix(MANAGED_SUFFIX).map(str::to_string),
    };

    base.map(|name| path.with_file_name(name))
}

pub async fn sha512sum(path: &PathBuf) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = <Sha512 as Digest>::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(
            buffer.get(..n).ok_or_else(|| {
                eyre::eyre!("failed to get buf slice for sha512 sum at {:?}", path)
            })?,
        );
    }

    Ok(hex::encode(hasher.finalize()))
}

pub struct Downloader(Vec<Download>);

impl Downloader {
    pub const fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, url: String, dest: PathBuf, sha512: String) {
        self.0.push(Download { url, dest, sha512 });
    }

    pub fn add_from(&mut self, downloads: &mut Vec<Download>) {
        self.0.append(downloads);
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    async fn download_item(
        &self,
        client: &reqwest::Client,
        download: &Download,
        multi: &MultiProgress,
    ) -> Result<()> {
        let filename = download.dest.file_name().map_or_else(
            || download.url.clone(),
            |name| name.to_string_lossy().to_string(),
        );

        let pb = multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::with_template(" {spinner:.blue} {msg}")?.tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏⠋"),
        );
        pb.set_message(filename.clone());
        pb.enable_steady_tick(Duration::from_millis(80));

        let result = self.download_file(client, download).await;

        pb.set_style(ProgressStyle::with_template(" {msg}")?);
        match &result {
            Ok(()) => pb.finish_with_message(format!("{} {}", "✔".green(), filename)),
            Err(_) => pb.finish_with_message(format!("{} {}", "✗".red(), filename)),
        }

        result
    }

    async fn download_file(&self, client: &reqwest::Client, download: &Download) -> Result<()> {
        let mut stream = client
            .get(&download.url)
            .send()
            .await?
            .error_for_status()?
            .bytes_stream();

        let mut file = File::create(&download.dest).await?;
        let mut hasher = <Sha512 as Digest>::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.flush().await?;

        let digest = hex::encode(hasher.finalize());
        if digest != download.sha512 {
            eyre::bail!(
                "sha512 mismatch for {} (expected {}, downloaded {})",
                download
                    .dest
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                download.sha512,
                digest
            );
        }

        Ok(())
    }

    pub async fn download(self) -> Result<()> {
        if self.0.is_empty() {
            return Ok(());
        }

        println!();
        println!("   {}", "Downloading".blue().bold().underline());

        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
        let multi = MultiProgress::new();

        stream::iter(&self.0)
            .map(|item| self.download_item(&client, item, &multi))
            .buffer_unordered(8)
            .try_collect::<Vec<_>>()
            .await?;

        Ok(())
    }
}

pub struct Download {
    pub url: String,
    pub dest: PathBuf,
    pub sha512: String,
}
