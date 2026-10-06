use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub const MODEL_DIRECTORY_NAME: &str = "parakeet-tdt-0.6b-v3-int8";

pub fn data_home() -> Result<PathBuf> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path));
    }

    let home = env::var_os("HOME").context("HOME is not defined")?;
    Ok(PathBuf::from(home).join(".local/share"))
}

pub fn default_model_path() -> Result<PathBuf> {
    Ok(data_home()?
        .join("impulse-voice/models")
        .join(MODEL_DIRECTORY_NAME))
}

pub fn default_socket_path() -> Result<PathBuf> {
    let runtime_dir = env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is not defined")?;
    Ok(PathBuf::from(runtime_dir).join("impulse-voice.sock"))
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
