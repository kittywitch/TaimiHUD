use nexus::{
    event::{arc::{CombatData, ACCOUNT_NAME, COMBAT_LOCAL},
        MUMBLE_IDENTITY_UPDATED, MumbleIdentityUpdate,
        event_subscribe, Event,
    },
    data_link::{read_mumble_link, MUMBLE_LINK, MumbleLink},
    event_consume,
    gui::{register_render, unregister_render, render, RenderType},
    imgui::{sys::cty::c_char, Ui, Window},
    keybind::{keybind_handler, register_keybind_with_string},
    paths::get_addon_dir,
    quick_access::add_quick_access,
    texture::{load_texture_from_file, texture_receive, Texture},
    AddonFlags, UpdateProvider,
};
use tokio::{runtime, select, task::JoinSet};
use tokio::sync::mpsc::{Receiver, Sender, channel, error::TryRecvError};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use std::{cell::{Cell, Ref, RefCell}, collections::VecDeque, ffi::CStr, ptr, thread::{self, JoinHandle}};
use std::sync::OnceLock;
use std::sync::Once;
use arcdps::{evtc::event::{EnterCombatEvent, Event as arcEvent}, Agent, AgentOwned};
use arcdps::Affinity;
use std::sync::{Arc, Mutex};
use glam::{swizzles::*, f32::Vec3};
use std::fs::File;
use palette::rgb::Rgb;
use palette::convert::{FromColorUnclamped, IntoColorUnclamped};
use palette::{Srgba};
use serde::{Deserialize, Serialize};
use glob::{glob, Paths};
use std::path::{Path, PathBuf};
mod xnacolour;
mod bhtimer;
use xnacolour::XNAColour;
use bhtimer::*;
use std::collections::HashMap;

static SENDER: OnceLock<Sender<TaimiThreadEvent>> = OnceLock::new();
static TM_THREAD: OnceLock<JoinHandle<()>> = OnceLock::new();


nexus::export! {
    name: "TaimiHUD",
    signature: -0x7331BABD, // raidcore addon id or NEGATIVE random unique signature
    load,
    unload,
    flags: AddonFlags::None,
    provider: UpdateProvider::GitHub,
    update_link: "https://github.com/kittywitch/gw2Taimi-rs",
    log_filter: "debug"
}

#[derive(Debug, Clone)]
enum TimerMachineState {
    // This possibly shouldn't happen?
    OffMap,
    // These can be met
    OnMap,
    OnMapWithinBoundaryUntriggered,
    Started,
    Finished,
}

#[derive(Debug, Clone)]
struct TimerMachine {
    // TODO: this should be an Arc<TimerFile>
    timer_file: bhtimer::TimerFile,
    current_phase: String,
    machine_state: TimerMachineState,
    time_elapsed: tokio::time::Duration,
    in_combat: bool,
}

impl TimerMachine {
    async fn process_state(&mut self, map_id: u32, position: Vec3, combat: bool) {
    }
}

#[derive(Debug, Clone)]
enum TaimiThreadEvent {
    MumbleIdentityUpdated(MumbleIdentityUpdate),
    Quit,
}

#[derive(Debug, Clone)]
struct TaimiState {
    addon_dir: PathBuf,
    cached_identity: Option<MumbleIdentityUpdate>,
    cached_link: Option<MumbleLink>,
    timers: HashMap<String, TimerFile>,
    // TODO: Refactor to be a hashmap of ID to pointer to timerfile
    // instead of any use of timer_id, use the Arc as a shared reference
    //
    // * no longer have to worry about .clone()
    // * don't have to worry about lifetimes thanks to arc
    // THANKS ARC <3
    //map_id_to_timers: HashMap<u32, Vec<Arc<TimerFile>>,
    //category_to_timers: HashMap<String, Vec<Arc<TimerFile>>,
    map_id_to_timer_ids: HashMap<u32, Vec<String>>,
    category_to_timer_ids: HashMap<String, Vec<String>>,
    map_id: Option<u32>,
    player_position: Option<Vec3>,
    timers_for_map: Vec<String>,
    // TODO: This should be...
    // current_timers: Vec<TimerMachine>
    starts_to_check: HashMap<String, TimerPhase>,
}

impl TaimiState {
    async fn load_timer_file(&self, path: PathBuf) -> anyhow::Result<bhtimer::TimerFile> {
        log::info!("Attempting to load the timer file at '{path:?}'.");
        let mut file = File::open(path)?;
        let timer_data: TimerFile = serde_jsonrc::from_reader(file)?;
        return Ok(timer_data)
    }

    async fn get_paths(&self, path: &PathBuf) -> anyhow::Result<Paths> {
        let timer_paths: Paths = glob(path.to_str().expect("Pattern is unparseable"))?;
        Ok(timer_paths)
    }

