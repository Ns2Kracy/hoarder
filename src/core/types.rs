use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseLocalIdError {
    value: String,
}

impl ParseLocalIdError {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }
}

impl fmt::Display for ParseLocalIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected a positive integer id, got `{}`",
            self.value
        )
    }
}

impl std::error::Error for ParseLocalIdError {}

fn positive_local_id(value: i64) -> Result<i64, ParseLocalIdError> {
    if value <= 0 {
        return Err(ParseLocalIdError::new(&value.to_string()));
    }

    Ok(value)
}

fn positive_local_id_from_uuid(uuid: uuid::Uuid) -> i64 {
    let value = uuid.as_u128() & (i64::MAX as u128);
    let value = i64::try_from(value).unwrap_or(i64::MAX);

    value.max(1)
}

macro_rules! local_id_newtype {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(i64);

        impl $name {
            pub fn new() -> Self {
                Self(positive_local_id_from_uuid(uuid::Uuid::new_v4()))
            }

            pub const fn from_i64(value: i64) -> Self {
                Self(value)
            }

            pub const fn as_i64(self) -> i64 {
                self.0
            }

            // Temporary UUID bridge for the migration branch. Remove when entities/repositories use integer IDs directly.
            pub fn from_uuid(uuid: uuid::Uuid) -> Self {
                Self(positive_local_id_from_uuid(uuid))
            }

            // Temporary UUID bridge for the migration branch. Remove when entities/repositories use integer IDs directly.
            pub fn as_uuid(self) -> uuid::Uuid {
                let value = u128::try_from(self.0).unwrap_or_default();

                uuid::Uuid::from_u128(value)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = i64::deserialize(deserializer)?;
                positive_local_id(value)
                    .map(Self)
                    .map_err(serde::de::Error::custom)
            }
        }

        impl FromStr for $name {
            type Err = ParseLocalIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let parsed = value
                    .parse::<i64>()
                    .map_err(|_| ParseLocalIdError::new(value))?;
                positive_local_id(parsed).map(Self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

local_id_newtype!(SourceId);
local_id_newtype!(JobId);
local_id_newtype!(RunId);
local_id_newtype!(ItemId);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    File,
    Directory,
    VirtualDocument,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Pending,
    Synced,
    Failed,
    Skipped,
    DeletedOnSource,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobScheduleKind {
    Manual,
    Interval,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Idle,
    Running,
    Paused,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    CompletedWithFailures,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKind {
    #[serde(rename = "opendal")]
    OpenDal,
    Notion,
    Feishu,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorCapabilities {
    pub supports_files: bool,
    pub supports_directories: bool,
    pub supports_virtual_documents: bool,
    pub supports_incremental_scan: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSnapshot {
    pub source_id: SourceId,
    pub source_path: String,
    pub item_type: ItemType,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub metadata_json: Option<Value>,
}

impl ItemSnapshot {
    #[must_use]
    pub fn item_ref(&self) -> ItemRef {
        ItemRef {
            source_id: self.source_id,
            source_path: self.source_path.clone(),
            item_type: self.item_type,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    pub source_id: SourceId,
    pub source_path: String,
    pub item_type: ItemType,
}
