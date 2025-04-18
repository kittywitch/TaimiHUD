use {
    crate::{
        settings::{RemoteSource, Settings, SettingsLock},
        timer::{Position, TimerFile, TimerMachine},
        MumbleIdentityUpdate, RenderEvent, SETTINGS,
    }, arcdps::{evtc::event::Event as arcEvent, AgentOwned}, glam::f32::Vec3, glob::{glob, Paths}, nexus::{data_link::{mumble::UiState, read_mumble_link, MumbleLink}, texture::{load_texture_from_file, RawTextureReceiveCallback}, texture_receive}, relative_path::RelativePathBuf, std::{
        collections::HashMap,
        fs::read_to_string,
        path::{Path, PathBuf},
        sync::Arc, time::SystemTime,
    }, strum_macros::Display, tokio::{
        runtime, select,
        sync::{
            mpsc::{Receiver, Sender},
            Mutex,
        },
        time::{interval, Duration},
    }
};

#[derive(Debug, Clone)]
pub struct Controller {
    pub agent: Option<AgentOwned>,
    pub previous_combat_state: bool,
    pub rt_sender: Sender<RenderEvent>,
    pub cached_identity: Option<MumbleIdentityUpdate>,
    pub cached_link: Option<MumbleLink>,
    pub map_id: Option<u32>,
    pub player_position: Option<Vec3>,
    alert_sem: Arc<Mutex<()>>,
    pub timers: Vec<Arc<TimerFile>>,
    pub current_timers: Vec<TimerMachine>,
    pub map_id_to_timers: HashMap<u32, Vec<Arc<TimerFile>>>,
    settings: SettingsLock,
}

impl Controller {
    pub fn player_position(&self) -> Option<Position> {
        self.player_position.map(Position::Vec3)
    }

