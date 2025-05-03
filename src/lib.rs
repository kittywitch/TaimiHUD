mod controller;
mod render;
mod settings;
mod timer;
mod marker;

#[cfg(feature = "space")]
mod space;

#[cfg(feature = "space")]
use space::{engine::SpaceEvent, resources::Texture, Engine};
use {
    crate::{
        controller::{Controller, ControllerEvent},
        render::{RenderEvent, RenderState},
        settings::SettingsLock,
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
        texture::Texture as NexusTexture,
        AddonFlags, UpdateProvider,
    },
    std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        path::PathBuf,
        ptr,
        sync::{Arc, Mutex, OnceLock, RwLock},
        thread::{self, JoinHandle},
    },
    tokio::sync::mpsc::{channel, Sender},
};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(feature = "space")]
static TEXTURES: OnceLock<RwLock<HashMap<PathBuf, Arc<Texture>>>> = OnceLock::new();
static IMGUI_TEXTURES: OnceLock<RwLock<HashMap<String, Arc<NexusTexture>>>> = OnceLock::new();
static CONTROLLER_SENDER: OnceLock<Sender<ControllerEvent>> = OnceLock::new();
static RENDER_SENDER: OnceLock<Sender<RenderEvent>> = OnceLock::new();

#[cfg(feature = "space")]
static SPACE_SENDER: OnceLock<Sender<SpaceEvent>> = OnceLock::new();

static CONTROLLER_THREAD: OnceLock<JoinHandle<()>> = OnceLock::new();

nexus::export! {
    name: "TaimiHUD",
    signature: -0x7331BABD, // raidcore addon id or NEGATIVE random unique signature
    load,
    unload,
    flags: AddonFlags::None,
    provider: UpdateProvider::GitHub,
    update_link: "https://github.com/kittywitch/TaimiHUD",
    log_filter: "debug"
}

static RENDER_STATE: OnceLock<Mutex<RenderState>> = OnceLock::new();
static SETTINGS: OnceLock<SettingsLock> = OnceLock::new();
#[cfg(feature = "space")]
thread_local! {
    static ENGINE_INITIALIZED: Cell<bool> = const { Cell::new(false) };
    static ENGINE: RefCell<Option<Engine>> = panic!("!");
}

fn load() {
    IMGUI_TEXTURES.set(RwLock::new(HashMap::new()));
    #[cfg(feature = "space")]
    TEXTURES.set(RwLock::new(HashMap::new()));
    // Say hi to the world :o
    let name = env!("CARGO_PKG_NAME");
    let authors = env!("CARGO_PKG_AUTHORS");
    log::info!("Loading {name} by {authors}");

    // Set up the thread
    let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");

    let (controller_sender, controller_receiver) = channel::<ControllerEvent>(32);
    let (render_sender, render_receiver) = channel::<RenderEvent>(32);

    let _ = CONTROLLER_SENDER.set(controller_sender);
    let _ = RENDER_SENDER.set(render_sender.clone());

    let controller_handler =
        thread::spawn(|| Controller::load(controller_receiver, render_sender, addon_dir));

    // muh queues
    let _ = CONTROLLER_THREAD.set(controller_handler);
    let _ = RENDER_STATE.set(Mutex::new(RenderState::new(render_receiver)));

    // Rendering setup
    let taimi_window = render!(|ui| {
        let mut state = RenderState::lock();
        state.draw(ui);
        drop(state);
    });
    register_render(RenderType::Render, taimi_window).revert_on_unload();

    #[cfg(feature = "space")]
    let space_render = render!(|ui| {
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            if settings.enable_katrender {
                if !ENGINE_INITIALIZED.get() {
                    let (space_sender, space_receiver) = channel::<SpaceEvent>(32);
                    let _ = SPACE_SENDER.set(space_sender);
                    let drawstate_inner = Engine::initialise(ui, space_receiver);
                    if let Err(error) = &drawstate_inner {
                        log::error!("DrawState setup failed: {}", error);
                    };
                    ENGINE.set(drawstate_inner.ok());
                    ENGINE_INITIALIZED.set(true);
                }
                ENGINE.with_borrow_mut(|ds_op| {
                    if let Some(ds) = ds_op {
                        if let Err(error) = ds.render(ui) {
                            log::error!("Engine error: {error}");
                        }
                    }
                });
            }
        }
    });
    #[cfg(feature = "space")]
    register_render(RenderType::Render, space_render).revert_on_unload();

    // Handle window toggling with keybind and button
    let main_window_keybind_handler = keybind_handler!(|_id, is_release| {
        if !is_release {
            let sender = RENDER_SENDER.get().unwrap();
            let _ = sender.try_send(RenderEvent::RenderKeybindUpdate);
        }
    });

    register_keybind_with_string(
        "Taimi Window Toggle",
        main_window_keybind_handler,
        "ALT+SHIFT+M",
    )
    .revert_on_unload();

    let event_trigger_keybind_handler = keybind_handler!(|id, is_release| {
        let sender = CONTROLLER_SENDER.get().unwrap();
        let _ = sender.try_send(ControllerEvent::TimerKeyTrigger(id.to_string(), is_release));
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
        let sender = CONTROLLER_SENDER.get().unwrap();
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
            let sender = CONTROLLER_SENDER.get().unwrap();
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
    #[cfg(feature = "space")]
    ENGINE.set(None);
    TEXTURES.set(Default::default());
    /*ENGINE.with_borrow_mut(|e| {
        //#[cfg(todo)]
        //e.cleanup();
    });*/
    let sender = CONTROLLER_SENDER.get().unwrap();
    let event_send = sender.try_send(ControllerEvent::Quit);
    drop(event_send);
}
