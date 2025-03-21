mod bhtimer;
mod xnacolour;
mod timermachine;
mod taimistate;

use {
    nexus::{
        event::{
            event_consume, MumbleIdentityUpdate, MUMBLE_IDENTITY_UPDATED,
        },
        gui::{register_render, render, RenderType},
        imgui::{Ui, Window},
        keybind::{keybind_handler, register_keybind_with_string},
        paths::get_addon_dir,
        quick_access::add_quick_access,
        //texture::{load_texture_from_file, texture_receive, Texture},
        AddonFlags, UpdateProvider,
    },
    std::{
        sync::{Mutex, OnceLock},
        thread::{self, JoinHandle},
    },
    tokio::sync::mpsc::{channel, Sender},
    taimistate::{TaimiState, TaimiThreadEvent},
};

static SENDER: OnceLock<Sender<taimistate::TaimiThreadEvent>> = OnceLock::new();
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


struct RenderState {
    window_open: bool,
    show: bool,
}

impl RenderState {
    const fn new() -> Self {
        Self {
            window_open: true,
            show: false,
        }
    }
    fn keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            self.window_open = !self.window_open;
        }
    }
    fn render(&mut self, ui: &Ui) {
        let show = &mut self.show;
        if self.window_open {
            Window::new("Taimi")
                .opened(&mut self.window_open)
                .build(ui, || {
                    if *show {
                        *show = !ui.button("hide");
                        ui.text("Hello world");
                    } else {
                        *show = ui.button("show");
                    }
                });
        }
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
    let (event_sender, event_receiver) = channel::<TaimiThreadEvent>(32);
    let tm_handler = thread::spawn(|| TaimiState::load(event_receiver, addon_dir));
    // muh queues
    let _ = TM_THREAD.set(tm_handler);
    let _ = SENDER.set(event_sender);

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
    register_keybind_with_string("TAIMI_MENU_KEYBIND", keybind_handler, "ALT+SHIFT+M")
        .revert_on_unload();

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
    MUMBLE_IDENTITY_UPDATED
        .subscribe(event_consume!(<MumbleIdentityUpdate> |mumble_identity| {
            let sender = SENDER.get().unwrap();
            match mumble_identity {
                None => (),
                Some(ident) => {
                    let copied_identity = ident.clone();
                    sender.try_send(TaimiThreadEvent::MumbleIdentityUpdated(copied_identity));
                },
            }
        }))
        .revert_on_unload();
}

fn unload() {
    log::info!("Unloading addon");
    // all actions passed to on_load() or revert_on_unload() are performed automatically
    let sender = SENDER.get().unwrap();
    let _ = sender.try_send(TaimiThreadEvent::Quit);
}
