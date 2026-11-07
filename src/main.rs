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
            warn!("webOS status polling is not implemented yet");
            info!(tv_host = %cfg.tv.host, "configured television");
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
