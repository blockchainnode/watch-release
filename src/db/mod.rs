use microkv::MicroKV;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Release {
    pub url: String,
    pub name: String,
    pub detail: ReleaseDetail,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ReleaseDetail {
    #[serde(rename = "name")]
    pub release_name: String,
    pub tag_name: String,
    pub prerelease: bool,
    pub published_at: String,
    pub html_url: String,
}

pub enum KeyFlag {
    Exist,
    NotExist,
    FnFail,
}

impl Release {
    pub fn new(url: String, name: String, detail: ReleaseDetail) -> Release {
        Release { url, name, detail }
    }
}

pub fn key_in_db_status(db: MicroKV, key: &str) -> KeyFlag {
    match db.exists(&key) {
        Err(_) => KeyFlag::FnFail,
        Ok(flag) => {
            if flag {
                KeyFlag::Exist
            } else {
                KeyFlag::NotExist
            }
        }
    }
}
