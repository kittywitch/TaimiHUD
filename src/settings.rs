use {
    crate::{controller::ProgressBarStyleChange, render::TextFont, SETTINGS},
    anyhow::anyhow,
    async_compression::tokio::bufread::GzipDecoder,
    chrono::{DateTime, Utc},
    futures::stream::{StreamExt, TryStreamExt},
    octocrab::models::repos::Release,
    reqwest::{Client, IntoUrl, Response},
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        fmt, io,
        path::{Path, PathBuf},
        sync::Arc,
    },
    tokio::{
        fs::{create_dir_all, read_to_string, try_exists, File},
        io::AsyncWriteExt,
        sync::RwLock,
    },
    tokio_tar::Archive,
    tokio_util::io::StreamReader,
};

pub type SettingsLock = Arc<RwLock<Settings>>;

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
    pub fn toggle(&mut self) -> bool {
        self.disabled = !self.disabled;
        self.disabled
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct RemoteState {
    pub source: Arc<RemoteSource>,
    installed_tag: Option<String>,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct GitHubSource {
    pub owner: String,
    pub repository: String,
}

impl fmt::Display for GitHubSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repository)
    }
}

impl GitHubSource {
    fn folder_name(&self) -> String {
        format!("{}_{}", self.owner, self.repository)
    }

    async fn get<U: IntoUrl>(url: U) -> anyhow::Result<Response> {
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let user_agent = format!("{} by {}", name, authors);
        let client = Client::builder().user_agent(user_agent).build()?;
        Ok(client.get(url).send().await?)
    }

