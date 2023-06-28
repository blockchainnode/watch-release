use crate::config::{Repo, RETRY};
use crate::db::{key_in_db_status, KeyFlag, Release, ReleaseDetail};
use crate::shutdown::Shutdown;
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, trace};
use microkv::MicroKV;
use reqwest::{self, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Semaphore;
use tokio::time::{self, Duration};

#[derive(Serialize, Deserialize, Clone)]
pub struct Puller {
    pub retry_interval: u64,
    pub repo: Repo,
    pub db: MicroKV,
    pub retry: u8,
}

type PullerList = Vec<Puller>;

impl Puller {
    pub fn new(db: MicroKV, retry_interval: u64, repo: Repo, retry: u8) -> Puller {
        Puller {
            retry_interval,
            repo,
            db,
            retry,
        }
    }

    fn update_retry(&mut self) {
        self.retry = self.retry + 1;
    }

    async fn pull(&self, release_tx: Sender<Release>) -> Result<()> {
        let client = Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("masayil")
            .build()?;
        trace!("Build http client complete.");
        let resp = client.get(&self.repo.url).send().await?.text().await?;
        debug!(
            "Requested the latest release version of the {}",
            self.repo.name
        );
        let detail: ReleaseDetail =
            serde_json::from_str(resp.as_str()).context("Deserialize http resopnes failed!")?;
        trace!("Deserialized the http resopnes to crate::db::ReleaseDetail.");
        let release = Release::new(self.repo.url.clone(), self.repo.name.clone(), detail);

        match key_in_db_status(self.db.clone(), self.repo.name.as_str()) {
            KeyFlag::Exist => {
                let value: Release = self.db.get_unwrap(&self.repo.name)?;
                trace!("Get the value of key:{}", self.repo.name);
                if value != release {
                    let _ = release_tx.send(release.clone()).await?;
                    info!(
                        "send {} latest release version to the alert channel",
                        self.repo.name
                    );
                    info!(
                        "Repo: {} found the new release version. Current version is {}. The latest version is {}",
                        self.repo.name,value.detail.release_name, release.detail.release_name
                    );
                    let _ = self.db.put(&self.repo.name, &release)?;
                    debug!("Update key:{} in db.", self.repo.name);
                } else {
                    info!(
                        "Repo: {} has not the new release version. Current version is {}",
                        self.repo.name, value.detail.release_name
                    );
                }
            }
            KeyFlag::NotExist => {
                info!(
                    "Repo: {} found the new release version. The latest version is {}",
                    self.repo.name, release.detail.release_name
                );
                let _ = self.db.put(&self.repo.name, &release)?;
                debug!("Update key:{} in db.", self.repo.name);
            }
            KeyFlag::FnFail => {
                error!("Query key:{} in the db failed.", self.repo.name);
                return Err(anyhow!("Get error when execute db::exists!!"));
            }
        }
        Ok(())
    }
}

pub async fn do_watch(
    period: u64,
    retry_interval: u64,
    repo_list: Vec<Repo>,
    db: MicroKV,
    mut notify_shutdown_watch: Shutdown,
    _shutdown_complete_tx_watch: Sender<()>,
    release_tx: Sender<Release>,
) {
    let mut puller_list = PullerList::new();
    for v in repo_list.into_iter() {
        puller_list.push(Puller::new(db.clone(), retry_interval, v, 1));
    }
    while !notify_shutdown_watch.is_shutdown() {
        info!("Start doing watch repo release.");
        tokio::select! {
            _ = notify_shutdown_watch.recv() => {
                info!("Watch module is stopping.");
            },
            _ = try_watch(puller_list.clone(),period,release_tx.clone())=>{
            },
        }
    }
}

async fn try_watch(puller_list: PullerList, period: u64, release_tx: Sender<Release>) {
    let mut spawn_queue = Vec::new();
    let semaphore = Arc::new(Semaphore::new(8));
    for mut v in puller_list.into_iter() {
        if semaphore.acquire().await.is_ok() {
            let release_tx = release_tx.clone();
            let handler = tokio::spawn(async move {
                loop {
                    let release_tx = release_tx.clone();
                    match v.pull(release_tx).await {
                        Err(e) => {
                            error!("Pull {} release info failed. Error: {}", v.repo.name, e);
                            if v.retry > RETRY {
                                break;
                            }
                            info!(
                                "Retry pull {} release info after {} seconds!",
                                v.repo.name, v.retry_interval
                            );
                            v.update_retry();
                            time::sleep(Duration::from_secs(v.retry_interval.into())).await;
                        }
                        Ok(_) => {
                            break;
                        }
                    }
                }
            });
            spawn_queue.push(handler);
        } else {
            error!(
                "the semaphore has been closed. Close task for pulling repo {}",
                v.repo.name
            );
        }
    }
    for v in spawn_queue.into_iter() {
        let _ = v.await;
    }
    info!("Complete doing watch repo release.");
    semaphore.close();
    time::sleep(Duration::from_secs(period)).await;
}
