mod bhtimer;
mod geometry;
mod taimistate;
mod timermachine;
mod xnacolour;

use {
    nexus::{
        event::{event_consume, MumbleIdentityUpdate, MUMBLE_IDENTITY_UPDATED},
        gui::{register_render, render, RenderType},
        imgui::{Ui, Window, WindowFlags},
        keybind::{keybind_handler, register_keybind_with_string},
        paths::get_addon_dir,
        quick_access::add_quick_access,
        //texture::{load_texture_from_file, texture_receive, Texture},
        AddonFlags,
        UpdateProvider,
    },
    std::{
        collections::VecDeque,
        sync::{Mutex, MutexGuard, OnceLock},
        thread::{self, JoinHandle},
    },
    taimistate::{TaimiState, TaimiThreadEvent},
    tokio::sync::mpsc::{channel, Receiver, Sender},
};

static TS_SENDER: OnceLock<Sender<taimistate::TaimiThreadEvent>> = OnceLock::new();
static RT_RECEIVER: OnceLock<Receiver<RenderThreadEvent>> = OnceLock::new();
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

enum RenderThreadEvent {
    AlertStart(String),
    AlertEnd,
}

struct RenderState {
    receiver: Receiver<RenderThreadEvent>,
    primary_window_open: bool,
    primary_show: bool,
    alert: Option<String>,
}

impl RenderState {
    fn new(receiver: Receiver<RenderThreadEvent>) -> Self {
        Self {
            receiver,
            primary_window_open: true,
            primary_show: false,
            alert: None,
        }
    }
    fn keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            self.primary_window_open = !self.primary_window_open;
        }
    }
    fn render(&mut self, ui: &Ui) {
        let primary_show = &mut self.primary_show;
        let io = ui.io();
        match self.receiver.try_recv() {
            Ok(event) => {
                use RenderThreadEvent::*;
                match event {
                    AlertStart(message) => {
                        self.alert = Some(message);
                    }
                    AlertEnd => {
                        self.alert = None;
                    }
                }
            }
            Err(_error) => (),
        }
        if let Some(message) = &self.alert {
            Self::render_alert(ui, io, message)
        }
        if self.primary_window_open {
            Window::new("Taimi")
                .opened(&mut self.primary_window_open)
                .build(ui, || {
                    if *primary_show {
                        *primary_show = !ui.button("hide");
                        ui.text("Hello world");
                    } else {
                        *primary_show = ui.button("show");
                    }
                });
        }
    }
    fn render_alert(ui: &Ui, io: &nexus::imgui::Io, text: &String) {
        use WindowFlags;
        let [text_width, text_height] = ui.calc_text_size(text);
        let offset_x = text_width / 2.0;
        let prior_cursor_position = ui.cursor_screen_pos();
        let [game_width, game_height] = io.display_size;
        let centre_x = game_width / 2.0;
        let centre_y = game_height / 2.0;
        // this will either be 80% or 20%, i don't know how their coordinates work
        let above_y = game_height * 0.2;
        let text_x = centre_x - offset_x;
        let text_y = centre_y - above_y;
        Window::new("TAIMIHUD_ALERT_AREA")
            .flags(
                WindowFlags::ALWAYS_AUTO_RESIZE
                    | WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_SCROLLBAR
                    | WindowFlags::NO_INPUTS
                    | WindowFlags::NO_FOCUS_ON_APPEARING
                    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
            )
            .build(ui, || {
                ui.set_cursor_screen_pos([text_x, text_y]);
                ui.set_cursor_screen_pos(prior_cursor_position);
                ui.text(text);
            });
    }

    fn lock() -> MutexGuard<'static, RenderState> {
        RENDER_STATE.get().unwrap().lock().unwrap()
    }
}

static RENDER_STATE: OnceLock<Mutex<RenderState>> = OnceLock::new();

fn load() {
    // Say hi to the world :o
    let name = env!("CARGO_PKG_NAME");
    let authors = env!("CARGO_PKG_AUTHORS");
    log::info!("Loading {name} by {authors}");

    // Set up the thread
    let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
    let (ts_event_sender, ts_event_receiver) = channel::<TaimiThreadEvent>(32);
    let (rt_event_sender, rt_event_receiver) = channel::<RenderThreadEvent>(32);
    let tm_handler =
        thread::spawn(|| TaimiState::load(ts_event_receiver, rt_event_sender, addon_dir));
    // muh queues
    let _ = TM_THREAD.set(tm_handler);
    let _ = TS_SENDER.set(ts_event_sender);
    RENDER_STATE.set(Mutex::new(RenderState::new(rt_event_receiver)));

    // Rendering setup
    let taimi_window = render!(|ui| {
        let mut state = RenderState::lock();
        state.render(ui);
    });

    register_render(RenderType::Render, taimi_window).revert_on_unload();

    // Handle window toggling with keybind and button
    let keybind_handler = keybind_handler!(|id, is_release| {
        let mut state = RenderState::lock();
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
            let sender = TS_SENDER.get().unwrap();
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
    let sender = TS_SENDER.get().unwrap();
    let _ = sender.try_send(TaimiThreadEvent::Quit);
}
