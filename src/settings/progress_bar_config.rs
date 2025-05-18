use {
    super::GitHubSource,
    crate::{controller::ProgressBarStyleChange, render::TextFont, SETTINGS},
    anyhow::anyhow,
    async_compression::tokio::bufread::GzipDecoder,
    chrono::{DateTime, Utc},
    futures::stream::{StreamExt, TryStreamExt},
    nexus::imgui::Ui,
    reqwest::{Client, IntoUrl, Response},
    serde::{de::DeserializeOwned, Deserialize, Serialize},
    std::{
        collections::HashMap,
        fmt::{self, Display},
        fs, io,
        path::{Path, PathBuf},
        sync::Arc,
    },
    strum_macros::Display,
    tokio::{
        fs::{create_dir_all, read_to_string, remove_dir_all, try_exists, File},
        io::AsyncWriteExt,
        sync::RwLock,
    },
    tokio_tar::Archive,
    tokio_util::io::StreamReader,
};

fn default_text_font() -> TextFont {
    TextFont::Ui
}

fn default_height() -> f32 {
    24.0
}

fn bool_true() -> bool {
    true
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProgressBarSettings {
    #[serde(default)]
    pub stock: bool,

    #[serde(default = "default_text_font")]
    pub font: TextFont,
    #[serde(default = "default_height")]
    pub height: f32,
    #[serde(default = "bool_true")]
    pub shadow: bool,
    #[serde(default)]
    pub centre_after: bool,
}

impl Default for ProgressBarSettings {
    fn default() -> Self {
        Self {
            font: default_text_font(),
            height: default_height(),
            stock: false,
            shadow: true,
            centre_after: false,
        }
    }
}

impl ProgressBarSettings {
    pub fn set_height(&mut self, height: f32) {
        self.height = height;
    }
    pub fn set_font(&mut self, font: TextFont) {
        self.font = font;
    }
    pub fn set_shadow(&mut self, shadow: bool) {
        self.shadow = shadow;
    }
    pub fn toggle_shadow(&mut self) {
        self.shadow = !self.shadow;
    }
    pub fn set_stock(&mut self, stock: bool) {
        self.stock = stock;
    }
    pub fn toggle_stock(&mut self) {
        self.stock = !self.stock;
    }
    pub fn set_centre_after(&mut self, centre_after: bool) {
        self.centre_after = centre_after;
    }
    pub fn toggle_centre_after(&mut self) {
        self.centre_after = !self.centre_after;
    }
}
