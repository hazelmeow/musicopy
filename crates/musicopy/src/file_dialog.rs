//! File dialogs for desktop platforms using `rfd`.

use crate::error::CoreError;
use anyhow::Context;

#[uniffi::export]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn pick_folder() -> Result<String, CoreError> {
    let file = rfd::FileDialog::new()
        .pick_folder()
        .context("failed to pick folder")?;
    Ok(file.to_string_lossy().to_string())
}

#[uniffi::export]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn pick_folder() -> Result<String, CoreError> {
    return Err(CoreError::from(anyhow::anyhow!(
        "pick_folder is not supported on this platform"
    )));
}

#[uniffi::export]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub async fn pick_folders() -> Result<Vec<String>, CoreError> {
    let files = rfd::FileDialog::new()
        .pick_folders()
        .context("failed to pick folders")?;
    Ok(files
        .into_iter()
        .map(|f| f.to_string_lossy().to_string())
        .collect())
}

#[uniffi::export]
#[cfg(any(target_os = "android", target_os = "ios"))]
pub async fn pick_folders() -> Result<Vec<String>, CoreError> {
    return Err(CoreError::from(anyhow::anyhow!(
        "pick_folders is not supported on this platform"
    )));
}
