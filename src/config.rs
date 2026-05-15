use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub vault_path: PathBuf,
    pub listen_addr: SocketAddr,
    pub job_concurrency: usize,
    pub file_concurrency: usize,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("./hoarder.db"),
            vault_path: PathBuf::from("./vault"),
            listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4761),
            job_concurrency: 1,
            file_concurrency: 4,
            log_level: default_log_level(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettings {
    pub database_path: String,
    pub vault_path: String,
    pub listen_addr: SocketAddr,
    pub job_concurrency: usize,
    pub file_concurrency: usize,
    pub log_level: String,
    pub read_only: RuntimeSettingsReadOnly,
}

impl RuntimeSettings {
    #[must_use]
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            database_path: config.database_path.to_string_lossy().into_owned(),
            vault_path: config.vault_path.to_string_lossy().into_owned(),
            listen_addr: config.listen_addr,
            job_concurrency: config.job_concurrency,
            file_concurrency: config.file_concurrency,
            log_level: config.log_level.clone(),
            read_only: RuntimeSettingsReadOnly {
                database_path: true,
                vault_path: true,
                listen_addr: true,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettingsReadOnly {
    pub database_path: bool,
    pub vault_path: bool,
    pub listen_addr: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettingsPatch {
    pub job_concurrency: Option<usize>,
    pub file_concurrency: Option<usize>,
    pub log_level: Option<String>,
}

fn default_log_level() -> String {
    "info".to_owned()
}
