use nexus::{
    event::{arc::{CombatData, ACCOUNT_NAME, COMBAT_LOCAL}, event_subscribe, Event},
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
use glam::f32::Vec3;
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

static SENDER: OnceLock<Sender<TimarksThreadEvent>> = OnceLock::new();
static TM_THREAD: OnceLock<JoinHandle<()>> = OnceLock::new();


nexus::export! {
    name: "gw2timarks-rs",
    signature: -0x7331BABD, // raidcore addon id or NEGATIVE random unique signature
    load,
    unload,
    flags: AddonFlags::None,
    provider: UpdateProvider::GitHub,
    update_link: "https://github.com/kittywitch/gw2timarks-rs",
    log_filter: "debug"
}

struct TimerState {
    timer_file: bhtimer::TimerFile,
    current_phase: String,
    time_elapsed: tokio::time::Duration,
}

enum TimarksThreadEvent {
    Quit,
}

struct TimarksState {

}

impl TimarksState {
    async fn tick(&mut self) -> anyhow::Result<()> {
            Ok(())
    }
   async fn handle_event(&mut self, event: TimarksThreadEvent) -> anyhow::Result<bool> {
        use TimarksThreadEvent::*;
        match event {
            Quit => {
                return Ok(false)
            },
            _  => (),
        }
        Ok(true)
    }
}

fn load_timarks(mut tm_receiver: Receiver<TimarksThreadEvent>) {
    let mut state = TimarksState {
    };
    let evt_loop = async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(250));
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
                _ = interval.tick() => {
                    // does stuff every second
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
            Window::new("Timarks").opened(&mut self.window_open).build(ui, || {
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

fn load_timer_file(path: PathBuf) -> anyhow::Result<bhtimer::TimerFile> {
    log::info!("Attempting to load the timer file at '{path:?}'.");
    let mut file = File::open(path)?;
    let timer_data: TimerFile = serde_jsonrc::from_reader(file)?;
    return Ok(timer_data)
}

fn get_paths(path: &PathBuf) -> anyhow::Result<Paths> {
    let timer_paths: Paths = glob(path.to_str().expect("Pattern is unparseable"))?;
    Ok(timer_paths)
}

fn load_timer_files(addon_dir: &PathBuf) -> Vec<bhtimer::TimerFile> {
    let mut timers = Vec::new();
    let glob_str = addon_dir.join("*.bhtimer");
    log::info!("Path to load timer files is '{glob_str:?}'.");
    let timer_paths: Paths = get_paths(&glob_str).unwrap();
    for path in timer_paths {
        let path = path.expect("Path illegible!");
        match load_timer_file(path.clone()) {
            Ok(data) => {
                log::info!("Successfully loaded the timer file at '{path:?}'.");
                timers.push(data)
            },
            Err(error) => log::warn!("Failed to load the timer file at '{path:?}': {error}."),
        };
    }
    timers
}

fn load() {
    log::info!("Loading addon");
    let addon_dir = get_addon_dir("timarks").expect("invalid addon dir");
    let timer_files = load_timer_files(&addon_dir);
    log::debug!("Timers: {:?}", timer_files);
    let (event_sender, event_receiver) = channel::<TimarksThreadEvent>(32);
    let tm_handler = thread::spawn(|| { load_timarks(event_receiver) });
    TM_THREAD.set(tm_handler);
    SENDER.set(event_sender);
    let timark_window = render!(|ui| {
            let mut state = RENDER_STATE.lock().unwrap();
            state.render(ui)
        });

    register_render(RenderType::Render, timark_window).revert_on_unload();
    let receive_texture =
        texture_receive!(|id: &str, _texture: Option<&Texture>| log::info!("texture {id} loaded"));
    load_texture_from_file("TIMARKS_ICON", addon_dir.join("icon.png"), Some(receive_texture));
    load_texture_from_file(
        "TIMARKS_ICON_HOVER",
        addon_dir.join("icon_hover.png"),
        Some(receive_texture),
    );


    let keybind_handler = keybind_handler!(|id, is_release| {
            let mut state = RENDER_STATE.lock().unwrap();
            state.keybind_handler(id, is_release)
    });
    register_keybind_with_string("TIMARKS_MENU_KEYBIND", keybind_handler, "ALT+SHIFT+M").revert_on_unload();
    add_quick_access(
        "Timarks Control",
        "TIMARKS_ICON",
        "TIMARKS_ICON_HOVER",
        "TIMARKS_MENU_KEYBIND",
        "Open Timarks control menu",
    )
    .revert_on_unload();

}

fn unload() {
    log::info!("Unloading addon");
    // all actions passed to on_load() or revert_on_unload() are performed automatically
    let sender = SENDER.get().unwrap();
    sender.try_send(TimarksThreadEvent::Quit);
}
