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
            header::HeaderValue::from_static(SLACK_HTTP_CONTENT),
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
            "*name:* {}\n*tag:* {}\n*release_name:* {}\n*publish_at:* {}\n*url:* {}\n",
            release.name,
            release.detail.tag_name,
            release.detail.release_name,
            release.detail.published_at,
            release.detail.html_url,
        );
        let block = SlackNoticeBlock {
            type_alias: "section".to_string(),
            text: SlackNoticeText {
                type_alias: "mrkdwn".to_string(),
                text: msg,
            },
        };
        let header = SlackNoticeBlock {
            type_alias: "header".to_string(),
            text: SlackNoticeText {
                type_alias: "plain_text".to_string(),
                text: "New Github Release Version".to_string(),
            },
        };
        let attachment = SlackNoticeAttachment {
            color: SLACK_COLOR.to_string(),
            blocks: vec![header, block],
        };
        let slack_notice = SlackNotice {
            attachments: vec![attachment],
        };

        let tmp = json!(slack_notice).to_string();
        trace!("slack json content: {}", tmp);
        Bytes::from(tmp)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SlackNotice {
    attachments: Vec<SlackNoticeAttachment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SlackNoticeAttachment {
    blocks: Vec<SlackNoticeBlock>,
    color: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SlackNoticeBlock {
    text: SlackNoticeText,
    #[serde(rename = "type")]
    type_alias: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SlackNoticeText {
    text: String,
    #[serde(rename = "type")]
    type_alias: String,
}

const SLACK_COLOR: &str = "#f2c744";
const SLACK_HTTP_CONTENT: &str = "application/json";
