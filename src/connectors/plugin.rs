#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AppError, AppResult, core::types::ConnectorCapabilities};

pub const CONNECTOR_PLUGIN_ABI_VERSION: u32 = 1;
pub const CONNECTOR_PLUGIN_ENTRYPOINT: &str = "hoarder_connector_plugin_v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorPluginManifest {
    pub abi_version: u32,
    pub id: String,
    pub display_name: String,
    pub entrypoint_symbol: String,
    pub config_schema: Value,
    pub secret_fields: Vec<PluginSecretField>,
    pub capabilities: ConnectorCapabilities,
}

impl ConnectorPluginManifest {
    /// Validates the static connector plugin contract before a host attempts to
    /// load executable code.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest targets an unsupported ABI version or
    /// omits required identity, entrypoint, or configuration schema fields.
    pub fn validate(&self) -> AppResult<()> {
        if self.abi_version != CONNECTOR_PLUGIN_ABI_VERSION {
            return Err(AppError::Validation(format!(
                "unsupported connector plugin ABI version {}; expected {}",
                self.abi_version, CONNECTOR_PLUGIN_ABI_VERSION
            )));
        }
        require_non_empty("id", &self.id)?;
        require_non_empty("displayName", &self.display_name)?;
        require_non_empty("entrypointSymbol", &self.entrypoint_symbol)?;
        if self.entrypoint_symbol != CONNECTOR_PLUGIN_ENTRYPOINT {
            return Err(AppError::Validation(format!(
                "unsupported connector plugin entrypoint {}; expected {}",
                self.entrypoint_symbol, CONNECTOR_PLUGIN_ENTRYPOINT
            )));
        }
        if !self.config_schema.is_object() {
            return Err(AppError::Validation(
                "connector plugin configSchema must be a JSON object".to_owned(),
            ));
        }
        for field in &self.secret_fields {
            require_non_empty("secretFields.name", &field.name)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSecretField {
    pub name: String,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ConnectorPluginAbi {
    pub abi_version: u32,
    pub manifest_json: extern "C" fn() -> ConnectorPluginBuffer,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ConnectorPluginBuffer {
    pub ptr: *const u8,
    pub len: usize,
}

fn require_non_empty(field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "connector plugin {field} must not be empty"
        )));
    }

    Ok(())
}
