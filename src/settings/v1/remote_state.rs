use {
    crate::{
        settings::{source::Source, GitHubSource, NeedsUpdate, RemoteSource},
        timer::TimerFile,
    },
    futures::stream::StreamExt,
    serde::{Deserialize, Serialize},
    std::{
        path::PathBuf,
        sync::Arc,
    },
    tokio::fs::remove_dir_all,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct RemoteState {
    pub source: Arc<RemoteSource>,
    pub installed_tag: Option<String>,
    pub installed_path: Option<PathBuf>,
    #[serde(skip)]
    pub needs_update: NeedsUpdate,
}

impl RemoteState {
    pub fn new(owner: &str, repository: &str, description: &str) -> Self {
        Self {
            source: Arc::new(RemoteSource::GitHub(GitHubSource {
                owner: owner.to_string(),
                repository: repository.to_string(),
                description: Some(description.to_string()),
            })),
            installed_tag: Default::default(),
            installed_path: Default::default(),
            needs_update: Default::default(),
        }
    }

    pub fn new_from_source(source: &RemoteSource) -> Self {
        let source = Arc::new(source.clone());
        Self {
            source,
            installed_tag: Default::default(),
            installed_path: Default::default(),
            needs_update: Default::default(),
        }
    }

    pub fn source(&self) -> GitHubSource {
        self.source.source()
    }

    pub async fn load(&self) -> Vec<Arc<TimerFile>> {
        let association = self.source.clone();
        if let Some(path) = &self.installed_path {
            TimerFile::load_many(path, association, 100)
                .await
                .expect("Could not load timer file for source {self.source}")
        } else {
            Default::default()
        }
    }

    pub fn update(&mut self, source: Arc<RemoteSource>) {
        self.source = source;
    }

    pub async fn uninstall(&mut self) -> anyhow::Result<()> {
        // fuck man, be careful o:
        if let Some(path) = &self.installed_path {
            if path.exists() {
                log::warn!("Uninstalling: removing {path:?}!");
                remove_dir_all(path).await?;
            } else {
                log::warn!("Uninstalling: {path:?} no longer exists.");
            }
        }
        self.installed_tag = None;
        self.installed_path = None;
        self.needs_update = NeedsUpdate::Unknown;
        Ok(())
    }

    pub fn hardcoded_sources() -> Vec<(&'static str, &'static str, &'static str)> {
        let hardcoded_sources = [
            ("kittywitch", "Hero-Timers", "The author of this mod's fork of the below; changes such as Sabetha markers and others planned, specific to this addon."),
            ("QuitarHero", "Hero-Timers", "The OG timer pack for BlishHUD!")
        ];
        hardcoded_sources.into()
    }
    pub fn suggested_sources() -> impl Iterator<Item = Self> {
        Self::hardcoded_sources()
            .into_iter()
            .map(|(owner, repository, description)| Self::new(owner, repository, description))
    }

    pub async fn needs_update(&self) -> NeedsUpdate {
        use NeedsUpdate::*;
        let source = self.source();
        let remote_id = source.latest_id().await;
        log::debug!("{:?}", remote_id);
        match remote_id {
            Ok(rid) => {
                if let Some(lid) = &self.installed_tag {
                    Known(*lid != rid, rid)
                } else {
                    Known(true, rid)
                }
            }
            Err(err) => {
                log::error!("Update check failed: {}", err);
                NeedsUpdate::Error(err.to_string())
            }
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
