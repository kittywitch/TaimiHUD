use {
    async_compression::tokio::bufread::GzipDecoder,
    bytes::Bytes,
    futures::stream::TryStreamExt,
    octocrab::models::{repos::Release, ReleaseId},
    reqwest::{Client, IntoUrl, Response},
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap, ffi::{OsStr, OsString}, fmt, fs, io::{self, prelude::*, Cursor, ErrorKind, Read, SeekFrom, Write}, path::{Path, PathBuf}, sync::Arc
    },
    tempfile::{Builder, TempDir},
    tokio::{
        fs::{create_dir, create_dir_all, read_dir, read_to_string, try_exists, File, OpenOptions},
        io::{copy, AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
        sync::RwLock,
    },
    tokio_stream::*,
    tokio_tar::Archive,
    tokio_util::io::StreamReader,
};

pub type Settings = Arc<RwLock<SettingsRaw>>;

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
pub struct TimerSettings {
    #[serde(default)]
    pub disabled: bool,
}

impl TimerSettings {
    fn disable(&mut self) {
        self.disabled = true;
    }
    fn enable(&mut self) {
        self.disabled = false;
    }
    pub fn toggle(&mut self) {
        self.disabled = !self.disabled;
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
pub struct DownloadData {
    pub owner: String,
    pub repository: String,
    last_release_id: Option<String>,
    installed_path: Option<PathBuf>,
    #[serde(skip)]
    pub needs_update: NeedsUpdate,
}

#[derive(PartialEq, Clone, Debug, Default)]
pub enum NeedsUpdate {
    #[default]
    Unknown,
    Known(bool, String),
}

impl fmt::Display for NeedsUpdate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use NeedsUpdate::*;
        match &self {
            Unknown => write!(f, "Unknown!"),
            Known(true, id) => write!(f, "Newer version, {} available!", id),
            Known(false, _id) => write!(f, "Up to date!"),
        }
    }
}