    pub fn load(
        mut tm_receiver: Receiver<ControllerEvent>,
        rt_sender: Sender<crate::RenderEvent>,
        addon_dir: PathBuf,
    ) {
        let evt_loop = async move {
            let settings = Settings::load_access(&addon_dir.clone()).await;
            let mut state = Controller {
                previous_combat_state: Default::default(),
                rt_sender,
                settings,
                agent: Default::default(),
                cached_identity: Default::default(),
                cached_link: Default::default(),
                map_id: Default::default(),
                player_position: Default::default(),
                alert_sem: Default::default(),
                timers: Default::default(),
                current_timers: Default::default(),
                map_id_to_timers: Default::default(),
            };
            let _ = SETTINGS.set(state.settings.clone());
            state.setup_timers().await;
            let mut taimi_interval = interval(Duration::from_millis(125));
            let mut mumblelink_interval = interval(Duration::from_millis(20));
            loop {
                select! {
                    evt = tm_receiver.recv() => match evt {
                        Some(evt) => {
                            match state.handle_event(evt).await {
                                Ok(true) => (),
                                Ok(false) => break,
                                Err(error) => {
                                    log::error!("Error! {}", error)
                                }
                            }
                        },
                        None => {
                            break
                        },
                    },
                    _ = mumblelink_interval.tick() => {
                        let _ = state.mumblelink_tick().await;
                    },
                    _ = taimi_interval.tick() => {
                        let _ = state.tick().await;
                    },
                }
            }
        };
        let rt = match runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(error) => {
                log::error!("Error! {}", error);
                return;
            }
        };
        rt.block_on(evt_loop);
    }
    async fn load_timer_file(&self, path: PathBuf) -> anyhow::Result<TimerFile> {
        log::debug!("Attempting to load the timer file at '{path:?}'.");
        //let file = File::open(path)?;
        //let timer_data: TimerFile = serde_jsonrc::from_reader(file)?;
        let mut file_data = read_to_string(&path)?;
        json_strip_comments::strip(&mut file_data)?;
        let mut timer_data: TimerFile = serde_json::from_str(&file_data)?;
        timer_data.path = Some(path);
        Ok(timer_data)
    }

    async fn get_paths(&self, path: &Path) -> anyhow::Result<Paths> {
        let timer_paths: Paths = glob(path.to_str().expect("Pattern is unparseable"))?;
        Ok(timer_paths)
    }

    async fn load_timer_files(&self) -> Vec<Arc<TimerFile>> {
        let mut timers = Vec::new();
        let settings_lock = self.settings.read().await;
        let paths_to_try = settings_lock.get_paths();
        let mut paths = Vec::new();
        for path in paths_to_try {
            let glob_str = path.join("**/*.bhtimer");
            log::info!("A path to load timer files from is '{glob_str:?}'.");
            let timer_paths: Paths = self.get_paths(&glob_str).await.unwrap();
            paths.extend(timer_paths);
        }
        drop(settings_lock);
        let mut total_files = 0;
        for path in paths {
            total_files += 1;
            let path = path.expect("Path illegible!");
            match self.load_timer_file(path.clone()).await {
                Ok(data) => {
                    log::debug!("Successfully loaded the timer file at '{path:?}'.");
                    timers.push(Arc::new(data));
                }
                Err(error) => log::warn!("Failed to load the timer file at '{path:?}': {error}."),
            };
        }
        let timers_len = timers.len();
        log::info!(
            "Loaded {} out of {} timers successfully. {} failed to load.",
            timers_len,
            total_files,
            total_files - timers_len
        );
        timers
    }

    async fn setup_timers(&mut self) {
        log::info!("Preparing to setup timers");
        self.timers = self.load_timer_files().await;
        for timer in &self.timers {
            // Handle map to timers
            self.map_id_to_timers.entry(timer.map_id).or_default();
            if let Some(val) = self.map_id_to_timers.get_mut(&timer.map_id) {
                val.push(timer.clone());
            };
            // Handle id to timer file allocation
            log::info!(
                "Set up {0}: {3} for map {1}, category {2}",
                timer.id,
                timer.name.replace("\n", " "),
                timer.map_id,
                timer.category
            );
        }
        log::info!("Set up {} timers.", self.timers.len());
        let _ = self
            .rt_sender
            .send(RenderEvent::TimerData(self.timers.clone()))
            .await;
    }

    async fn tick(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn mumblelink_tick(&mut self) -> anyhow::Result<()> {
        if let Some(mumble_link_data) = read_mumble_link() {
            self.player_position = Some(Vec3::from_array(mumble_link_data.avatar.position));
            if let Some(pos) = self.player_position() {
                for machine in &mut self.current_timers {
                    machine.tick(pos).await
                }
            }
            let combat_state = mumble_link_data.context.ui_state.contains(UiState::IS_IN_COMBAT);
            if combat_state != self.previous_combat_state {
                if combat_state {
                    log::info!("MumbleLink: Combat begins at {:?}!", SystemTime::now());
                    for machine in &mut self.current_timers {
                        machine.combat_entered()
                    }
                } else {
                    log::info!("MumbleLink: Combat ends at {:?}!", SystemTime::now());
                    for machine in &mut self.current_timers {
                        machine.combat_exited()
                    }
                }
                self.previous_combat_state = combat_state;
            }
            self.cached_link = Some(mumble_link_data);
        }
        Ok(())
    }

    async fn handle_mumble(&mut self, identity: MumbleIdentityUpdate) {
        let new_map_id = identity.map_id;
        if Some(new_map_id) != self.map_id {
            for timer in &mut self.current_timers {
                timer.cleanup().await;
            }
            self.current_timers.clear();
            if self.map_id_to_timers.contains_key(&new_map_id) {
                let map_timers = &self.map_id_to_timers[&new_map_id];
                for timer in map_timers {
                    let settings_lock = self.settings.read().await;
                    let settings_for_timer = settings_lock.timers.get(&timer.id);
                    let timer_enabled = match settings_for_timer {
                        Some(setting) => !setting.disabled,
                        None => true,
                    };
                    if timer_enabled {
                        self.current_timers.push(TimerMachine::new(
                            timer.clone(),
                            self.alert_sem.clone(),
                            self.rt_sender.clone(),
                        ));
                    }
                    drop(settings_lock);
                }
                for machine in &mut self.current_timers {
                    machine.update_on_map(new_map_id)
                }
            }
            self.map_id = Some(new_map_id);
        }
        self.cached_identity = Some(identity);
    }

    async fn handle_combat_event(&mut self, src: arcdps::AgentOwned, evt: arcEvent) {
        let is_self = src.is_self != 0;
        if is_self {
            match &mut self.agent {
                Some(agent) if src.name != agent.name => {
                    log::info!("Character changed from {:?} to {:?}!", agent.name, src.name);
                    *agent = src;
                }
                Some(_agent) => (),
                None => {
                    log::info!("Character selected, {:?}!", src.name);
                    self.agent = Some(src);
                }
            };
        }
        use arcdps::StateChange;
        match evt.get_statechange() {
            StateChange::None => {}
            StateChange::EnterCombat => {
                log::info!("ArcDPS: Combat begins at {}!", evt.time);
                for machine in &mut self.current_timers {
                    machine.combat_entered()
                }
            }
            StateChange::ExitCombat => {
                log::info!("ArcDPS: Combat ends at {}!", evt.time);
                for machine in &mut self.current_timers {
                    machine.combat_exited()
                }
            }
            _ => (),
        }
    }

    async fn toggle_timer(&mut self, id: &str) {
        let mut settings_lock = self.settings.write().await;
        let disabled = settings_lock.toggle_timer(id.to_string()).await;
        drop(settings_lock);
        match disabled {
            false => {
                if let Some(map_id) = self.map_id {
                    if let Some(timers_for_map) = &self.map_id_to_timers.get(&map_id) {
                        let timers = timers_for_map.iter().filter(|t| t.id == id);
                        for timer in timers {
                            log::debug!("Creating timer machine for {} as it has been enabled.", timer.id);
                            self.current_timers.push(TimerMachine::new(
                                timer.clone(),
                                self.alert_sem.clone(),
                                self.rt_sender.clone(),
                            ));
                        }
                    }
                }
            },
            true => {
                let timers_to_remove =
                    self.current_timers.iter_mut().filter(|t| t.timer.id == id);
                for timer in timers_to_remove {
                    log::debug!("Starting cleanup for timer {} as it has been disabled.", timer.timer.id);
                    timer.cleanup().await;
                }
            }
        }
    }

    async fn enable_timer(&mut self, id: &str) {
        let mut settings_lock = self.settings.write().await;
        settings_lock.enable_timer(id.to_string()).await;
        drop(settings_lock);
        if let Some(map_id) = self.map_id {
            if let Some(timers_for_map) = &self.map_id_to_timers.get(&map_id) {
                let timers = timers_for_map.iter().filter(|t| t.id == id);
                for timer in timers {
                    log::debug!("Creating timer machine for {}", timer.id);
                    self.current_timers.push(TimerMachine::new(
                        timer.clone(),
                        self.alert_sem.clone(),
                        self.rt_sender.clone(),
                    ));
                }
            }
        }
    }

    async fn disable_timer(&mut self, id: &str) {
        let mut settings_lock = self.settings.write().await;
        settings_lock.disable_timer(id.to_string()).await;
        drop(settings_lock);
        let timers_to_remove = self.current_timers.iter_mut().filter(|t| t.timer.id == id);
        for timer in timers_to_remove {
            log::debug!("Starting cleanup for timer {}", timer.timer.id);
            timer.cleanup().await;
        }
        self.current_timers.retain(|t| t.timer.id != id);
    }

    async fn check_updates(&mut self) {
        let _ = self
            .rt_sender
            .send(RenderEvent::CheckingForUpdates(true))
            .await;
        match Settings::check_for_updates().await {
            Ok(_) => (),
            Err(err) => log::error!("Controller.check_updates(): {}", err),
        }
        let _ = self
            .rt_sender
            .send(RenderEvent::CheckingForUpdates(false))
            .await;
    }

    async fn do_update(&mut self, source: &RemoteSource) {
        match Settings::download_latest(source).await {
            Ok(_) => (),
            Err(err) => log::error!("Controller.do_update() error for \"{}\": {}", source, err),
        };
        self.setup_timers().await;
    }

    async fn progress_bar_switch(&mut self, stock: bool) {
        let mut settings_lock = self.settings.write().await;
        settings_lock.set_progress_bar(stock).await;
        drop(settings_lock);
    }
    
    async fn set_window_state(&mut self, window: String, state: bool) {
        let mut settings_lock = self.settings.write().await;
        settings_lock.set_window_state(&window, state).await;
        drop(settings_lock);
    }

    async fn timer_key_trigger(&mut self, id: String, is_release: bool) {
        if !is_release {
            log::info!("{}", self.current_timers.len());
            for timer in &mut self.current_timers {
                timer.key_pressed(id.clone());
            }
        }
    }

    async fn load_texture(&self, rel: RelativePathBuf, base: PathBuf) {
        if let Some(base) = base.parent() {
            let abs = rel.to_path(base);
            let cally: RawTextureReceiveCallback = texture_receive!(|id, _texture| {
                log::info!("Texture {id} loaded.");
            });
            load_texture_from_file(rel.as_str(), abs, Some(cally));
        }
    }

    async fn reset_timers(&mut self) {
        for timer in &mut self.current_timers {
            timer.do_reset().await;
        }
    }

    async fn handle_event(&mut self, event: ControllerEvent) -> anyhow::Result<bool> {
        use ControllerEvent::*;
        log::debug!("Controller received event: {}", event);
        match event {
            MumbleIdentityUpdated(identity) => self.handle_mumble(identity).await,
            CombatEvent { src, evt } => self.handle_combat_event(src, evt).await,
            TimerEnable(id) => self.enable_timer(&id).await,
            TimerDisable(id) => self.disable_timer(&id).await,
            TimerToggle(id) => self.toggle_timer(&id).await,
            TimerReset => self.reset_timers().await,
            CheckDataSourceUpdates => self.check_updates().await,
            TimerKeyTrigger(id, is_release) => self.timer_key_trigger(id, is_release).await,
            DoDataSourceUpdate { source } => self.do_update(&source).await,
            ProgressBarStyle(stock) => self.progress_bar_switch(stock).await,
            WindowState(window, state) => self.set_window_state(window, state).await,
            LoadTexture(rel, base) => self.load_texture(rel, base).await,
            Quit => return Ok(false),
            // I forget why we needed this, but I think it's a holdover from the buttplug one o:
            //_ => (),
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Display)]
pub enum ControllerEvent {
    MumbleIdentityUpdated(MumbleIdentityUpdate),
    CombatEvent {
        src: arcdps::AgentOwned,
        evt: arcEvent,
    },
    DoDataSourceUpdate {
        source: Arc<RemoteSource>,
    },
    ProgressBarStyle(bool),
    WindowState(String, bool),
    TimerKeyTrigger(String, bool),
    LoadTexture(RelativePathBuf, PathBuf),
    CheckDataSourceUpdates,
    #[allow(dead_code)]
    TimerEnable(String),
    #[allow(dead_code)]
    TimerDisable(String),
    TimerReset,
    TimerToggle(String),
    Quit,
}
