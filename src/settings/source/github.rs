use {
    super::RemoteSource,
    crate::{controller::ProgressBarStyleChange, render::TextFont, settings::Source, SETTINGS},
    anyhow::anyhow,
    async_compression::tokio::bufread::GzipDecoder,
    chrono::{DateTime, Utc},
    futures::stream::{StreamExt, TryStreamExt},
    nexus::paths::get_addon_dir,
    reqwest::{Certificate, Client, IntoUrl, Response},
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::{
        collections::HashMap,
        fmt, io,
        path::{Path, PathBuf},
        sync::Arc,
    },
    tokio::{
        fs::{create_dir_all, read_to_string, remove_dir_all, try_exists, File},
        io::AsyncWriteExt,
        sync::RwLock,
    },
    tokio_tar::Archive,
    tokio_util::io::StreamReader,
    url::Url,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GitHubLatestRelease {
    url: Url,
    html_url: Url,
    assets_url: Url,
    upload_url: Url,
    tarball_url: Option<Url>,
    zipball_url: Option<Url>,
    id: usize,
    node_id: String,
    tag_name: String,
    target_commitish: String,
    name: Option<String>,
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    created_at: DateTime<Utc>,
    published_at: DateTime<Utc>,
    // i don't really care about these ><
    author: Value,
    assets: Value,
}

#[derive(Deserialize, Serialize, Debug, Hash, Eq, Clone, PartialEq)]
pub struct GitHubSource {
    pub owner: String,
    pub repository: String,
    pub description: Option<String>,
}

impl fmt::Display for GitHubSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repository)
    }
}

impl GitHubSource {
    pub fn repo_string(&self) -> String {
        format!("{}", self)
    }

    pub async fn latest_release(&self) -> anyhow::Result<GitHubLatestRelease> {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            self.repo_string()
        );
        let response = Self::get(url).await?;
        let json_data = response.text().await?;
        let data = serde_json::from_str::<GitHubLatestRelease>(&json_data)?;
        Ok(data)
    }
}

impl Source for GitHubSource {
    fn install_dir(&self) -> String {
        format!("{}_{}", self.owner, self.repository)
    }
    fn view_url(&self) -> String {
        format!("https://github.com/{}", self.repo_string())
    }
    async fn download_latest(&self) -> anyhow::Result<String> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let install_dir = addon_dir.join(self.install_dir());
        create_dir_all(&install_dir).await?;
        let latest = self.latest_release().await?;
        if let Some(tarball_url) = latest.tarball_url {
            Self::get_and_extract_tar(&install_dir, tarball_url).await?;
        }
        Ok(latest.tag_name)
    }

    async fn latest_id(&self) -> anyhow::Result<String> {
        let release = self.latest_release().await?;
        Ok(release.tag_name)
    }
}
