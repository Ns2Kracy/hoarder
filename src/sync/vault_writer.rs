use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

use crate::{
    AppResult,
    connectors::traits::ByteStream,
    core::{
        types::ItemRef,
        vault_path::{normalize_source_path, target_path},
    },
};

#[derive(Clone, Debug)]
pub struct VaultWriter {
    vault_root: PathBuf,
}

impl VaultWriter {
    #[must_use]
    pub const fn new(vault_root: PathBuf) -> Self {
        Self { vault_root }
    }

    /// Streams an item into the vault through a temporary file.
    ///
    /// # Errors
    ///
    /// Returns an error when path normalization, directory creation, writing,
    /// hashing, or atomic promotion fails.
    pub async fn write(&self, item_ref: &ItemRef, bytes: ByteStream) -> AppResult<VaultWrite> {
        let normalized_path = normalize_source_path(&item_ref.source_path)?;
        let target_path = target_path(&self.vault_root, &item_ref.source_id, &normalized_path)?;
        let temp_path = self.temp_path(&item_ref.source_id.to_string(), &normalized_path);

        let outcome = self.write_via_temp(&temp_path, &target_path, bytes).await;
        if outcome.is_err() {
            fs::remove_file(&temp_path).await.ok();
        }
        outcome
    }

    fn temp_path(&self, source_id: &str, normalized_path: &str) -> PathBuf {
        let leaf = normalized_path
            .rsplit('/')
            .next()
            .filter(|value| !value.is_empty())
            .unwrap_or("item");
        self.vault_root
            .join(".hoarder/tmp")
            .join(format!("{source_id}-{}-{leaf}.tmp", Uuid::new_v4()))
    }

    async fn write_via_temp(
        &self,
        temp_path: &Path,
        target_path: &Path,
        mut bytes: ByteStream,
    ) -> AppResult<VaultWrite> {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(temp_path).await?;
        let mut hasher = Sha256::new();
        let mut bytes_written = 0;

        while let Some(chunk) = bytes.next().await {
            let chunk: Bytes = chunk?;
            hasher.update(&chunk);
            bytes_written += chunk.len() as u64;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        drop(file);

        fs::rename(temp_path, target_path).await?;

        Ok(VaultWrite {
            target_path: target_path.to_owned(),
            content_hash: format!("sha256:{}", hex::encode(hasher.finalize())),
            bytes_written,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultWrite {
    pub target_path: PathBuf,
    pub content_hash: String,
    pub bytes_written: u64,
}
