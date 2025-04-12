mod geometry;
mod taimistate;
mod timer;
mod timermachine;
mod xnacolour;

use {
    arcdps::AgentOwned,
    tokio::time::Instant,
    nexus::{
        data_link::read_nexus_link,
        event::{
            arc::{CombatData, COMBAT_LOCAL},
            event_consume, MumbleIdentityUpdate, MUMBLE_IDENTITY_UPDATED,
        },
        gui::{register_render, render, RenderType},
        imgui::{internal::RawCast, Condition, Font, FontId, Ui, Io, Window, WindowFlags},
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
        sync::{Mutex, MutexGuard, OnceLock, Arc},
        thread::{self, JoinHandle},
    },
    crate::{
        timer::timeralert::TimerAlert,
        taimistate::{TaimiState, TaimiThreadEvent},
    },
    tokio::sync::mpsc::{channel, Receiver, Sender},
};

static TS_SENDER: OnceLock<Sender<taimistate::TaimiThreadEvent>> = OnceLock::new();
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
    AlertFeed(Vec<TimerAlert>),
    AlertReset,
    AlertStart(String),
    AlertEnd,
}

struct RenderState {
    receiver: Receiver<RenderThreadEvent>,
    primary_window_open: bool,
    primary_show: bool,
    timers_window_open: bool,
    alert: Option<String>,
    phase_state: Option<PhaseState>,
}

#[derive(Clone)]
struct PhaseState {
    start: Instant,
    alerts: Vec<TimerAlert>,
}

impl RenderState {
    fn new(receiver: Receiver<RenderThreadEvent>) -> Self {
        Self {
            receiver,
            primary_window_open: true,
            primary_show: false,
            timers_window_open: true,
            alert: Default::default(),
            phase_state: Default::default(),

        }
    }
    fn main_window_keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            self.primary_window_open = !self.primary_window_open;
        }
    }
    fn render(&mut self, ui: &Ui) {
        let io = ui.io();
        match self.receiver.try_recv() {
            Ok(event) => {
                use RenderThreadEvent::*;
                match event {
                    AlertStart(message) => {
                        self.alert = Some(message);
                    },
                    AlertEnd => {
                        self.alert = None;
                    },
                    AlertFeed(alerts) => {
                        log::info!("I received an alert feed event!");
                        self.phase_state = Some(PhaseState {
                            start: Instant::now(),
                            alerts,
                        });
                    },
                    AlertReset => {
                        log::info!("I received an alert reset event!");
                        self.phase_state = None;
                    },
                }
            }
            Err(_error) => (),
        }
        self.handle_alert(ui, io);
        self.handle_taimi_main_window(ui);
        self.handle_timers_window(ui);
    }
    fn handle_taimi_main_window(&mut self, ui: &Ui) {
        let primary_show = &mut self.primary_show;
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
    fn handle_timers_window(&mut self, ui: &Ui) {
        Window::new("Timers")
        .opened(&mut self.timers_window_open)
            .build(ui, || {
                    if let Some(ps) = &self.phase_state {
                        for alert in ps.alerts.iter() {
                                alert.progress_bar(ui, ps.start)
                        }
                    }
            });
    }
    fn handle_alert(&mut self, ui: &Ui, io: &Io) {
        if let Some(message) = &self.alert {
            let nexus_link = read_nexus_link().unwrap();
            let imfont_pointer = nexus_link.font_big;
            let imfont = unsafe { Font::from_raw(&*imfont_pointer) };
            Self::render_alert(ui, io, message, imfont.id(), imfont.scale);
        }
    }
    fn render_alert(ui: &Ui, io: &nexus::imgui::Io, text: &String, font: FontId, font_scale: f32) {
        use WindowFlags;
        let font_handle = ui.push_font(font);
        let fb_scale = io.display_framebuffer_scale;
        let [text_width, text_height] = ui.calc_text_size(text);
        let text_width = text_width * font_scale;
        let offset_x = text_width / 2.0;
        let [game_width, game_height] = io.display_size;
        let centre_x = game_width / 2.0;
        let centre_y = game_height / 2.0;
        let above_y = game_height * 0.2;
        let text_x = (centre_x - offset_x) * fb_scale[0];
        let text_y = (centre_y - above_y) * fb_scale[1];
        Window::new("TAIMIHUD_ALERT_AREA")
            .flags(
                WindowFlags::ALWAYS_AUTO_RESIZE
                    | WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_BACKGROUND
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_SCROLLBAR
                    | WindowFlags::NO_INPUTS
                    | WindowFlags::NO_FOCUS_ON_APPEARING
                    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
            )
            .position([text_x, text_y], Condition::Always)
            .size([text_width * 1.25, text_height * 2.0], Condition::Always)
            .build(ui, || {
                ui.text(text);
            });
        font_handle.pop();
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
    let _ = RENDER_STATE.set(Mutex::new(RenderState::new(rt_event_receiver)));

    // Rendering setup
    let taimi_window = render!(|ui| {
        let mut state = RenderState::lock();
        state.render(ui);
    });

    register_render(RenderType::Render, taimi_window).revert_on_unload();

    // Handle window toggling with keybind and button
    let main_window_keybind_handler = keybind_handler!(|id, is_release| {
        let mut state = RenderState::lock();
        state.main_window_keybind_handler(id, is_release)
    });
    register_keybind_with_string("TAIMI_MENU_KEYBIND", main_window_keybind_handler, "ALT+SHIFT+M")
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

    let combat_callback = event_consume!(|cdata: Option<&CombatData>| {
        let sender = TS_SENDER.get().unwrap();
        if let Some(combat_data) = cdata {
            if let Some(evt) = combat_data.event() {
                if let Some(agt) = combat_data.src() {
                    let agt = AgentOwned::from(unsafe { ptr::read(agt) });
                    let event_send = sender.try_send(TaimiThreadEvent::CombatEvent {
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
                    let event_send = sender.try_send(TaimiThreadEvent::MumbleIdentityUpdated(copied_identity));
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
    let event_send = sender.try_send(TaimiThreadEvent::Quit);
    drop(event_send);
}
