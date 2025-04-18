mod settings;
mod controller;
mod timer;
mod render;

use {
    crate::{
        render::{RenderState, RenderEvent},
        settings::SettingsLock,
        controller::{Controller, ControllerEvent},
    },
    arcdps::AgentOwned,
    nexus::{
        event::{
            arc::{CombatData, COMBAT_LOCAL},
            event_consume, MumbleIdentityUpdate, MUMBLE_IDENTITY_UPDATED,
        },
        gui::{register_render, render, RenderType},
        keybind::{keybind_handler, register_keybind_with_string},
        paths::get_addon_dir,
        quick_access::add_quick_access,
        // TODO
        //texture::{load_texture_from_file, texture_receive, Texture},
        AddonFlags,
        UpdateProvider,
    },
    std::{
        ptr,
        sync::{Mutex, OnceLock},
        thread::{self, JoinHandle},
    },
    tokio::sync::mpsc::{channel, Sender},
};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

static TS_SENDER: OnceLock<Sender<controller::ControllerEvent>> = OnceLock::new();
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

static RENDER_STATE: OnceLock<Mutex<RenderState>> = OnceLock::new();
static SETTINGS: OnceLock<SettingsLock> = OnceLock::new();

fn load() {
    // Say hi to the world :o
    let name = env!("CARGO_PKG_NAME");
    let authors = env!("CARGO_PKG_AUTHORS");
    log::info!("Loading {name} by {authors}");

    // Set up the thread
    let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");

    let (ts_event_sender, ts_event_receiver) = channel::<ControllerEvent>(32);
    let (rt_event_sender, rt_event_receiver) = channel::<RenderEvent>(32);
    let tm_handler =
        thread::spawn(|| Controller::load(ts_event_receiver, rt_event_sender, addon_dir));
    // muh queues
    let _ = TM_THREAD.set(tm_handler);
    let _ = TS_SENDER.set(ts_event_sender);
    let _ = RENDER_STATE.set(Mutex::new(RenderState::new(rt_event_receiver)));

    // Rendering setup
    let taimi_window = render!(|ui| {
        let mut state = RenderState::lock();
        state.draw(ui);
    });

    register_render(RenderType::Render, taimi_window).revert_on_unload();

    // Handle window toggling with keybind and button
    let main_window_keybind_handler = keybind_handler!(|id, is_release| {
        let mut state = RenderState::lock();
        state.primary_window.keybind_handler(id, is_release)
    });
    register_keybind_with_string(
        "Taimi Window Toggle",
        main_window_keybind_handler,
        "ALT+SHIFT+M",
    )
    .revert_on_unload();

    let event_trigger_keybind_handler = keybind_handler!(|id, is_release| {
        let sender = TS_SENDER.get().unwrap();
        let _ = sender.try_send(ControllerEvent::TimerKeyTrigger(
            id.to_string(),
            is_release,
        ));
    });
    for i in 0..5 {
        register_keybind_with_string(
            format!("Timer Key Trigger {}", i),
            event_trigger_keybind_handler,
            "",
        )
        .revert_on_unload();
    }

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
        "Taimi Window Toggle",
        "Open Taimi control menu",
    )
    .revert_on_unload();

    let combat_callback = event_consume!(|cdata: Option<&CombatData>| {
        let sender = TS_SENDER.get().unwrap();
        if let Some(combat_data) = cdata {
            if let Some(evt) = combat_data.event() {
                if let Some(agt) = combat_data.src() {
                    let agt = AgentOwned::from(unsafe { ptr::read(agt) });
                    let event_send = sender.try_send(ControllerEvent::CombatEvent {
                        src: agt,
                        evt: evt.clone(),
                    });
                    drop(event_send);
                }
            }
        }
    });
    COMBAT_LOCAL.subscribe(combat_callback).revert_on_unload();

    // MumbleLink Identity
    MUMBLE_IDENTITY_UPDATED
        .subscribe(event_consume!(<MumbleIdentityUpdate> |mumble_identity| {
            let sender = TS_SENDER.get().unwrap();
            match mumble_identity {
                None => (),
                Some(ident) => {
                    let copied_identity = ident.clone();
                    let event_send = sender.try_send(ControllerEvent::MumbleIdentityUpdated(copied_identity));
                    drop(event_send);
                },
            }
        }))
        .revert_on_unload();
}

fn unload() {
    log::info!("Unloading addon");
    // all actions passed to on_load() or revert_on_unload() are performed automatically
    let sender = TS_SENDER.get().unwrap();
    let event_send = sender.try_send(ControllerEvent::Quit);
    drop(event_send);
}
