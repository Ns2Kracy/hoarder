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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("./hoarder.db"),
            vault_path: PathBuf::from("./vault"),
            listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4761),
            job_concurrency: 1,
            file_concurrency: 4,
        }
    }
}
