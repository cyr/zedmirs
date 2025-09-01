
use std::{path::{Path, PathBuf}, sync::Arc};

use anyhow::{bail, Context};
use async_channel::{bounded, Sender, Receiver};
use reqwest::{Client, StatusCode};
use tokio::{fs::{remove_file, symlink}, io::AsyncWriteExt, task::JoinHandle};

use super::progress::Progress;

#[derive(Clone)]
pub struct Downloader {
    sender: Sender<Box<Download>>,
    _tasks: Arc<Vec<JoinHandle<()>>>,
    progress: Progress
}

impl Default for Downloader {
    fn default() -> Self {
        let (sender, _) = bounded(1);
        Self {
            sender,
            _tasks: Default::default(),
            progress: Default::default()
        }
    }
}

impl Downloader {
    pub fn build(num_threads: u8) -> Self {
        let (sender, receiver) = bounded(1024);

        let mut tasks = Vec::with_capacity(num_threads as usize);
        let progress = Progress::new();
        let http_client = reqwest::Client::new();

        for _ in 0..num_threads {
            let task_receiver: Receiver<Box<Download>> = receiver.clone();
            let task_progress = progress.clone();
            let task_http_client = http_client.clone();

            let handle = tokio::spawn(async move {
                while let Ok(dl) = task_receiver.recv().await {
                    _ = Downloader::download_and_track(&task_http_client, task_progress.clone(), dl).await;
                }
            });

            tasks.push(handle);
        }

        Self {
            sender,
            _tasks: Arc::new(tasks),
            progress
        }
    }

    pub async fn queue(&self, download_entry: Box<Download>) -> anyhow::Result<()> {
        if let Some(size) = download_entry.size {
            self.progress.bytes.inc_total(size);
        }

        self.progress.files.inc_total(1);

        self.sender.send(download_entry).await?;

        Ok(())
    }

    async fn download_and_track(http_client: &Client, progress: Progress, dl: Box<Download>) -> anyhow::Result<()> {
        match download_file(http_client, dl, 
            |downloaded| progress.bytes.inc_success(downloaded)
        ).await {
            Ok(true) => progress.files.inc_success(1),
            Ok(false) => progress.files.inc_skipped(1),
            Err(_) => progress.files.inc_skipped(1),
        }

        Ok(())
    }

    pub fn progress(&self) -> Progress {
        self.progress.clone()
    }
}

async fn download_file<F>(http_client: &Client, download: Box<Download>, mut progress_cb: F) -> anyhow::Result<bool>
    where F: FnMut(u64) {
    
    let mut downloaded = false;

    if needs_downloading(&download) {
        create_dirs(&download.primary_target_path).await?;

        let mut output = tokio::fs::File::create(&download.primary_target_path).await?;

        if download.size.is_some_and(|v| v > 0) || download.size.is_none() {
            let mut response = http_client.get(download.url.as_str()).send().await
                .with_context(|| format!("downloading {}", download.url.clone()))?;

            if response.status() == StatusCode::NOT_FOUND {
                drop(output);
                tokio::fs::remove_file(&download.primary_target_path).await?;

                bail!("{}: 404", download.url.clone())
            }

            while let Some(chunk) = response.chunk().await? {
                output.write_all(&chunk).await?;
        
                progress_cb(chunk.len() as u64);
            }
        
            output.flush().await?;
            downloaded = true;
        }

        if let Some(symlink_path) = &download.symlink_path {
            create_dirs(symlink_path).await?;
            
            let symlink_path = PathBuf::from(symlink_path);

            let rel_primary_path = pathdiff::diff_paths(
                &download.primary_target_path,
                symlink_path.parent().expect("base dir needs to exist"),
            ).expect("all files will be in some relative path");

            _ = remove_file(&symlink_path).await;

            symlink(&rel_primary_path, symlink_path).await?;
        }
    }
    
    Ok(downloaded)
}

pub async fn create_dirs<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    if let Some(parent_dir) = path.as_ref().parent() && !parent_dir.exists()  {
        tokio::fs::create_dir_all(parent_dir).await?;
    }

    Ok(())
}

fn needs_downloading(dl: &Download) -> bool {
    if dl.always_download {
        return true
    }

    let Some(exists) = std::fs::exists(&dl.primary_target_path).ok() else {
        return true
    };

    !exists
}

#[derive(Debug)]
pub struct Download {
    pub url: String,
    pub size: Option<u64>,
    pub primary_target_path: String,
    pub symlink_path: Option<String>,
    pub always_download: bool,
}