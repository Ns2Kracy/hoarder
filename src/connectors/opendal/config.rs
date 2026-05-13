use std::{collections::BTreeMap, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    connectors::traits::ConnectorConfig,
    error::{AppError, AppResult},
};

pub const REDACTED_SECRET: &str = "[redacted]";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenDalServiceKind {
    Fs,
    #[serde(rename = "webdav")]
    WebDav,
    Sftp,
    S3,
}

impl OpenDalServiceKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fs => "fs",
            Self::WebDav => "webdav",
            Self::Sftp => "sftp",
            Self::S3 => "s3",
        }
    }
}

impl fmt::Display for OpenDalServiceKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OpenDalServiceKind {
    type Err = AppError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "fs" => Ok(Self::Fs),
            "webdav" | "web_dav" => Ok(Self::WebDav),
            "sftp" => Ok(Self::Sftp),
            "s3" => Ok(Self::S3),
            other => Err(AppError::Validation(format!(
                "unsupported OpenDAL service `{other}`"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "service", rename_all = "lowercase")]
pub enum OpenDalServiceConfig {
    Fs {
        root: String,
    },
    #[serde(rename = "webdav")]
    WebDav {
        endpoint: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        root: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        username: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    Sftp {
        endpoint: String,
        username: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        root: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        private_key: Option<String>,
    },
    S3 {
        bucket: String,
        region: String,
        access_key_id: String,
        secret_access_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        endpoint: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        root: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_token: Option<String>,
    },
}

impl OpenDalServiceConfig {
    #[must_use]
    pub const fn kind(&self) -> OpenDalServiceKind {
        match self {
            Self::Fs { .. } => OpenDalServiceKind::Fs,
            Self::WebDav { .. } => OpenDalServiceKind::WebDav,
            Self::Sftp { .. } => OpenDalServiceKind::Sftp,
            Self::S3 { .. } => OpenDalServiceKind::S3,
        }
    }

    #[must_use]
    pub fn redacted(&self) -> Self {
        match self {
            Self::Fs { root } => Self::Fs { root: root.clone() },
            Self::WebDav {
                endpoint,
                root,
                username,
                password,
                token,
            } => Self::WebDav {
                endpoint: endpoint.clone(),
                root: root.clone(),
                username: username.clone(),
                password: password.as_ref().map(|_| REDACTED_SECRET.to_owned()),
                token: token.as_ref().map(|_| REDACTED_SECRET.to_owned()),
            },
            Self::Sftp {
                endpoint,
                username,
                root,
                password,
                private_key,
            } => Self::Sftp {
                endpoint: endpoint.clone(),
                username: username.clone(),
                root: root.clone(),
                password: password.as_ref().map(|_| REDACTED_SECRET.to_owned()),
                private_key: private_key.as_ref().map(|_| REDACTED_SECRET.to_owned()),
            },
            Self::S3 {
                bucket,
                region,
                access_key_id,
                secret_access_key,
                endpoint,
                root,
                session_token,
            } => Self::S3 {
                bucket: bucket.clone(),
                region: region.clone(),
                access_key_id: redact_present(access_key_id),
                secret_access_key: redact_present(secret_access_key),
                endpoint: endpoint.clone(),
                root: root.clone(),
                session_token: session_token.as_ref().map(|_| REDACTED_SECRET.to_owned()),
            },
        }
    }
}

#[must_use]
pub const fn supported_service_kinds() -> &'static [OpenDalServiceKind] {
    &[
        OpenDalServiceKind::Fs,
        OpenDalServiceKind::WebDav,
        OpenDalServiceKind::Sftp,
        OpenDalServiceKind::S3,
    ]
}

/// Validates a generic connector config as an `OpenDAL` config.
///
/// # Errors
///
/// Returns an error when the service kind is unknown or required service
/// options are missing.
pub fn validate_connector_config(config: &ConnectorConfig) -> AppResult<OpenDalServiceConfig> {
    match config {
        ConnectorConfig::OpenDal { service, options } => {
            validate_service_options(service.parse()?, options)
        }
    }
}

#[must_use]
pub fn redacted_connector_config(config: &ConnectorConfig) -> ConnectorConfig {
    match config {
        ConnectorConfig::OpenDal { service, options } => ConnectorConfig::OpenDal {
            service: service.clone(),
            options: redacted_options(options),
        },
    }
}

#[must_use]
pub fn redacted_options(options: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    options
        .iter()
        .map(|(key, value)| {
            let value = if is_secret_option_key(key) {
                REDACTED_SECRET.to_owned()
            } else {
                value.clone()
            };

            (key.clone(), value)
        })
        .collect()
}

fn validate_service_options(
    kind: OpenDalServiceKind,
    options: &BTreeMap<String, String>,
) -> AppResult<OpenDalServiceConfig> {
    match kind {
        OpenDalServiceKind::Fs => Ok(OpenDalServiceConfig::Fs {
            root: required(options, kind, "root")?,
        }),
        OpenDalServiceKind::WebDav => Ok(OpenDalServiceConfig::WebDav {
            endpoint: required(options, kind, "endpoint")?,
            root: optional(options, "root"),
            username: optional(options, "username"),
            password: optional(options, "password"),
            token: optional(options, "token"),
        }),
        OpenDalServiceKind::Sftp => Ok(OpenDalServiceConfig::Sftp {
            endpoint: required(options, kind, "endpoint")?,
            username: required(options, kind, "username")?,
            root: optional(options, "root"),
            password: optional(options, "password"),
            private_key: optional(options, "private_key").or_else(|| optional(options, "key")),
        }),
        OpenDalServiceKind::S3 => Ok(OpenDalServiceConfig::S3 {
            bucket: required(options, kind, "bucket")?,
            region: required(options, kind, "region")?,
            access_key_id: required(options, kind, "access_key_id")?,
            secret_access_key: required(options, kind, "secret_access_key")?,
            endpoint: optional(options, "endpoint"),
            root: optional(options, "root"),
            session_token: optional(options, "session_token"),
        }),
    }
}

fn required(
    options: &BTreeMap<String, String>,
    kind: OpenDalServiceKind,
    key: &str,
) -> AppResult<String> {
    optional(options, key).ok_or_else(|| {
        AppError::Validation(format!(
            "OpenDAL {kind} config missing required option `{key}`"
        ))
    })
}

fn optional(options: &BTreeMap<String, String>, key: &str) -> Option<String> {
    options
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn is_secret_option_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();

    normalized.contains("password")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("privatekey")
        || (normalized.contains("access") && normalized.contains("key"))
}

fn redact_present(_value: &str) -> String {
    REDACTED_SECRET.to_owned()
}
