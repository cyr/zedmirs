use std::{fmt::Display, process::exit};

use clap::Parser;

use crate::{config::Config};

mod package_meta;
mod progress;
mod downloader;
mod mirror;
mod serve;
mod config;
mod index;
mod ext_searcher;

#[tokio::main()]
async fn main() {
    let config = Config::parse();

    if let Err(e) = config.cmd.execute(&config).await {
        log(format!("FATAL: {e}"));
        exit(-1)
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn log<M: Display>(msg: M) {
    println!("{} {msg}", now());
}