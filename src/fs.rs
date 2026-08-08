use color_eyre::eyre::{self, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha512};
use std::io::Read;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub const USER_AGENT: &str = "bedrocko (+https://bedrocko.com)";

pub fn sha512sum(path: &PathBuf) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
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

    pub async fn download(self) -> Result<()> {
        if self.0.is_empty() {
            return Ok(());
        }

        let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

        for download in self.0 {
            println!("downloading {}", download.url);
            let mut stream = client
                .get(download.url)
                .send()
                .await?
                .error_for_status()?
                .bytes_stream();

            let mut file = File::create(&download.dest).await?;
            let mut hasher = Sha512::new();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                hasher.update(&chunk);
                file.write_all(&chunk).await?;
            }
            file.flush().await?;

            if hex::encode(hasher.finalize()) != download.sha512 {
                eyre::bail!("sha512 mismatch");
            }

            println!("  done");
        }

        Ok(())
    }
}

pub struct Download {
    pub url: String,
    pub dest: PathBuf,
    pub sha512: String,
}
