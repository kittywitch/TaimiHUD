use {
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        fs::{exists, read_to_string, File},
        io::Write,
        path::{Path, PathBuf},
        sync::Arc,
    },
    tokio::sync::RwLock,
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

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct SettingsRaw {
    #[serde(skip)]
    addon_dir: PathBuf,
    #[serde(default)]
    pub timers: HashMap<String, TimerSettings>,
}

impl SettingsRaw {
    pub fn toggle_timer(&mut self, timer: String) {
        let entry = self.timers.entry(timer.clone()).or_default();
        entry.toggle();
        let irrelevant = entry == &Default::default();
        if irrelevant {
            self.timers.remove(&timer);
        }
        let _ = self.save(&self.addon_dir);
    }
    pub fn disable_timer(&mut self, timer: String) {
        if let Some(entry_mut) = self.timers.get_mut(&timer) {
            entry_mut.disable();
        } else {
            self.timers.insert(timer, TimerSettings { disabled: true });
        }
        let _ = self.save(&self.addon_dir);
    }
    pub fn enable_timer(&mut self, timer: String) {
        if let Some(entry_mut) = self.timers.get_mut(&timer) {
            entry_mut.enable();
        } else {
            self.timers.insert(timer, TimerSettings::default());
        }
        let _ = self.save(&self.addon_dir);
    }

    pub fn load(addon_dir: &Path) -> anyhow::Result<Self> {
        let settings_path = addon_dir.join("settings.json");
        if exists(&settings_path)? {
            let file_data = read_to_string(settings_path)?;
            return Ok(serde_json::from_str::<Self>(&file_data)?);
        }
        Ok(Self {
            addon_dir: addon_dir.to_path_buf(),
            timers: Default::default(),
        })
    }

    pub fn load_default(addon_dir: &Path) -> Self {
        match SettingsRaw::load(addon_dir) {
            Ok(settings) => settings,
            Err(err) => {
                log::error!("Settings load error: {}", err);
                Self {
                    addon_dir: addon_dir.to_path_buf(),
                    timers: Default::default(),
                }
            }
        }
    }

    pub fn load_access(addon_dir: &Path) -> Settings {
        Arc::new(RwLock::new(Self::load_default(addon_dir)))
    }

    pub fn save(&self, addon_dir: &Path) -> anyhow::Result<()> {
        let settings_path = addon_dir.join("settings.json");
        let settings_str = serde_json::to_string(self)?;
        let mut file = File::create(settings_path)?;
        file.write_all(settings_str.as_bytes())?;
        Ok(())
    }
}
