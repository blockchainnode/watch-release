use crate::server;
use crate::shutdown::run_until_ctrl_c;
use anyhow::Result;
use chrono::offset::Local;
use chrono::DateTime;
use clap::{Parser, Subcommand};
use env_logger::{Builder, Env};
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the client
    Server(server::Command),
}

pub fn init_log() {
    let env = Env::default()
        .default_write_style_or("never")
        .default_filter_or("info");
    Builder::from_env(env)
        .format(|buf, record| {
            let local: DateTime<Local> = Local::now();
            let file = record.file().unwrap_or("unknown caller file.");
            // 0 = unable to get the line number in file
            let line = record.line().unwrap_or(0);
            writeln!(
                buf,
                "{} [{}] [caller:\"{}:{}\"] {}",
                record.level(),
                local.format("%Y-%m-%d %H:%M:%S%.6f"),
                file,
                line,
                record.args()
            )
        })
        .init();
}

pub async fn run() -> Result<()> {
    let opt = Cli::parse();

    match opt.command {
        Commands::Server(command) => run_until_ctrl_c(command).await,
    }
}
