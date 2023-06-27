pub mod slack;
pub mod wechat;
use crate::db::Release;
use crate::shutdown::Shutdown;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Semaphore;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub slack: slack::AlertProvider,
    pub wechat: wechat::AlertProvider,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            slack: Default::default(),
            wechat: Default::default(),
        }
    }
}

pub async fn do_alert(
    alert: Config,
    mut notify_shutdown_alert: Shutdown,
    _shutdown_complete_tx_alert: Sender<()>,
    release_rx: Receiver<Release>,
) {
    info!("Start doing alert repo release.");

    tokio::join!(notify_shutdown_alert.recv(), try_alert(release_rx, alert));
    info!("alert module is stopping.");
}

async fn try_alert(mut release_rx: Receiver<Release>, alert: Config) {
    let semaphore = Arc::new(Semaphore::new(4));
    while let Some(v) = release_rx.recv().await {
        if semaphore.acquire().await.is_ok() {
            let alert = alert.clone();
            tokio::spawn(async move {
                if !alert.slack.webhook_url.is_empty() {
                    match alert.slack.send(v.clone()).await {
                        Ok(_) => {
                            info!("repo:{} - send alert to slack!", v.name);
                        }
                        Err(e) => {
                            error!("{}", e);
                        }
                    }
                }
                if !alert.wechat.webhook_url.is_empty() {
                    match alert.wechat.send(v.clone()).await {
                        Ok(_) => {
                            info!("repo:{} - send alert to wechat!", v.name);
                        }
                        Err(e) => {
                            error!("{}", e);
                        }
                    }
                }
            });
        } else {
            error!(
                "the semaphore has been closed. Close task for send alert msg {}",
                v.name
            );
        }
    }
    semaphore.close();
}
