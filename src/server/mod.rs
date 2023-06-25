pub mod alert;
pub mod watch;
use crate::config;
use anyhow::{anyhow, Context, Result};
use clap::Args;
use log::info;
use microkv::MicroKV;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;

use crate::shutdown::Shutdown;

#[derive(Args)]
pub struct Command {
    /// Sets a custom config file(required)
    #[arg(short, long, required = true, value_parser = is_config_exist)]
    config_file: PathBuf,
}

fn is_config_exist(s: &str) -> Result<PathBuf> {
    let mut path = PathBuf::new();
    path.push(s);
    if path.is_file() {
        Ok(path)
    } else {
        Err(anyhow!(
            "\"{}\" is not a regular file or not exist",
            path.display()
        ))
    }
}

impl Command {
    pub fn get_config_file(&self) -> PathBuf {
        self.config_file.clone()
    }

    pub async fn init(&self) -> Result<config::ServerConfig> {
        info!("start watching the github repo releases");
        info!("use config file: {}", self.config_file.display());
        let server_config = config::parse_config(&self.config_file)?;
        let log_json_config_tmp = serde_json::to_string(&server_config)?;
        info!(
            "complete to parse the config file. The content is {}",
            log_json_config_tmp
        );

        Ok(server_config)
    }
}

pub async fn execute(
    server_config: config::ServerConfig,
    notify_shutdown_watch: Shutdown,
    shutdown_complete_tx_watch: Sender<()>,
    _notify_shutdown_alert: Shutdown,
    _shutdown_complete_tx_alert: Sender<()>,
) -> Result<()> {
    let db = MicroKV::open_with_base_path("github-release", server_config.db_path.clone())
        .context("Failed to create MicroKV from a stored file or create MicroKV for this file")?
        .set_auto_commit(true);

    let watch = watch::do_watch(
        server_config.period,
        server_config.retry_interval,
        server_config.repo_list.clone(),
        db.clone(),
        notify_shutdown_watch,
        shutdown_complete_tx_watch,
    );

    tokio::join!(watch);

    Ok(())
}
