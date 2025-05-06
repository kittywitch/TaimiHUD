use {
    crate::settings::source::GitHubSource, crate::{controller::ProgressBarStyleChange, render::TextFont, SETTINGS}, anyhow::anyhow, async_compression::tokio::bufread::GzipDecoder, chrono::{DateTime, Utc}, futures::stream::{StreamExt, TryStreamExt}, nexus::imgui::Ui, reqwest::{Client, IntoUrl, Response}, serde::{de::DeserializeOwned, Deserialize, Serialize}, std::{
        collections::HashMap, fmt::{self, Display}, fs, io, path::{Path, PathBuf}, sync::Arc
    }, strum_macros::Display, tokio::{
        fs::{create_dir_all, read_to_string, remove_dir_all, try_exists, File},
        io::AsyncWriteExt,
        sync::RwLock,
    }, tokio_tar::Archive, tokio_util::io::StreamReader,
};

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
pub struct TimerSettings {
    #[serde(default)]
    pub disabled: bool,
}

impl TimerSettings {
    pub fn disable(&mut self) {
        self.disabled = true;
    }
    pub fn enable(&mut self) {
        self.disabled = false;
    }
    pub fn toggle(&mut self) -> bool {
        self.disabled = !self.disabled;
        self.disabled
    }
}

