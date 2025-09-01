use clap::{Parser, Subcommand};

use crate::{mirror::{mirror, MirrorOpts}, serve::{serve, ServeOpts}};


#[derive(Parser)]
pub struct Config {
    #[arg(long, short, help="Mirror data path")]
    pub output: String,

    #[command(subcommand)]
    pub cmd: Op
}


#[derive(Subcommand)]
pub enum Op {
    Mirror(MirrorOpts),
    Serve(ServeOpts)
}

impl Op {
    pub async fn execute(&self, config: &Config) -> anyhow::Result<()> {
        match self {
            Op::Mirror(opts) => mirror(opts, &config.output).await,
            Op::Serve(opts) => serve(opts, &config.output).await,
        }
    }
}