#[cfg(feature = "markers")]
use {
    crate::marker::{
        atomic::{CurrentPerspective, MarkerInputData, MinimapPlacement, ScreenPoint},
        format::{MarkerFile, MarkerSet, MarkerType, RuntimeMarkers},
    },
    itertools::Itertools,
    windows::Win32::{
        Foundation::{LPARAM, POINT, RECT, WPARAM},
        Graphics::Gdi::ClientToScreen,
        UI::{
            Input::KeyboardAndMouse::{GetActiveWindow, GetCapture},
            WindowsAndMessaging::{
                GetCursorPos, GetForegroundWindow, GetWindowRect, SetCursorPos, WM_MOUSEMOVE,
            },
        },
    },
};
use {
    crate::{
        render::TextFont,
        settings::{RemoteSource, RemoteState, Settings, SettingsLock, SourceKind, SourcesFile},
        timer::{CombatState, Position, TimerFile, TimerMachine},
        MumbleIdentityUpdate, RenderEvent, IMGUI_TEXTURES, SETTINGS, SOURCES,
    },
    arcdps::{evtc::event::Event as arcEvent, AgentOwned},
    glam::{f32::Vec3, Vec2},
    glamour::Point2,
    glob::{glob, Paths},
    nexus::{
        data_link::{
            get_mumble_link_ptr, get_nexus_link,
            mumble::{MumblePtr, UIScaling, UiState},
            read_nexus_link, MumbleLink, NexusLink,
        },
        gamebind::invoke_gamebind_async,
        imgui::{
            sys::{igSetCursorPos, ImVec2},
            Context,
        },
        paths::get_addon_dir,
        texture::{load_texture_from_file, RawTextureReceiveCallback},
        texture_receive,
        wnd_proc::send_wnd_proc_to_game,
    },
    relative_path::RelativePathBuf,
    std::{
        collections::HashMap,
        ffi::OsStr,
        fs::{exists, read_to_string},
        path::{Path, PathBuf},
        sync::{Arc, RwLock},
        time::SystemTime,
    },
    strum_macros::Display,
    tokio::{
        fs::create_dir_all,
        runtime, select,
        sync::{
            mpsc::{Receiver, Sender},
            Mutex,
        },
        time::{interval, sleep, Duration},
    },
};

#[cfg(feature = "space")]
use crate::space::dx11::PerspectiveInputData;

#[derive(Debug, Clone)]
pub struct Controller {
    pub agent: Option<AgentOwned>,
    pub previous_combat_state: bool,
    #[cfg(feature = "markers")]
    pub markers: HashMap<String, Vec<Arc<MarkerSet>>>,
    pub rt_sender: Sender<RenderEvent>,
    pub cached_identity: Option<MumbleIdentityUpdate>,
    pub mumble_pointer: Option<MumblePtr>,
    pub nexus_pointer: *const NexusLink,
    pub map_id: Option<u32>,
    pub player_position: Option<Vec3>,
    alert_sem: Arc<Mutex<()>>,
    pub timers: Vec<Arc<TimerFile>>,
    pub current_timers: Vec<TimerMachine>,
    pub sources_to_timers: HashMap<Arc<RemoteSource>, Vec<Arc<TimerFile>>>,
    pub map_id_to_timers: HashMap<u32, Vec<Arc<TimerFile>>>,
    settings: SettingsLock,
    last_fov: f32,
    scaling: f32,
}

impl Controller {
    pub fn player_position(&self) -> Option<Position> {
        self.player_position.map(Position::Vec3)
    }

