use {
    super::GitHubSource, crate::{controller::ProgressBarStyleChange, render::TextFont, SETTINGS}, anyhow::anyhow, async_compression::tokio::bufread::GzipDecoder, chrono::{DateTime, Utc}, futures::stream::{StreamExt, TryStreamExt}, nexus::imgui::Ui, reqwest::{Client, IntoUrl, Response}, serde::{de::DeserializeOwned, Deserialize, Serialize}, std::{
        collections::HashMap, fmt::{self, Display}, fs, io, path::{Path, PathBuf}, sync::Arc
    }, strum_macros::Display, tokio::{
        fs::{create_dir_all, read_to_string, remove_dir_all, try_exists, File},
        io::AsyncWriteExt,
        sync::RwLock,
    }, tokio_tar::Archive, tokio_util::io::StreamReader,
};

#[derive(PartialEq, Clone, Debug, Default)]
pub enum NeedsUpdate {
    #[default]
    Unknown,
    Error(String),
Known(bool, String),
}

impl fmt::Display for NeedsUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use NeedsUpdate::*;
        match &self {
            Unknown => write!(f, "Unknown"),
            Error(e) => write!(f, "Error: {e}!"),
            Known(true, id) => write!(f, "Available: {}", id),
            Known(false, _id) => write!(f, "Up to date!"),
        }
    }
}

impl NeedsUpdate {
    pub fn draw(&self, ui: &Ui) {
        let text = self.to_string();
        use NeedsUpdate::*;
        match &self {
            Unknown => ui.text_colored([1.0, 1.0, 0.0, 1.0], text),
            Error(_e) => ui.text_colored([1.0, 0.0, 0.0, 1.0], text),
            Known(true, _id) => ui.text_colored([1.0, 0.6, 0.0, 1.0], text),
            Known(false, _id) => ui.text_colored([0.0, 1.0, 0.0, 1.0], text),
        }
    }
}