impl DownloadData {
    fn new(owner: &str, repository: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repository: repository.to_string(),
            last_release_id: Default::default(),
            installed_path: Default::default(),
            needs_update: Default::default(),
        }
    }

    fn suggested_sources() -> impl Iterator<Item = Self> {
        let hardcoded_sources = [("QuitarHero", "Hero-Timers")];
        hardcoded_sources
            .into_iter()
            .map(|(owner, repository)| Self::new(owner, repository))
    }

    pub async fn check_for_updates(&mut self) {
        self.needs_update = self.needs_update().await;
    }

    pub async fn needs_update(&self) -> NeedsUpdate {
        use NeedsUpdate::*;
        if let Ok(remote_release_id) = self.get_latest_id().await {
            if let Some(release_id) = &self.last_release_id {
                Known(*release_id != remote_release_id, remote_release_id)
            } else {
                Known(true, remote_release_id)
            }
        } else {
            Unknown
        }
    }

    async fn get_latest_id(&self) -> anyhow::Result<String> {
        Ok(self.get_latest_release().await?.tag_name)
    }

    async fn get<U: IntoUrl>(url: U) -> anyhow::Result<Response> {
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let user_agent = format!("{} by {}", name, authors);
        let client = Client::builder().user_agent(user_agent).build()?;
        Ok(client.get(url).send().await?)
    }

    async fn get_and_extract_tar<U: IntoUrl>(&self, dir: &Path, url: U) -> anyhow::Result<()> {
        let response = Self::get(url).await?;
        let bytes_stream = response
            .bytes_stream()
            .map_err(io::Error::other);
        let stream_reader = StreamReader::new(bytes_stream);
        let gzip_decoder = GzipDecoder::new(stream_reader);
        let mut tar_file = Archive::new(gzip_decoder);
        let entries = tar_file.entries()?;
        let mut containing_directory: Option<PathBuf> = None;
        let mut iterator = entries;
        iterator.next().await; // skip pax_global_header
        while let Some(file) = iterator.next().await {
            let mut f = file?;
            let path = f.path()?;
            log::debug!("Path in tarball: {}", path.display());
            if let Some(prefix) = &containing_directory {
                let destination_suffix = path.strip_prefix(prefix)?;
                log::debug!("Destination suffix: {}", destination_suffix.display());
                let destination_path = dir.join(destination_suffix);
                if let Some(destination_parent)= destination_path.parent() {
                    create_dir_all(destination_parent).await?;
                    f.unpack(destination_path).await?;
                    //f.unpack_in(destination).await?;
                }
            } else {
                containing_directory = Some(path.into_owned());
            }
        }
        Ok(())
    }

    pub async fn get_latest_release(&self) -> anyhow::Result<Release> {
        Ok(octocrab::instance()
            .repos(&self.owner, &self.repository)
            .releases()
            .get_latest()
            .await?)
    }

    pub async fn download_latest(&mut self, addon_dir: &Path) -> anyhow::Result<()> {
        let latest = self.get_latest_release().await?;
        let addon_folder_name = format!("{}_{}", self.owner, self.repository);
        let install_dir = addon_dir.join(addon_folder_name);
        if let Some(tarball_url) = latest.tarball_url {
            self.get_and_extract_tar(&install_dir, tarball_url).await?;
        }
        self.last_release_id = Some(latest.tag_name);
        self.needs_update = self.needs_update().await;
        self.installed_path = Some(install_dir);
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct SettingsRaw {
    #[serde(skip)]
    addon_dir: PathBuf,
    #[serde(default)]
    pub timers: HashMap<String, TimerSettings>,
    #[serde(default)]
    pub downloaded_releases: Vec<DownloadData>,
}

impl SettingsRaw {
    pub fn get_paths(&self) -> Vec<&PathBuf> {
        self.downloaded_releases
            .iter()
            .filter_map(|dd| dd.installed_path.as_ref())
            .collect()
    }

    pub async fn toggle_timer(&mut self, timer: String) {
        let entry = self.timers.entry(timer.clone()).or_default();
        entry.toggle();
        let irrelevant = entry == &Default::default();
        if irrelevant {
            self.timers.remove(&timer);
        }
        let _ = self.save(&self.addon_dir).await;
    }
    pub async fn disable_timer(&mut self, timer: String) {
        if let Some(entry_mut) = self.timers.get_mut(&timer) {
            entry_mut.disable();
        } else {
            self.timers.insert(timer, TimerSettings { disabled: true });
        }
        let _ = self.save(&self.addon_dir).await;
    }
    pub async fn enable_timer(&mut self, timer: String) {
        if let Some(entry_mut) = self.timers.get_mut(&timer) {
            entry_mut.enable();
        } else {
            self.timers.insert(timer, TimerSettings::default());
        }
        let _ = self.save(&self.addon_dir).await;
    }

    pub async fn download_latest(&mut self, owner: String, repository: String) {
        for release in self
            .downloaded_releases
            .iter_mut()
            .filter(|dd| dd.owner == owner && dd.repository == repository)
        {
            match release.download_latest(&self.addon_dir).await {
                Ok(_) => (),
                Err(err) => log::error!("{}", err),
            }
        }
        let _ = self.save(&self.addon_dir).await;
    }

    pub async fn new(addon_dir: &Path) -> Self {
        Self {
            addon_dir: addon_dir.to_path_buf(),
            timers: Default::default(),
            downloaded_releases: DownloadData::suggested_sources().collect(),
        }
    }
    pub async fn load(addon_dir: &Path) -> anyhow::Result<Self> {
        let settings_path = addon_dir.join("settings.json");
        if try_exists(&settings_path).await? {
            let file_data = read_to_string(settings_path).await?;
            let mut settings = serde_json::from_str::<Self>(&file_data)?;
            settings.addon_dir = addon_dir.to_path_buf();
            return Ok(settings);
        }
        Ok(Self::new(addon_dir).await)
    }

    pub async fn load_default(addon_dir: &Path) -> Self {
        match SettingsRaw::load(addon_dir).await {
            Ok(settings) => settings,
            Err(err) => {
                log::error!("Settings load error: {}", err);
                Self::new(addon_dir).await
            }
        }
    }

    pub async fn load_access(addon_dir: &Path) -> Settings {
        Arc::new(RwLock::new(Self::load_default(addon_dir).await))
    }

    pub async fn save(&self, addon_dir: &Path) -> anyhow::Result<()> {
        let settings_path = addon_dir.join("settings.json");
        let settings_str = serde_json::to_string(self)?;
        let mut file = File::create(settings_path).await?;
        file.write_all(settings_str.as_bytes()).await?;
        Ok(())
    }
}
