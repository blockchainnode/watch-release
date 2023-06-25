use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ServerConfig {
    #[serde(rename = "dbPath")]
    pub db_path: PathBuf,
    //Convert the unit of period to seconds
    pub period: u64,
    #[serde(rename = "retryInterval")]
    //Convert the unit of retry_interval to seconds
    pub retry_interval: u64,
    pub alert: Vec<Provider>,
    #[serde(rename = "repoList")]
    pub repo_list: Vec<Repo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Repo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    pub client: String,
    #[serde(rename = "webhook-url")]
    pub webhook_url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let mut working_dir = env::current_dir().expect("cannot get the working dir.");
        working_dir.push("data");
        Self {
            db_path: working_dir,
            period: 7200,
            retry_interval: 600,
            alert: Vec::new(),
            repo_list: Vec::new(),
        }
    }
}

pub fn parse_config(file: &PathBuf) -> Result<ServerConfig> {
    let content = fs::read_to_string(file).context("cannot read the config file")?;
    let server_config: ServerConfig =
        serde_json::from_str(&content).context("fail to deserialize config file(json)")?;
    for v in server_config.alert.iter() {
        if !VAILDALERT.contains(&v.client.as_str()) {
            return Err(anyhow!("Provide unsupported alert client - {}", v.client));
        }
    }
    Ok(server_config)
}

const VAILDALERT: [&str; 2] = ["slack", "wechat"];
pub const RETRY: u8 = 2;
