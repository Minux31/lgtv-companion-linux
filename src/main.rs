mod config;
mod webos;
mod wol;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "lgtv-companion", version, about)]
struct Args {
    /// Configuration file (system default: /etc/lgtv-companion/config.toml)
    #[arg(long, global = true, default_value = "/etc/lgtv-companion/config.toml")]
    config: std::path::PathBuf,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate and display non-secret configuration values.
    CheckConfig,
    /// Query the TV connection (implementation in progress).
    Status,
    /// Pair with the TV and persist the client key under /var/lib/lgtv-companion.
    Pair,
    /// Send a Wake-on-LAN magic packet.
    Wake,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(false)
        .init();

    let args = Args::parse();
    let cfg = config::Config::load(&args.config)
        .with_context(|| format!("cannot load configuration {}", args.config.display()))?;

    match args.command.unwrap_or(Command::Status) {
        Command::CheckConfig => {
            info!(tv_host = %cfg.tv.host, "configuration loaded");
            info!("configuration syntax is valid; credentials are not displayed");
        }
        Command::Status => {
            warn!("power-state polling is not implemented yet; use pair to test webOS registration");
            info!(tv_host = %cfg.tv.host, "configured television");
        }
        Command::Pair => {
            let endpoint = webos::WebOsEndpoint {
                host: cfg.tv.host.clone(),
                secure: cfg.tv.secure,
                port: cfg.tv.port,
            };
            let state_dir = std::path::Path::new("/var/lib/lgtv-companion");
            let response = webos::register_and_store(&endpoint, state_dir).await?;
            info!(response_type = %response.response_type, "webOS registration successful; client key stored securely");
        }
        Command::Wake => {
            let mac = cfg.tv.mac.as_deref().context("tv.mac is required for Wake-on-LAN")?;
            wol::send_magic_packet(mac, &cfg.network.broadcast, cfg.network.port)
                .await
                .context("Wake-on-LAN failed")?;
            info!("Wake-on-LAN packet sent");
        }
    }
    Ok(())
}
