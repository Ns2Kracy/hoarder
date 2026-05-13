use std::path::{Path, PathBuf};

use crate::{
    core::types::SourceId,
    error::{AppError, AppResult},
};

/// Normalizes a source-relative path for safe storage in the vault.
///
/// # Errors
///
/// Returns an error when the path is empty, absolute, contains traversal, or
/// targets Hoarder's reserved `.hoarder` directory.
pub fn normalize_source_path(input: &str) -> AppResult<String> {
    if input.is_empty() {
        return Err(AppError::Path("source path cannot be empty".to_owned()));
    }

    if input.starts_with('/') || input.starts_with('\\') {
        return Err(AppError::Path("source path cannot be absolute".to_owned()));
    }

    if has_windows_drive_prefix(input) {
        return Err(AppError::Path(
            "source path cannot include a Windows drive prefix".to_owned(),
        ));
    }

    if input.contains('\0') {
        return Err(AppError::Path("source path cannot contain NUL".to_owned()));
    }

    let unified = input.replace('\\', "/");
    let mut parts = Vec::new();

    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                return Err(AppError::Path(
                    "source path cannot include traversal".to_owned(),
                ));
            }
            value => parts.push(value),
        }
    }

    let Some(first) = parts.first() else {
        return Err(AppError::Path("source path cannot be empty".to_owned()));
    };

    if *first == ".hoarder" {
        return Err(AppError::Path(
            "source path cannot target the .hoarder directory".to_owned(),
        ));
    }

    Ok(parts.join("/"))
}

/// Builds a vault target path for a normalized source path.
///
/// # Errors
///
/// Returns an error when `normalized_path` is not a safe source-relative path.
pub fn target_path(
    vault_root: impl AsRef<Path>,
    source_id: &SourceId,
    normalized_path: &str,
) -> AppResult<PathBuf> {
    let normalized_path = normalize_source_path(normalized_path)?;

    Ok(vault_root
        .as_ref()
        .join(source_id.to_string())
        .join(normalized_path))
}

fn has_windows_drive_prefix(input: &str) -> bool {
    let bytes = input.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}
