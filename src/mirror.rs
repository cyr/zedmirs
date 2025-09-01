use std::{os::unix::fs::MetadataExt, path::PathBuf};

use anyhow::{bail, Context};
use clap::{arg, Parser};
use tokio::io::{AsyncReadExt, BufReader};

use crate::{downloader::{Download, Downloader}, index::Indexer, package_meta::ExtensionListData, progress::spawn_updater};

const MAX_SCHEMA_VERSION: i32 = 1_i32;

#[derive(Clone, Parser)]
pub struct MirrorOpts {
    #[arg(short, long, default_value="https://api.zed.dev",
        help="Zed API url")]
    pub api_url: String,
    #[arg(short, long, default_value_t=8u8)]
    pub dl_threads: u8,
}

pub struct MirrorCtx {
    pub tmp_path: String,
    pub downloader: Downloader,
}

impl MirrorCtx {
    pub async fn init(opts: &MirrorOpts, mut output: &str) -> anyhow::Result<Self> {
        if let Some(path) = output.strip_suffix('/') {
            output = path
        }

        let tmp_path = format!("{output}/.tmp");
        
        tokio::fs::create_dir_all(&tmp_path).await?;

        let downloader = Downloader::build(opts.dl_threads);

        Ok(Self {
            tmp_path,
            downloader
        })
    }
}

pub async fn mirror(opts: &MirrorOpts, output: &str) -> anyhow::Result<()> {
    crate::log("Mirroring started");

    let ctx = MirrorCtx::init(opts, output).await
        .with_context(|| "initializing mirror context")?;

    let progress = ctx.downloader.progress();

    progress.set_total_steps(3);
    progress.next_step("Downloading metadata").await;

    let ext_path = download_extension_list(&ctx, opts).await
        .with_context(|| "downloading extension list")?;

    progress.next_step("Downloading extensions").await;

    let ext_list = download_extensions(&ctx, opts, output, ext_path).await
        .with_context(|| "downloading extensions")?;

    progress.next_step("Generating index").await;

    generate_index(&ctx, output, ext_list).await
        .with_context(|| "generating index")?;

    promote_tmp(&ctx, output).await
        .with_context(|| "finishing up")?;

    crate::log("Mirroring completed");

    Ok(())
}

async fn promote_tmp(ctx: &MirrorCtx, output: &str) -> anyhow::Result<()> {
    let new_meta = PathBuf::from(format!("{}/extensions.json", ctx.tmp_path));

    let current_meta = PathBuf::from(format!("{output}/extensions.json"));

    if tokio::fs::try_exists(&current_meta).await? {
        tokio::fs::remove_file(&current_meta).await?;
    }

    tokio::fs::rename(new_meta, current_meta).await?;

    let new_idx = PathBuf::from(format!("{}/idx", ctx.tmp_path));
    let current_idx = PathBuf::from(format!("{output}/idx"));

    if tokio::fs::try_exists(&current_idx).await? {
        tokio::fs::remove_dir_all(&current_idx).await?;
    }
    
    tokio::fs::rename(new_idx, current_idx).await?;

    Ok(())
}

async fn generate_index(ctx: &MirrorCtx, output: &str, ext_list: ExtensionListData) -> anyhow::Result<()> {
    let indexer = match Indexer::init(output).await {
        Ok(indexer) => indexer,
        Err(e) => {
            crate::log(format!("{e:?}"));
            return Err(e)
        }
    };
    
    let progress = ctx.downloader.progress();
    progress.files.inc_total(ext_list.data.len() as u64);

    let pb = progress.create_download_progress_bar().await;

    let updater = spawn_updater(vec![(progress.clone(), pb.clone())]).await;

    indexer.index(ext_list, progress)
        .with_context(|| "indexing documents")?;

    updater.abort();

    Ok(())
}

async fn download_extensions(ctx: &MirrorCtx, opts: &MirrorOpts, output: &str, new_extension_file: String) -> anyhow::Result<ExtensionListData> {
    let file = tokio::fs::File::open(new_extension_file).await?;

    let size = file.metadata().await?.size();

    let mut reader = BufReader::new(file);
    
    let mut buf = Vec::with_capacity(size as usize);
    
    _ = reader.read_to_end(&mut buf).await?;

    let extension_list: ExtensionListData = serde_json::from_slice(&buf)?;

    let progress = ctx.downloader.progress();

    let pb = progress.create_download_progress_bar().await;

    let updater = spawn_updater(vec![(progress.clone(), pb.clone())]).await;

    for extension in &extension_list.data {
        let Some(id) = extension.get("id").and_then(|v| v.as_str()) else {
            bail!("document lacks string id field")
        };

        let Some(version) = extension.get("version").and_then(|v| v.as_str()) else {
            bail!("document lacks string version field")
        };

        let dl = Box::new(Download {
            url: format!("{}/extensions/{}/{}/download", opts.api_url, id, version),
            size: None,
            primary_target_path: format!("{output}/extensions/{}/{}/archive.tar.gz", id, version),
            always_download: false,
            symlink_path: Some(format!("{output}/extensions/{}/archive.tar.gz", id))
        });

        ctx.downloader.queue(dl).await?;
    }

    progress.wait_for_completion(&pb).await;

    updater.abort();

    Ok(extension_list)
}

async fn download_extension_list(ctx: &MirrorCtx, opts: &MirrorOpts) -> anyhow::Result<String> {
    let progress = ctx.downloader.progress();

    let pb = progress.create_download_no_size_progress_bar().await;

    let updater = spawn_updater(vec![(progress.clone(), pb.clone())]).await;

    let new_extensions_path = format!("{}/extensions.json", &ctx.tmp_path);

    let dl = Box::new(Download {
        url: format!("{}/extensions?max_schema_version={MAX_SCHEMA_VERSION}", &opts.api_url),
        size: None,
        primary_target_path: new_extensions_path.clone(),
        always_download: true,
        symlink_path: None
    });

    ctx.downloader.queue(dl).await
        .with_context(|| "queuing download")?;

    progress.wait_for_completion(&pb).await;

    updater.abort();

    Ok(new_extensions_path)
}