    async fn load_timer_files(&self) -> Vec<bhtimer::TimerFile> {
        let mut timers = Vec::new();
        let glob_str = self.addon_dir.join("*.bhtimer");
        log::info!("Path to load timer files is '{glob_str:?}'.");
        let timer_paths: Paths = self.get_paths(&glob_str).await.unwrap();
        for path in timer_paths {
            let path = path.expect("Path illegible!");
            match self.load_timer_file(path.clone()).await {
                Ok(data) => {
                    log::info!("Successfully loaded the timer file at '{path:?}'.");
                    timers.push(data);
                },
                Err(error) => log::warn!("Failed to load the timer file at '{path:?}': {error}."),
            };
        }
        timers
    }

    async fn setup_timers(&mut self) {
        log::info!("Preparing to setup timers");
        let timers = self.load_timer_files().await;

        for timer in timers {
            let timer_held = timer.clone();
            // Handle map_id to timer_id
            if !self.map_id_to_timer_ids.contains_key(&timer.map_id) {
                self.map_id_to_timer_ids.insert(timer.map_id.clone(), Vec::new());
            }
            if let Some(val) = self.map_id_to_timer_ids.get_mut(&timer.map_id) { val.push(timer.id.clone()); };
            // Handle category to timer_id list
            if !self.category_to_timer_ids.contains_key(&timer.category) {
                self.category_to_timer_ids.insert(timer.category.clone(), Vec::new());
            }
            if let Some(val) = self.category_to_timer_ids.get_mut(&timer.category) { val.push(timer.id.clone()); };
            // Handle id to timer file allocation
            log::info!("Set up {0} for map {1}, category {2}", timer.id, timer.map_id, timer.category);
            self.timers.insert(timer.id, timer_held);
        }
    }

    // TODO: refactor code such that the start triggers are handled as part of the
    // TimerMachine, where we check if it is OnMap and untriggered...
    // The code for checking sphere/cuboid regions should be built into the actual TimerMachine
    // This avoids mutating a collection and allows us to reckon with these things as checking the
    // Enum value
    async fn tick(&mut self) -> anyhow::Result<()> {
            let mut started_ids = Vec::new();
            for (timer_id, start_phase) in &self.starts_to_check {
                use bhtimer::TimerTriggerType::*;
                let start_trigger = &start_phase.start;
                match &start_trigger.kind {
                    Location => {
                        let p1 = start_trigger.position().unwrap();
                        if let Some(player) = self.player_position {
                            // I don't know why this is necessary
                            // Check a sphere
                            if let Some(radius) = start_trigger.radius {
                                    if p1.distance(player) < radius {
                                        log::info!("Player is within the spherical boundary for '{}'.", start_phase.name);
                                        started_ids.push(timer_id.clone());
                                    }
                            }
                            // Check a cuboid
                            if let Some(p2) = start_trigger.antipode() {
                                let mins = p1.min(p2);
                                let maxs = p1.max(p2);
                                let min_cmp = player.cmpge(mins);
                                let max_cmp = player.cmple(maxs);
                                let player_in_area = min_cmp.all() && max_cmp.all();
                                if player_in_area {
                                    log::info!("Player is within the cuboid boundary for '{}'.", start_phase.name);
                                    started_ids.push(timer_id.clone());
                                }
                            }
                        }
                    },
                    Key => (),
                }
            }
            for started_id in started_ids {
               self.starts_to_check.remove(&started_id);
            }
            Ok(())
    }

    async fn mumblelink_tick(&mut self) -> anyhow::Result<()> {
            self.cached_link = read_mumble_link();
            if let Some(link) = &self.cached_link {
                self.player_position = Some(Vec3::from_array(link.avatar.position));
            };
            Ok(())
    }

   async fn handle_event(&mut self, event: TaimiThreadEvent) -> anyhow::Result<bool> {
        use TaimiThreadEvent::*;
        match event {
            MumbleIdentityUpdated(identity) => {
                if self.map_id != Some(identity.map_id) {
                    match self.map_id {
                        Some(map_id) => log::info!("User has changed map from {0} to {1}", map_id, identity.map_id),
                        None => log::info!("User's map is {0}", identity.map_id),
                    }
                    self.map_id = Some(identity.map_id);
                    let map_id_local = &self.map_id.unwrap();
                    if self.map_id_to_timer_ids.contains_key(map_id_local) {
                        let timers_for_map = &self.map_id_to_timer_ids[map_id_local];
                        let timers_list = timers_for_map.join(", ");
                        let mut starts_to_check = HashMap::new();
                        for timer_id in timers_for_map {
                            let timer = &self.timers[timer_id];
                            let start_phase = &timer.phases[0];
                            starts_to_check.insert(timer_id.clone(), start_phase.clone());
                        };
                        self.starts_to_check = starts_to_check;
                        self.timers_for_map = timers_for_map.to_vec();
                        log::info!("Timers found for map {0}: {1}", map_id_local, timers_list);
                    } else {
                        self.starts_to_check = HashMap::new();
                        self.timers_for_map = Vec::new();
                        log::info!("No timers found for map {0}.", map_id_local);
                    }
                }
                self.cached_identity = Some(identity);
            },
            Quit => {
                return Ok(false)
            },
            _  => (),
        }
        Ok(true)
    }
}

