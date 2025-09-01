use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use axum::Router;
use clap::Parser;
use tantivy::Index;
use tokio::{net::TcpListener, signal};

use crate::ext_searcher::ExtSearcher;

pub mod extensions;

#[derive(Clone, Parser)]
pub struct ServeOpts {
    #[arg(long, short, help="Web server port", default_value = "8070")]
    pub port: u16
}

#[derive(Clone)]
pub struct AppState {
    searcher: ExtSearcher,
    output: Arc<str>,
}

impl AppState {
    pub fn init(output: &str) -> anyhow::Result<Self> {
        let index = Index::open_in_dir(format!("{output}/idx"))?;

        let searcher = ExtSearcher::init(index)?;

        let output: Arc<str> = Arc::from(output);

        Ok(Self {
            searcher,
            output
        })
    }
}

pub async fn serve(opts: &ServeOpts, output: &str) -> anyhow::Result<()> {
    let state = AppState::init(output)?;

    let app = Router::new()
        .merge(extensions::get_routes(state.clone()))
        .with_state(state);

    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), opts.port)).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}


async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}