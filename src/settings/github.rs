use {
    crate::{controller::ProgressBarStyleChange, render::TextFont, SETTINGS}, anyhow::anyhow, async_compression::tokio::bufread::GzipDecoder, chrono::{DateTime, Utc}, futures::stream::{StreamExt, TryStreamExt}, reqwest::{Certificate, Client, IntoUrl, Response}, serde::{Deserialize, Serialize}, serde_json::Value, std::{
        collections::HashMap,
        fmt, io,
        path::{Path, PathBuf},
        sync::Arc,
    }, tokio::{
        fs::{create_dir_all, read_to_string, remove_dir_all, try_exists, File},
        io::AsyncWriteExt,
        sync::RwLock,
    }, tokio_tar::Archive, tokio_util::io::StreamReader, url::Url
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct GitHubSource {
    pub owner: String,
    pub repository: String,
    pub description: String,
}

impl fmt::Display for GitHubSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repository)
    }
}

impl GitHubSource {
    pub fn folder_name(&self) -> String {
        format!("{}_{}", self.owner, self.repository)
    }

    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo_string())
    }

    async fn get<U: IntoUrl>(url: U) -> anyhow::Result<Response> {
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let user_agent = format!("{} by {}", name, authors);
        let client = Client::builder().user_agent(user_agent).build()?;
        let resp = client.get(url).send().await?.error_for_status()?;
        Ok(resp)

    }

    async fn get_and_extract_tar<U: IntoUrl>(dir: &Path, url: U) -> anyhow::Result<()> {
        let url = url.into_url()?;
        log::debug!("Beginning to fetch and extract into {dir:?} from {:?}", url);
        let response = Self::get(url.clone()).await?;
        let bytes_stream = response.bytes_stream().map_err(io::Error::other);
        let stream_reader = StreamReader::new(bytes_stream);
        let gzip_decoder = GzipDecoder::new(stream_reader);
        let mut tar_file = Archive::new(gzip_decoder);
        let entries = tar_file.entries()?;
        let mut containing_directory: Option<PathBuf> = None;
        let mut iterator = entries;
        iterator.next().await; // skip pax_global_header
        if dir.exists() {
            log::info!("Directory {dir:?} exists already; removing prior to extraction.");
            remove_dir_all(dir).await?;
        }
        while let Some(file) = iterator.next().await {
            let mut f = file?;
            let path = f.path()?;
            if let Some(prefix) = &containing_directory {
                let destination_suffix = path.strip_prefix(prefix)?;
                let destination_path = dir.join(destination_suffix);
                if let Some(destination_parent) = destination_path.parent() {
                    create_dir_all(destination_parent).await?;
                    f.unpack(destination_path).await?;
                    //f.unpack_in(destination).await?;
                }
            } else {
                containing_directory = Some(path.into_owned());
            }
        }
        log::debug!("Completed fetching and extracting into {dir:?} from {:?}", url);
        Ok(())
    }

    pub async fn download_latest(&self, install_dir: &Path) -> anyhow::Result<String> {
        let latest = self.latest_release().await?;
        if let Some(tarball_url) = latest.tarball_url {
            Self::get_and_extract_tar(install_dir, tarball_url).await?;
        }
        Ok(latest.tag_name)
    }

    pub fn repo_string(&self) -> String {
        format!("{}/{}", self.owner, self.repository)
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

    pub async fn latest_id(&self) -> anyhow::Result<String> {
        let release = self.latest_release().await?;
        Ok(release.tag_name)
    }
}