fn load_taimi(mut tm_receiver: Receiver<TaimiThreadEvent>, addon_dir: PathBuf) {
    let mut state = TaimiState {
        addon_dir: addon_dir,
        cached_identity: None,
        cached_link: None,
        timers: HashMap::new(),
        map_id_to_timer_ids: HashMap::new(),
        category_to_timer_ids: HashMap::new(),
        map_id: None,
        player_position: None,
        timers_for_map: Default::default(),
        starts_to_check: Default::default(),
    };

    let evt_loop = async move {
        state.setup_timers().await;
        let mut taimi_interval = tokio::time::interval(tokio::time::Duration::from_millis(250));
        let mut mumblelink_interval = tokio::time::interval(tokio::time::Duration::from_millis(20));
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
                    state.mumblelink_tick().await;
                },
                _ = taimi_interval.tick() => {
                    state.tick().await;
                },
            }
        }
    };
    let rt = match runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(error) => {
            log::error!("Error! {}", error);
            return
        },
    };
    rt.block_on(evt_loop);
}

struct RenderState {
    window_open: bool,
    show: bool,
}

impl RenderState {
    const fn new() -> Self {
        Self{window_open: true, show: false}
    }
    fn keybind_handler(&mut self, id: &str, is_release: bool) {
        if !is_release {
            self.window_open = !self.window_open;
        }
    }
    fn render(&mut self, ui: &Ui) {
        let show = &mut self.show;
        if self.window_open {
            Window::new("Taimi").opened(&mut self.window_open).build(ui, || {
                if *show {
                    *show = !ui.button("hide");
                    ui.text("Hello world");
                } else {
                    *show = ui.button("show");
                }
            });
        }
    }
    fn build_window(&mut self, ui: &Ui) {
    }
}

static RENDER_STATE: Mutex<RenderState> = const { Mutex::new(RenderState::new()) };

fn load() {
    // Say hi to the world :o
    let name = env!("CARGO_PKG_NAME");
    let authors = env!("CARGO_PKG_AUTHORS");
    log::info!("Loading {name} by {authors}");


    // Set up the thread
    let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
    let passed_addon_dir = addon_dir.clone();
    let (event_sender, event_receiver) = channel::<TaimiThreadEvent>(32);
    let tm_handler = thread::spawn(|| { load_taimi(event_receiver, addon_dir) });
    TM_THREAD.set(tm_handler);
    SENDER.set(event_sender);

    // Rendering setup
    let taimi_window = render!(|ui| {
            let mut state = RENDER_STATE.lock().unwrap();
            state.render(ui)
    });

    register_render(RenderType::Render, taimi_window).revert_on_unload();



    // Handle window toggling with keybind and button
    let keybind_handler = keybind_handler!(|id, is_release| {
            let mut state = RENDER_STATE.lock().unwrap();
            state.keybind_handler(id, is_release)
    });
    register_keybind_with_string("TAIMI_MENU_KEYBIND", keybind_handler, "ALT+SHIFT+M").revert_on_unload();

    // Disused currently, icon loading for quick access
    /*
    let receive_texture =
        texture_receive!(|id: &str, _texture: Option<&Texture>| log::info!("texture {id} loaded"));
    load_texture_from_file("Taimi_ICON", addon_dir.join("icon.png"), Some(receive_texture));
    load_texture_from_file(
        "Taimi_ICON_HOVER",
        addon_dir.join("icon_hover.png"),
        Some(receive_texture),
    );
    */

    add_quick_access(
        "TAIMI Control",
        "TAIMI_ICON",
        "TAIMI_ICON_HOVER",
        "TAIMI_MENU_KEYBIND",
        "Open Taimi control menu",
    )
    .revert_on_unload();

    // MumbleLink Identity
    MUMBLE_IDENTITY_UPDATED.subscribe(event_consume!(<MumbleIdentityUpdate> |mumble_identity| {
        let sender = SENDER.get().unwrap();
        match mumble_identity {
            None => (),
            Some(ident) => {
                let copied_identity = ident.clone();
                sender.try_send(TaimiThreadEvent::MumbleIdentityUpdated(copied_identity));
            },
        }
    })).revert_on_unload();
}

fn unload() {
    log::info!("Unloading addon");
    // all actions passed to on_load() or revert_on_unload() are performed automatically
    let sender = SENDER.get().unwrap();
    sender.try_send(TaimiThreadEvent::Quit);
}