    async fn get_and_extract_tar<U: IntoUrl>(dir: &Path, url: U) -> anyhow::Result<()> {
        let response = Self::get(url).await?;
        let bytes_stream = response.bytes_stream().map_err(io::Error::other);
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
                if let Some(destination_parent) = destination_path.parent() {
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

    pub async fn download_latest(&self, install_dir: &Path) -> anyhow::Result<String> {
        let latest = self.latest_release().await?;
        if let Some(tarball_url) = latest.tarball_url {
            Self::get_and_extract_tar(install_dir, tarball_url).await?;
        }
        Ok(latest.tag_name)
    }

    pub async fn latest_release(&self) -> anyhow::Result<Release> {
        Ok(octocrab::instance()
            .repos(&self.owner, &self.repository)
            .releases()
            .get_latest()
            .await?)
    }

    async fn latest_id(&self) -> anyhow::Result<String> {
        Ok(self.latest_release().await?.tag_name)
    }
}
/*#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
enum RemoteSource {
    GitHub(GitHubSource),
}*/

pub type RemoteSource = GitHubSource;

impl RemoteState {
    fn new(owner: &str, repository: &str) -> Self {
        Self {
            source: Arc::new(RemoteSource {
                owner: owner.to_string(),
                repository: repository.to_string(),
            }),
            installed_tag: Default::default(),
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

    pub async fn needs_update(&self) -> NeedsUpdate {
        use NeedsUpdate::*;
        if let Ok(remote_release_id) = self.source.latest_id().await {
            if let Some(release_id) = &self.installed_tag {
                Known(*release_id != remote_release_id, remote_release_id)
            } else {
                Known(true, remote_release_id)
            }
        } else {
            Unknown
        }
    }
    pub async fn commit_downloaded(
        &mut self,
        tag_name: String,
        install_dir: PathBuf,
    ) -> anyhow::Result<()> {
        self.installed_tag = Some(tag_name);
        self.needs_update = self.needs_update().await;
        self.installed_path = Some(install_dir);
        Ok(())
    }
}

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
    fn set_height(&mut self, height: f32) {
        self.height = height;
    }
    fn set_font(&mut self, font: TextFont) {
        self.font = font;
    }
    fn set_shadow(&mut self, shadow: bool) {
        self.shadow = shadow;
    }
    fn toggle_shadow(&mut self) {
        self.shadow = !self.shadow;
    }
    fn set_stock(&mut self, stock: bool) {
        self.stock = stock;
    }
    fn toggle_stock(&mut self) {
        self.stock = !self.stock;
    }
    fn set_centre_after(&mut self, centre_after: bool) {
        self.centre_after = centre_after;
    }
    fn toggle_centre_after(&mut self) {
        self.centre_after = !self.centre_after;
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Settings {
    #[serde(default)]
    pub last_checked: Option<DateTime<Utc>>,
    #[serde(skip)]
    addon_dir: PathBuf,
    #[serde(default)]
    pub timers: HashMap<String, TimerSettings>,
    #[serde(default)]
    pub remotes: Vec<RemoteState>,
    #[serde(default)]
    pub primary_window_open: bool,
    #[serde(default)]
    pub timers_window_open: bool,
    #[serde(default)]
    pub progress_bar: ProgressBarSettings,
    #[serde(default)]
    pub enable_katrender: bool,
}

impl Settings {
    pub fn get_paths(&self) -> Vec<&PathBuf> {
        self.remotes
            .iter()
            .filter_map(|dd| dd.installed_path.as_ref())
            .collect()
    }

    pub async fn set_window_state(&mut self, window: &str, state: bool) {
        let window_open = match window {
            "primary" => &mut self.primary_window_open,
            "timers" => &mut self.timers_window_open,
            _ => unreachable!("unsupported window"),
        };

        *window_open = state;
        let _ = self.save(&self.addon_dir).await;
    }

    pub async fn toggle_timer(&mut self, timer: String) -> bool {
        let entry = self.timers.entry(timer.clone()).or_default();
        let new_state = entry.toggle();
        let irrelevant = entry == &Default::default();
        if irrelevant {
            self.timers.remove(&timer);
        }
        let _ = self.save(&self.addon_dir).await;
        new_state
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

    #[allow(dead_code)]
    pub async fn get_status_for(&self, source: &RemoteSource) -> Option<&RemoteState> {
        self.remotes.iter().find(|dd| *dd.source == *source)
    }

    pub async fn get_status_for_mut(&mut self, source: &RemoteSource) -> Option<&mut RemoteState> {
        self.remotes.iter_mut().find(|dd| *dd.source == *source)
    }

    pub async fn download_latest(source: &RemoteSource) -> anyhow::Result<()> {
        let settings_arc = SETTINGS
            .get()
            .expect("SettingsLock should've been initialized by now!");
        let install_dir = {
            let settings_read_lock = settings_arc.read().await;
            settings_read_lock.addon_dir.join(source.folder_name())
        };
        let tag_name = source.download_latest(&install_dir).await?;
        {
            let mut settings_write_lock = settings_arc.write().await;
            if let Some(dd_mut) = settings_write_lock.get_status_for_mut(source).await {
                let res = dd_mut.commit_downloaded(tag_name, install_dir).await;
                let _ = settings_write_lock
                    .save(&settings_write_lock.addon_dir)
                    .await;
                res
            } else {
                Err(anyhow!("GitHub repository \"{}\" not found.", source))
            }
        }?;
        Ok(())
    }

    pub async fn set_progress_bar(&mut self, style: ProgressBarStyleChange) -> ProgressBarSettings {
        use ProgressBarStyleChange::*;
        match style {
            Centre(t) => self.progress_bar.set_centre_after(t),
            Stock(t) => self.progress_bar.set_stock(t),
            Shadow(t) => self.progress_bar.set_shadow(t),
            Height(h) => self.progress_bar.set_height(h),
            Font(f) => self.progress_bar.set_font(f),
        }
        let _ = self.save(&self.addon_dir).await;
        self.progress_bar.clone()
    }

    pub async fn toggle_katrender(&mut self) {
        self.enable_katrender = !self.enable_katrender;
    }

    pub async fn check_for_updates() -> anyhow::Result<()> {
        let settings_arc = SETTINGS
            .get()
            .expect("SettingsLock should've been initialized by now!");
        let sources: Vec<(Arc<RemoteSource>, NeedsUpdate)> = {
            let settings_read_lock = settings_arc.read().await;
            tokio_stream::iter(settings_read_lock.remotes.iter())
                .then(|r| async move { (r.source.clone(), r.needs_update().await) })
                .collect()
                .await
        };
        {
            let mut settings_write_lock = settings_arc.write().await;
            for (source, nu) in sources {
                log::debug!("{} update state: {:?}", source, nu);
                if let Some(dd) = settings_write_lock.get_status_for_mut(&source).await {
                    log::debug!("Found dd {} update state: {:?}", dd.source, nu);
                    dd.needs_update = nu;
                }
            }
            settings_write_lock.last_checked = Some(Utc::now());
            settings_write_lock
                .save(&settings_write_lock.addon_dir)
                .await?;
        }
        Ok(())
    }

    pub async fn new(addon_dir: &Path) -> Self {
        Self {
            last_checked: None,
            addon_dir: addon_dir.to_path_buf(),
            timers: Default::default(),
            remotes: RemoteState::suggested_sources().collect(),
            progress_bar: Default::default(),
            timers_window_open: false,
            primary_window_open: false,
            enable_katrender: false,
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
        match Settings::load(addon_dir).await {
            Ok(settings) => settings,
            Err(err) => {
                log::error!("SettingsLock load error: {}", err);
                Self::new(addon_dir).await
            }
        }
    }

    pub async fn load_access(addon_dir: &Path) -> SettingsLock {
        Arc::new(RwLock::new(Self::load_default(addon_dir).await))
    }

    pub async fn save(&self, addon_dir: &Path) -> anyhow::Result<()> {
        create_dir_all(addon_dir).await?;
        let settings_path = addon_dir.join("settings.json");
        log::debug!("Settings: Saving to \"{:?}\".", settings_path);
        let settings_str = serde_json::to_string(self)?;
        let mut file = File::create(settings_path).await?;
        file.write_all(settings_str.as_bytes()).await?;
        Ok(())
    }
}
