use crate::server::{self, Command};
use anyhow::Result;
use futures::pin_mut;
use log::{error, info};
use std::process;
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct Shutdown {
    is_shutdown: bool,

    notify: broadcast::Receiver<()>,
}

impl Shutdown {
    pub fn new(notify: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            is_shutdown: false,
            notify,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub async fn recv(&mut self) {
        if self.is_shutdown {
            return;
        }

        let _ = self.notify.recv().await;

        self.is_shutdown = true;
    }
}

pub async fn run_until_ctrl_c(command: Command) -> Result<()> {
    let server_config = command.init().await?;

    let (notify_shutdown, _) = broadcast::channel(1);
    let (shutdown_complete_tx, mut shutdown_complete_rx) = mpsc::channel(1);

    let notify_shutdown_watch = Shutdown::new(notify_shutdown.subscribe());
    let notify_shutdown_alert = Shutdown::new(notify_shutdown.subscribe());
    let shutdown_complete_tx_watch = shutdown_complete_tx.clone();
    let shutdown_complete_tx_alert = shutdown_complete_tx.clone();

    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut stream = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let sigterm = stream.recv();

        tokio::spawn(async move {
            let res = server::execute(
                server_config,
                notify_shutdown_watch,
                shutdown_complete_tx_watch,
                notify_shutdown_alert,
                shutdown_complete_tx_alert,
            )
            .await;
            if let Err(e) = res {
                error!("Error: {}", e);
                process::exit(2);
            }
        });

        pin_mut!(sigterm, ctrl_c);

        tokio::select! {
            _ = ctrl_c => {
                info!("server::cli Received ctrl-c");
            },
            _ = sigterm => {
                info!("server::cli Received SIGTERM");
            }
        }
    }

    #[cfg(not(unix))]
    {
        tokio::spawn(async move {
            let res = server::execute(
                server_config,
                notify_shutdown_watch,
                shutdown_complete_tx_watch,
                notify_shutdown_alert,
                shutdown_complete_tx_alert,
            )
            .await;
            if let Err(e) = res {
                error!("Error: {}", e);
                process::exit(2);
            }
        });
        pin_mut!(ctrl_c);

        tokio::select! {
            _ = ctrl_c => {
                info!("server::cli Received ctrl-c");
            }
        }
    }

    drop(notify_shutdown);

    drop(shutdown_complete_tx);

    let _ = shutdown_complete_rx.recv().await;

    Ok(())
}
