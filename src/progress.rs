use std::{fmt::Display, sync::{atomic::{AtomicU64, AtomicU8, Ordering}, Arc}, time::Duration};

use compact_str::ToCompactString;
use console::{style, pad_str};
use indicatif::{ProgressBar, ProgressStyle, ProgressFinish, HumanBytes};
use tokio::{sync::Mutex, task::JoinHandle, time::sleep};

#[derive(Clone, Default)]
pub struct Progress {
    pub step: Arc<AtomicU8>,
    step_name: Arc<Mutex<String>>,
    pub files: ProgressPart,
    pub bytes: ProgressPart,
    pub total_bytes: Arc<AtomicU64>,
    total_steps: Arc<AtomicU8>
}

impl Progress {
    pub fn new() -> Self {
        Self {
            step_name: Arc::new(Mutex::new(String::new())),
            step: Arc::new(AtomicU8::new(0)),
            files: ProgressPart::new(),
            bytes: ProgressPart::new(),
            total_bytes: Arc::new(AtomicU64::new(0)),
            total_steps: Arc::new(AtomicU8::new(4))
        }
    }

    pub async fn create_prefix(&self) -> String {
        pad_str(
            &style(format!(
                "[{}/{}] {}", 
                self.step.load(Ordering::SeqCst),
                self.total_steps.load(Ordering::SeqCst), 
                self.step_name.lock().await)
            ).bold().to_string(), 
            26, 
            console::Alignment::Left, 
            None
        ).to_string()
    }

    pub async fn create_download_no_size_progress_bar(&self) -> ProgressBar {
        let prefix = self.create_prefix().await;

        ProgressBar::new(self.bytes.total())
            .with_style(
                ProgressStyle::default_bar()
                    .template(
                        "{prefix} [{elapsed_precise}] [{msg}]",
                    )
                    .expect("template string should follow the syntax")
                    .progress_chars("###"),
                    
            )
            .with_finish(ProgressFinish::AndLeave)
            .with_prefix(prefix)
    }

    pub async fn create_download_progress_bar(&self) -> ProgressBar {
        let prefix = self.create_prefix().await;

        ProgressBar::new(self.files.total())
            .with_style(
                ProgressStyle::default_bar()
                    .template(
                        "{prefix} [{wide_bar:.cyan/dim}] {pos}/{len} [{elapsed_precise}] [{msg}]",
                    )
                    .expect("template string should follow the syntax")
                    .progress_chars("###"),
                    
            )
            .with_finish(ProgressFinish::AndLeave)
            .with_prefix(prefix)
    }

    pub fn update_for_files(&self, progress_bar: &ProgressBar) {
        progress_bar.set_length(self.files.total());

        let files_total = self.files.total();
        let files_remaining = self.files.remaining();

        if files_total >= files_remaining {
            progress_bar.set_position(self.files.total() - self.files.remaining());
        }

        progress_bar.set_message(HumanBytes(self.bytes.success()).to_compact_string());
    }

    pub fn set_total_steps(&self, num_steps: u8) {
        self.total_steps.store(num_steps, Ordering::SeqCst);
    }

    pub async fn next_step(&self, step_name: &str) {
        *self.step_name.lock().await = step_name.to_string();

        self.bytes.reset();
        self.files.reset();

        self.step.fetch_add(1, Ordering::SeqCst);
    }
    
    pub async fn wait_for_completion(&self, progress_bar: &ProgressBar)  {
        while self.files.remaining() > 0 {
            self.update_for_files(progress_bar);
            sleep(Duration::from_millis(100)).await
        }

        self.total_bytes.fetch_add(self.bytes.success(), Ordering::SeqCst);

        self.update_for_files(progress_bar);

        progress_bar.finish_using_style();
    }
}


pub async fn spawn_updater(progress_pairs: Vec<(Progress, ProgressBar)>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            for (progress, pb) in &progress_pairs {
                progress.update_for_files(pb);
            }
    
            sleep(Duration::from_millis(100)).await
        }
    })
}

#[derive(Clone, Default, Debug)]
pub struct ProgressPart {
    total: Arc<AtomicU64>,
    success: Arc<AtomicU64>,
    skipped: Arc<AtomicU64>,
    failed: Arc<AtomicU64>
}

impl Display for ProgressPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(
            format_args!(
                "{} succeeded, {} skipped, {} failed",
                self.success(), self.skipped(), self.failed()
            )
        )
    }
}

impl ProgressPart {
    pub fn new() -> Self {
        Self {
            total: Arc::new(AtomicU64::new(0)),
            success: Arc::new(AtomicU64::new(0)),
            skipped: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn inc_total(&self, count: u64) {
        self.total.fetch_add(count, Ordering::SeqCst);
    }

    pub fn inc_success(&self, count: u64) {
        self.success.fetch_add(count, Ordering::SeqCst);
    }

    pub fn inc_skipped(&self, count: u64) {
        self.skipped.fetch_add(count, Ordering::SeqCst);
    }

    /*
    pub fn inc_failed(&self, count: u64) {
        self.failed.fetch_add(count, Ordering::SeqCst);
    }
     */

    pub fn total(&self) -> u64 {
        self.total.load(Ordering::SeqCst)
    }

    pub fn success(&self) -> u64 {
        self.success.load(Ordering::SeqCst)
    }

    pub fn skipped(&self) -> u64 {
        self.skipped.load(Ordering::SeqCst)
    }

    pub fn failed(&self) -> u64 {
        self.failed.load(Ordering::SeqCst)
    }

    pub fn remaining(&self) -> u64 {
        self.total.load(Ordering::SeqCst) -
            self.success.load(Ordering::SeqCst) -
            self.skipped.load(Ordering::SeqCst) -
            self.failed.load(Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.total.store(0, Ordering::SeqCst);
        self.success.store(0, Ordering::SeqCst);
        self.skipped.store(0, Ordering::SeqCst);
        self.failed.store(0, Ordering::SeqCst);
    }
}