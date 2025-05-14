use {
    super::GitHubSource, crate::{fl, controller::ProgressBarStyleChange, render::TextFont, SETTINGS}, anyhow::anyhow, async_compression::tokio::bufread::GzipDecoder, chrono::{DateTime, Utc}, futures::stream::{StreamExt, TryStreamExt}, nexus::imgui::Ui, reqwest::{Client, IntoUrl, Response}, serde::{de::DeserializeOwned, Deserialize, Serialize}, std::{
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
            Unknown => {
                let translation = fl!("update-unknown");
                write!(f, "{}", translation)
            },
            Error(e) => {
                let translation = fl!("update-error", error = e);
                write!(f, "{}", translation)
            },
            Known(true, id) => {
                let translation = fl!("update-available", version = id);
                write!(f, "{}", translation)
            },
            Known(false, _id) => {
                let translation = fl!("update-not-required");
                write!(f, "{}", translation)
            },
        }
    }
}

impl NeedsUpdate {
    pub fn draw(&self, ui: &Ui) {
        let text = self.to_string();
        ui.text_wrapped(text);
    }
}

