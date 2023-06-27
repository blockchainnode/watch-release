use crate::db::Release;
use anyhow::anyhow;
use bytes::Bytes;
use log::trace;
use reqwest::header::{self, HeaderMap};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AlertProvider {
    #[serde(rename = "webhook-url")]
    pub webhook_url: String,
}

impl Default for AlertProvider {
    fn default() -> Self {
        AlertProvider {
            webhook_url: String::new(),
        }
    }
}

impl AlertProvider {
    pub async fn send(&self, release: Release) -> anyhow::Result<()> {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(WECHAT_HTTP_CONTENT),
        );
        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(5))
            .build()?;
        let body = AlertProvider::build_http_body(release);

        let resp = client
            .post(self.webhook_url.clone())
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!(
                "cannot send the latest release info to alert provider. code = {}",
                resp.status().as_u16()
            ));
        }

        Ok(())
    }

    fn build_http_body(release: Release) -> Bytes {
        let msg = format!(
            "**<font color=\"warning\">New Github Release Version</font>**\n> name: <font color=\"info\">{}</font>\n> tag: <font color=\"info\">{}</font>\n> release_name: <font color=\"info\">{}</font>\n> published_at: <font color=\"info\">{}</font>\n> url: <font color=\"info\">{}</font>",
            release.name,
            release.detail.tag_name,
            release.detail.release_name,
            release.detail.published_at,
            release.detail.html_url,
        );
        let wx_data = WxData {
            msgtype: "markdown".to_string(),
            markdown: WxMarkdwon { content: msg },
        };

        let tmp = json!(wx_data).to_string();
        trace!("wechat json content: {}", tmp);
        Bytes::from(tmp)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WxData {
    markdown: WxMarkdwon,
    msgtype: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WxMarkdwon {
    content: String,
}

const WECHAT_HTTP_CONTENT: &str = "application/json";