    pub fn load(
        mut controller_receiver: Receiver<ControllerEvent>,
        rt_sender: Sender<crate::RenderEvent>,
        addon_dir: PathBuf,
    ) {
        let mumble_ptr = get_mumble_link_ptr() as *mut MumbleLink;
        let mumble_link = unsafe { MumblePtr::new(mumble_ptr) };
        let nexus_link = get_nexus_link();
        let evt_loop = async move {
            let sources = SourcesFile::load()
                .await
                .expect("Couldn't load sources file");
            let sources = Arc::new(RwLock::new(sources));
            let _ = SOURCES.set(sources);
            let settings = Settings::load_access(&addon_dir.clone()).await;
            let mut state = Controller {
                nexus_pointer: nexus_link,
                last_fov: 0.0,
                previous_combat_state: Default::default(),
                rt_sender,
                settings,
                #[cfg(feature = "markers")]
                markers: Default::default(),
                agent: Default::default(),
                cached_identity: Default::default(),
                mumble_pointer: mumble_link,
                map_id: Default::default(),
                player_position: Default::default(),
                alert_sem: Default::default(),
                timers: Default::default(),
                current_timers: Default::default(),
                sources_to_timers: Default::default(),
                map_id_to_timers: Default::default(),
                scaling: 0.0f32,
            };
            let _ = SETTINGS.set(state.settings.clone());
            let settings = SETTINGS.get().unwrap();
            let mut settings_lock = settings.write().await;
            settings_lock.handle_sources_changes();
            drop(settings_lock);
            state.setup_timers().await;
            #[cfg(feature = "markers")]
            match state.load_markers_files().await {
                Ok(()) => (),
                Err(err) => log::error!("Error loading markers: {}", err),
            }
            let mut taimi_interval = interval(Duration::from_millis(125));
            let mut mumblelink_interval = interval(Duration::from_millis(20));
            loop {
                select! {
                    evt = controller_receiver.recv() => match evt {
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

    /*async fn load_markers_file(&mut self) -> anyhow::Result<()> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let markers = MarkerFile::load(&addon_dir.join("Markers.json")).await?;
        let _ = self
            .rt_sender
            .send(RenderEvent::MarkerData(markers.clone()))
            .await;
        self.markers = Some(markers.clone());
        Ok(())
    }*/

    #[cfg(feature = "markers")]
    async fn load_markers_files(&mut self) -> anyhow::Result<()> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let markers_dir = addon_dir.join("markers");
        if !exists(&markers_dir).expect("Can't check if directory exists") {
            create_dir_all(&markers_dir).await;
        }
        let markers = RuntimeMarkers::load_many(&markers_dir, 100).await?;
        let markers = RuntimeMarkers::markers(markers).await;
        let _ = self
            .rt_sender
            .send(RenderEvent::MarkerData(markers.clone()))
            .await;
        self.markers = markers;
        Ok(())
    }

    async fn load_timer_files(&self) -> Vec<Arc<TimerFile>> {
        let settings_lock = self.settings.read().await;
        let mut timers = Vec::new();
        for remote in settings_lock.remotes.iter() {
            timers.extend(remote.load().await);
        }
        drop(settings_lock);
        let timers_len = timers.len();
        log::info!("Total loaded timers: {}", timers_len,);
        timers
    }

    async fn setup_timers(&mut self) {
        log::info!("Preparing to setup timers");
        self.timers = self.load_timer_files().await;
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let adhoc_timers_dir = addon_dir.join("timers");
        if exists(&adhoc_timers_dir).expect("oh no i cant access my own addon dir") {
            let adhoc_timers = TimerFile::load_many_sourceless(&adhoc_timers_dir, 100)
                .await
                .expect("wah");
            self.timers.extend(adhoc_timers);
        } else {
            create_dir_all(adhoc_timers_dir)
                .await
                .expect("Can't create timers dir");
        }
        for timer in &self.timers {
            if let Some(association) = &timer.association {
                self.sources_to_timers
                    .entry(association.clone())
                    .or_default();
                if let Some(val) = self.sources_to_timers.get_mut(association) {
                    val.push(timer.clone());
                };
            }
            // Handle map to timers
            self.map_id_to_timers.entry(timer.map_id).or_default();
            if let Some(val) = self.map_id_to_timers.get_mut(&timer.map_id) {
                val.push(timer.clone());
            };
            let association = match &timer.association {
                Some(s) => format!("{}", s),
                None => "unassociated".to_string(),
            };
            // Handle id to timer file allocation
            log::info!(
                "Set up {4} {0}: {3} for map {1}, category {2}",
                timer.id,
                timer.name.replace("\n", " "),
                timer.map_id,
                timer.category,
                association,
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
        if let Some(mumble) = self.mumble_pointer {
            let playpos = Vec3::from_array(mumble.read_avatar().position);
            #[cfg(feature = "space")]
            {
                let camera = mumble.read_camera();
                let front = Vec3::from_array(camera.front);
                let pos = Vec3::from_array(camera.position);
                PerspectiveInputData::swap_camera(front, pos, playpos);
            }
            #[cfg(feature = "markers")]
            {
                if let Some(nexus_link) = read_nexus_link() {
                    let scaling = nexus_link.scaling;
                    if self.scaling != scaling {
                        MarkerInputData::from_nexus(scaling);
                        self.scaling = scaling;
                    }
                }
                let ui_state = mumble.read_ui_state();
                let global_player_pos = Vec2::from(mumble.read_player_position());
                let global_map = Vec2::from(mumble.read_map_center());
                let compass_width = mumble.read_compass_width() as f32;
                let compass_height = mumble.read_compass_height() as f32;
                let compass_size = Vec2::new(compass_width, compass_height);
                let compass_rotation = mumble.read_compass_rotation();
                let map_scale = mumble.read_map_scale();
                let perspective = CurrentPerspective::from(ui_state.contains(UiState::IS_MAP_OPEN));
                let minimap_placement =
                    MinimapPlacement::from(ui_state.contains(UiState::IS_COMPASS_TOP_RIGHT));
                let rotation_enabled =
                    ui_state.contains(UiState::DOES_COMPASS_HAVE_ROTATION_ENABLED);
                MarkerInputData::from_tick(
                    playpos,
                    global_player_pos,
                    global_map,
                    compass_size,
                    compass_rotation,
                    map_scale,
                    perspective,
                    minimap_placement,
                    rotation_enabled,
                );
            }
            self.player_position = Some(playpos);
            let combat_state = mumble
                .read_context()
                .ui_state
                .contains(UiState::IS_IN_COMBAT);
            if combat_state != self.previous_combat_state {
                if combat_state {
                    log::info!("MumbleLink: Combat begins at {:?}!", SystemTime::now());
                    for machine in &mut self.current_timers {
                        machine.set_combat_state(CombatState::Entered);
                    }
                } else {
                    log::info!("MumbleLink: Combat ends at {:?}!", SystemTime::now());
                    for machine in &mut self.current_timers {
                        machine.set_combat_state(CombatState::Exited);
                    }
                }
                self.previous_combat_state = combat_state;
            }
            if let Some(pos) = self.player_position() {
                for machine in &mut self.current_timers {
                    machine.tick(pos).await
                }
            }
        }
        Ok(())
    }

    async fn handle_mumble(&mut self, identity: MumbleIdentityUpdate) {
        #[cfg(feature = "space")]
        {
            if self.last_fov != identity.fov {
                PerspectiveInputData::swap_fov(identity.fov);
                self.last_fov = identity.fov;
            }
        }
        let new_map_id = identity.map_id;
        if Some(new_map_id) != self.map_id {
            #[cfg(feature = "markers")]
            MarkerInputData::from_mapchange(new_map_id);
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
                    machine.set_combat_state(CombatState::Entered);
                }
            }
            StateChange::ExitCombat => {
                log::info!("ArcDPS: Combat ends at {}!", evt.time);
                for machine in &mut self.current_timers {
                    machine.set_combat_state(CombatState::Exited);
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
                            log::debug!(
                                "Creating timer machine for {} as it has been enabled.",
                                timer.id
                            );
                            self.current_timers.push(TimerMachine::new(
                                timer.clone(),
                                self.alert_sem.clone(),
                                self.rt_sender.clone(),
                            ));
                        }
                    }
                }
            }
            true => {
                let timers_to_remove = self.current_timers.iter_mut().filter(|t| t.timer.id == id);
                for timer in timers_to_remove {
                    log::debug!(
                        "Starting cleanup for timer {} as it has been disabled.",
                        timer.timer.id
                    );
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

    async fn reload_timers(&mut self) {
        self.timers.clear();
        self.sources_to_timers.clear();
        self.map_id_to_timers.clear();
        self.setup_timers().await;
        self.reset_timers().await;
        self.load_markers_files()
            .await
            .expect("markers load failed");
    }

    #[cfg(feature = "markers")]
    async fn set_marker(
        &self,
        points: Vec<ScreenPoint>,
        markers: Arc<MarkerSet>,
    ) -> anyhow::Result<()> {
        // TODO: provide configurability of wait_duration and invoke gamebind duration?
        let wait_duration = Duration::from_millis(50);
        let mut pos_ptr: POINT = POINT::default();
        let original_position = unsafe { GetCursorPos(&mut pos_ptr) }
            .map_err(anyhow::Error::from)
            .map(|()| pos_ptr)?;
        let hwnd = unsafe { GetForegroundWindow() };

        let zippy = points.iter().zip(markers.markers.clone());
        for (point, marker) in zippy {
            sleep(wait_duration).await;
            let mut my_pos: POINT = POINT {
                x: point.x as i32,
                y: point.y as i32,
            };
            unsafe { ClientToScreen(hwnd, &mut my_pos) };
            match unsafe { SetCursorPos(my_pos.x, my_pos.y) } {
                Ok(()) => (),
                Err(err) => log::error!("Error setmarker: {:?}", err),
            }
            sleep(wait_duration).await;
            invoke_gamebind_async(marker.marker.to_place_world_gamebind(), 10i32);
        }
        sleep(wait_duration).await;
        match unsafe { SetCursorPos(original_position.x, original_position.y) } {
            Ok(()) => (),
            Err(err) => log::error!("Error setmarker restore: {:?}", err),
        }
        // apparently h_wnd is ignored for this, fuck if I know? :3
        // WM_MOUSEMOVE == 0x0200
        /*unsafe {
            igSetCursorPos(ImVec2::new(point.x, point.y));
        }
        invoke_gamebind_async(marker_type.to_place_world_gamebind(), 100i32);*/
        /*let coordinates_isize = ((point.x as usize) << 16 | point.y as usize) as isize;
        log::debug!("coordinates: {:?}, {:?}", coordinates_isize, coordinates_isize.to_ne_bytes());
        let coordinates = LPARAM(coordinates_isize);
        let wnd_result = send_wnd_proc_to_game(windows::Win32::Foundation::HWND::default(), WM_MOUSEMOVE, WPARAM::default(), coordinates);*/
        // milliseconds
        //log::debug!("set_marker result: {wnd_result:?}");
        Ok(())
    }

    async fn do_update(&mut self, source: &RemoteSource) {
        match Settings::download_latest(source).await {
            Ok(_) => (),
            Err(err) => log::error!("Controller.do_update() error for \"{}\": {}", source, err),
        };
        self.reload_timers();
    }

    async fn progress_bar_style(&mut self, style: ProgressBarStyleChange) {
        let mut settings_lock = self.settings.write().await;
        let settings = settings_lock.set_progress_bar(style).await;
        let _ = self
            .rt_sender
            .send(RenderEvent::ProgressBarUpdate(settings))
            .await;

        drop(settings_lock);
    }

    async fn set_window_state(&mut self, window: String, state: Option<bool>) {
        let mut settings_lock = self.settings.write().await;
        settings_lock.set_window_state(&window, state).await;
        drop(settings_lock);
    }

    async fn open_openable<T: AsRef<OsStr>>(&self, key: String, uri: T) {
        match open::that(uri) {
            Ok(_) => (),
            Err(err) => {
                let _ = self
                    .rt_sender
                    .send(RenderEvent::OpenableError(key, err.into()))
                    .await;
            }
        }
    }
    async fn toggle_katrender(&mut self) {
        let mut settings_lock = self.settings.write().await;
        settings_lock.toggle_katrender().await;
        drop(settings_lock);
    }

    async fn uninstall_addon(&mut self, source: &RemoteSource) -> anyhow::Result<()> {
        let mut settings_lock = self.settings.write().await;
        settings_lock.uninstall_remote(source).await?;
        drop(settings_lock);
        Ok(())
    }

    async fn timer_key_trigger(&mut self, id: String, is_release: bool) {
        let idx = id.chars().last().unwrap().to_digit(10).unwrap();
        for timer in &mut self.current_timers {
            timer.key_event(idx, is_release);
        }
    }

    async fn move_mouse(&mut self, pos: Vec2) {
        let mut context = Context::current();
        let io_mut = context.io_mut();
        io_mut.mouse_pos = pos.into();
    }

    async fn load_texture(&self, rel: RelativePathBuf, base: PathBuf) {
        if let Some(base) = base.parent() {
            let abs = rel.to_path(base);
            let cally: RawTextureReceiveCallback = texture_receive!(|id, texture| {
                let gooey = IMGUI_TEXTURES.get().unwrap();
                let mut gooey_lock = gooey.write().unwrap();
                if let Some(texture) = texture {
                    gooey_lock
                        .entry(id.into())
                        .or_insert(Arc::new(texture.clone()));
                }
                drop(gooey_lock);
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
            ReloadTimers => self.reload_timers().await,
            ToggleKatRender => self.toggle_katrender().await,
            OpenOpenable(key, uri) => self.open_openable(key, uri).await,
            UninstallAddon(dd) => self.uninstall_addon(&dd).await?,
            MumbleIdentityUpdated(identity) => self.handle_mumble(identity).await,
            CombatEvent { src, evt } => self.handle_combat_event(src, evt).await,
            TimerEnable(id) => self.enable_timer(&id).await,
            TimerDisable(id) => self.disable_timer(&id).await,
            TimerToggle(id) => self.toggle_timer(&id).await,
            TimerReset => self.reset_timers().await,
            CheckDataSourceUpdates => self.check_updates().await,
            MoveMouse(pos) => self.move_mouse(pos).await,
            #[cfg(feature = "markers")]
            SetMarker(p, t) => self.set_marker(p, t).await?,
            TimerKeyTrigger(id, is_release) => self.timer_key_trigger(id, is_release).await,
            DoDataSourceUpdate { source } => self.do_update(&source).await,
            ProgressBarStyle(style) => self.progress_bar_style(style).await,
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
pub enum ProgressBarStyleChange {
    Centre(bool),
    Stock(bool),
    Shadow(bool),
    Height(f32),
    Font(TextFont),
}

#[derive(Debug, Clone, Display)]
pub enum ControllerEvent {
    OpenOpenable(String, String),
    MoveMouse(Vec2),
    #[cfg(feature = "markers")]
    SetMarker(Vec<ScreenPoint>, Arc<MarkerSet>),
    UninstallAddon(Arc<RemoteSource>),
    MumbleIdentityUpdated(MumbleIdentityUpdate),
    ToggleKatRender,
    CombatEvent {
        src: arcdps::AgentOwned,
        evt: arcEvent,
    },
    DoDataSourceUpdate {
        source: Arc<RemoteSource>,
    },
    ProgressBarStyle(ProgressBarStyleChange),
    WindowState(String, Option<bool>),
    #[strum(to_string = "Id {0}, pressed {1}")]
    TimerKeyTrigger(String, bool),
    LoadTexture(RelativePathBuf, PathBuf),
    CheckDataSourceUpdates,
    ReloadTimers,
    #[allow(dead_code)]
    TimerEnable(String),
    #[allow(dead_code)]
    TimerDisable(String),
    TimerReset,
    #[strum(to_string = "Toggled {0}")]
    TimerToggle(String),
    Quit,
}
