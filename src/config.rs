use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, net::Ipv4Addr, path::Path};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub tv: TvConfig,
    #[serde(default)]
    pub network: NetworkConfig,
}

#[derive(Debug, Deserialize)]
pub struct TvConfig {
    pub host: String,
    pub mac: Option<String>,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub secure: bool,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_broadcast")]
    pub broadcast: Ipv4Addr,
    #[serde(default = "default_wol_port")]
    pub port: u16,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self { broadcast: default_broadcast(), port: default_wol_port() }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let cfg: Self = toml::from_str(&raw).context("invalid TOML configuration")?;
        anyhow::ensure!(!cfg.tv.host.trim().is_empty(), "tv.host must not be empty");
        Ok(cfg)
    }
}

fn default_port() -> u16 { 3000 }
fn default_broadcast() -> Ipv4Addr { Ipv4Addr::BROADCAST }
fn default_wol_port() -> u16 { 9 }
