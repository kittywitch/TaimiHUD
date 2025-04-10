use {
    crate::{
        geometry::Position, timer::timerfile::TimerFile, timermachine::TimerMachine,
        RenderThreadEvent, *,
    },
    arcdps::{evtc::event::Event as arcEvent, AgentOwned},
    glam::f32::Vec3,
    glob::{glob, Paths},
    nexus::data_link::{read_mumble_link, MumbleLink},
    std::{
        collections::HashMap,
        fs::read_to_string,
        path::{Path, PathBuf},
        sync::Arc,
    },
    tokio::{
        runtime, select,
        sync::{
            mpsc::{Receiver, Sender},
            Mutex,
        },
        task::JoinHandle,
        time::{interval, sleep, Duration},
    },
};

#[derive(Debug, Clone)]
pub struct TaimiState {
    pub agent: Option<AgentOwned>,

    pub rt_sender: Sender<RenderThreadEvent>,
    pub addon_dir: PathBuf,
    pub cached_identity: Option<MumbleIdentityUpdate>,
    pub cached_link: Option<MumbleLink>,
    pub map_id: Option<u32>,
    pub player_position: Option<Vec3>,
    alert_sem: Arc<Mutex<()>>,
    pub timers: Vec<Arc<TimerFile>>,
    pub current_timers: Vec<TimerMachine>,
    pub map_id_to_timers: HashMap<u32, Vec<Arc<TimerFile>>>,
    pub category_to_timers: HashMap<String, Vec<Arc<TimerFile>>>,
}

impl TaimiState {
    pub fn player_position(&self) -> Option<Position> {
        match self.player_position {
            Some(pos) => Some(Position::Vec3(pos)),
            None => None,
        }
    }

    pub fn load(
        mut tm_receiver: Receiver<TaimiThreadEvent>,
        rt_sender: Sender<crate::RenderThreadEvent>,
        addon_dir: PathBuf,
    ) {
        let mut state = TaimiState {
            addon_dir,
            rt_sender,
            agent: Default::default(),
            cached_identity: Default::default(),
            cached_link: Default::default(),
            map_id: Default::default(),
            player_position: Default::default(),
            alert_sem: Default::default(),

            // originally timer_ids
            timers: Default::default(),
            current_timers: Default::default(),
            map_id_to_timers: Default::default(),
            category_to_timers: Default::default(),
        };

        let evt_loop = async move {
            state.setup_timers().await;
            let mut taimi_interval = interval(Duration::from_millis(250));
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
        let mut file_data = read_to_string(path)?;
        json_strip_comments::strip(&mut file_data)?;
        let timer_data: TimerFile = serde_json::from_str(&file_data)?;

        Ok(timer_data)
    }

    async fn get_paths(&self, path: &Path) -> anyhow::Result<Paths> {
        let timer_paths: Paths = glob(path.to_str().expect("Pattern is unparseable"))?;
        Ok(timer_paths)
    }

    async fn load_timer_files(&self) -> Vec<Arc<TimerFile>> {
        let mut timers = Vec::new();
        let glob_str = self.addon_dir.join("*.bhtimer");
        log::info!("Path to load timer files from is '{glob_str:?}'.");
        let timer_paths: Paths = self.get_paths(&glob_str).await.unwrap();
        let mut total_files = 0;
        for path in timer_paths {
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
            // Handle map_id to timer_id
            self.map_id_to_timers.entry(timer.map_id).or_default();
            if let Some(val) = self.map_id_to_timers.get_mut(&timer.map_id) {
                val.push(timer.clone());
            };
            // Handle category to timer_id list
            if !self.category_to_timers.contains_key(&timer.category) {
                self.category_to_timers
                    .insert(timer.category.clone(), Vec::new());
            }
            if let Some(val) = self.category_to_timers.get_mut(&timer.category) {
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
        log::info!("Set up {} timers.", self.timers.len())
    }

    #[allow(dead_code)]
    async fn send_alert(
        sender: Sender<RenderThreadEvent>,
        lock: Arc<Mutex<()>>,
        message: String,
        duration: Duration,
    ) {
        let alert_handle = lock.lock().await;
        let _ = sender.send(RenderThreadEvent::AlertStart(message)).await;
        sleep(duration).await;
        let _ = sender.send(RenderThreadEvent::AlertEnd).await;
        // this is my EMOTIONAL SUPPORT drop
        drop(alert_handle);
    }

    #[allow(dead_code)]
    fn alert(&self, message: String, duration: Duration) -> JoinHandle<()> {
        tokio::spawn(Self::send_alert(
            self.rt_sender.clone(),
            self.alert_sem.clone(),
            message,
            duration,
        ))
    }

    async fn tick(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn mumblelink_tick(&mut self) -> anyhow::Result<()> {
        self.cached_link = read_mumble_link();
        if let Some(link) = &self.cached_link {
            self.player_position = Some(Vec3::from_array(link.avatar.position));
            if let Some(pos) = self.player_position() {
                for machine in &mut self.current_timers {
                    machine.tick(pos)
                }
            }
        };
        Ok(())
    }

    async fn handle_mumble(&mut self, identity: MumbleIdentityUpdate) {
        let new_map_id = identity.map_id;
        if Some(new_map_id) != self.map_id {
            self.current_timers.clear();
            if self.map_id_to_timers.contains_key(&new_map_id) {
                let map_timers = self.map_id_to_timers[&new_map_id].clone();
                for timer in map_timers {
                    self.current_timers.push(TimerMachine::new(
                        timer,
                        self.alert_sem.clone(),
                        self.rt_sender.clone(),
                    ));
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
                log::info!("Combat begins at {}!", evt.time);
                for machine in &mut self.current_timers {
                    machine.combat_entered()
                }
            }
            StateChange::ExitCombat => {
                log::info!("Combat ends at {}!", evt.time);
                for machine in &mut self.current_timers {
                    machine.combat_exited()
                }
            }
            _ => (),
        }
    }

    async fn handle_event(&mut self, event: TaimiThreadEvent) -> anyhow::Result<bool> {
        use TaimiThreadEvent::*;
        match event {
            MumbleIdentityUpdated(identity) => self.handle_mumble(identity).await,
            CombatEvent { src, evt } => self.handle_combat_event(src, evt).await,
            Quit => return Ok(false),
            // I forget why we needed this, but I think it's a holdover from the buttplug one o:
            //_ => (),
        }
        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub enum TaimiThreadEvent {
    MumbleIdentityUpdated(MumbleIdentityUpdate),
    CombatEvent {
        src: arcdps::AgentOwned,
        evt: arcEvent,
    },
    Quit,
}
