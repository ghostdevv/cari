use color_eyre::eyre::{self, Result};
use futures::stream::{self, StreamExt, TryStreamExt};
use sha2::{Digest, Sha512};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yansi::Paint;

pub const USER_AGENT: &str = "bedrocko (+https://bedrocko.com)";

pub async fn sha512sum(path: &PathBuf) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = <Sha512 as Digest>::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub struct Downloader(Vec<Download>);

impl Downloader {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn add(&mut self, url: String, dest: PathBuf, sha512: String) {
        self.0.push(Download { url, dest, sha512 });
    }

    pub fn add_from(&mut self, downloads: &mut Vec<Download>) {
        self.0.append(downloads);
    }

    async fn download_item(&self, client: &reqwest::Client, download: &Download) -> Result<()> {
        let filename = download
            .dest
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| download.url.clone());

        println!(" {} {}", "↓".blue(), filename.dim());

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

        if hex::encode(hasher.finalize()) != download.sha512 {
            eyre::bail!("sha512 mismatch");
        }

        println!(" {} {}", "✔".green(), filename);

        Ok(())
    }

    pub async fn download(self) -> Result<()> {
        if self.0.is_empty() {
            return Ok(());
        }

        println!();
        println!("   {}", "Downloading".blue().bold().underline());

        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

        stream::iter(&self.0)
            .map(|item| self.download_item(&client, item))
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
