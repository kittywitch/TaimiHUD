mod controller;
mod render;
mod settings;
mod timer;

#[cfg(feature = "markers")]
mod marker;

#[cfg(feature = "space")]
mod space;

use std::ffi::{c_char, CString};

use nexus::{event::arc::ACCOUNT_NAME, imgui::{MenuItem, Ui}, localization::translate, quick_access::{add_quick_access_context_menu, notify_quick_access}};
use settings::{SourcesFile};
#[cfg(feature = "space")]
use space::{engine::SpaceEvent, resources::Texture, Engine};
use tokio::sync::OnceCell;
use {
    crate::{
        controller::{Controller, ControllerEvent},
        render::{RenderEvent, RenderState},
        settings::SettingsLock,
    },
    arcdps::AgentOwned,
    nexus::{
        event::{
            Event,
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
        ffi::CStr,
        sync::{Arc, Mutex, OnceLock, RwLock, LazyLock},
        thread::{self, JoinHandle},
    },
    tokio::sync::mpsc::{channel, Sender},
};
use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    DefaultLocalizer, LanguageLoader, RustEmbedNotifyAssets,
};
use unic_langid_impl::LanguageIdentifier;
//use i18n_embed_fl::fl;
use rust_embed::RustEmbed;

// https://github.com/kellpossible/cargo-i18n/blob/95634c35eb68643d4a08ff4cd17406645e428576/i18n-embed/examples/library-fluent/src/lib.rs
#[derive(RustEmbed)]
#[folder = "i18n/"]
pub struct LocalizationsEmbed;

pub static LOCALIZATIONS: LazyLock<RustEmbedNotifyAssets<LocalizationsEmbed>> = LazyLock::new(|| {
    RustEmbedNotifyAssets::new(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("i18n/"))
});

static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_available_languages(&*LOCALIZATIONS)
        .expect("Error while loading fallback language");
    loader.set_use_isolating(false);

    loader
});

#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

pub fn localizer() -> DefaultLocalizer<'static> {
    DefaultLocalizer::new(&*LANGUAGE_LOADER, &*LOCALIZATIONS)
}

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[cfg(feature = "space")]
static TEXTURES: OnceLock<RwLock<HashMap<PathBuf, Arc<Texture>>>> = OnceLock::new();
static IMGUI_TEXTURES: OnceLock<RwLock<HashMap<String, Arc<NexusTexture>>>> = OnceLock::new();
static CONTROLLER_SENDER: OnceLock<Sender<ControllerEvent>> = OnceLock::new();
static RENDER_SENDER: OnceLock<Sender<RenderEvent>> = OnceLock::new();
static ACCOUNT_NAME_CELL: OnceLock<String> = OnceLock::new();

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

static SOURCES: OnceLock<Arc<RwLock<SourcesFile>>> = OnceLock::new();
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

    reload_language();

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
            let sender = CONTROLLER_SENDER.get().unwrap();
            let _ =
                sender.try_send(ControllerEvent::WindowState("primary".to_string(), None));
        }
    });

    register_keybind_with_string(
        fl!("primary-window-toggle"),
        main_window_keybind_handler,
        "ALT+SHIFT+M",
    )
    .revert_on_unload();

    // Handle window toggling with keybind and button
    let timer_window_keybind_handler = keybind_handler!(|_id, is_release| {
        if !is_release {
            let sender = CONTROLLER_SENDER.get().unwrap();
            let _ =
                sender.try_send(ControllerEvent::WindowState("timers".to_string(), None));
        }
    });

    register_keybind_with_string(
        fl!("timer-window-toggle"),
        timer_window_keybind_handler,
        "ALT+SHIFT+K",
    )
    .revert_on_unload();

    let event_trigger_keybind_handler = keybind_handler!(|id, is_release| {
        let sender = CONTROLLER_SENDER.get().unwrap();
        let _ = sender.try_send(ControllerEvent::TimerKeyTrigger(id.to_string(), is_release));
    });
    for i in 0..5 {
        register_keybind_with_string(
            fl!("timer-key-trigger", id = format!("{}", i)),
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

    let same_identifier = "TAIMI_BUTTON";

    add_quick_access(
        same_identifier,
        "TAIMI_ICON",
        "TAIMI_ICON_HOVER",
        fl!("primary-window-toggle"),
        fl!("primary-window-toggle-text"),
    )
    .revert_on_unload();


    add_quick_access_context_menu(
        "TAIMI_MENU",
        Some(same_identifier), // maybe some day
        //None::<&str>,
        render!(|ui| {
            if ui.button("Timers") {
                let sender = CONTROLLER_SENDER.get().unwrap();
                let _ =
                    sender.try_send(ControllerEvent::WindowState("timers".to_string(), None));
            }
            if ui.button("Primary") {
                let sender = CONTROLLER_SENDER.get().unwrap();
                let _ =
                    sender.try_send(ControllerEvent::WindowState("primary".to_string(), None));
            }

    })
    )
    .revert_on_unload();

    ACCOUNT_NAME
        .subscribe(event_consume!(<c_char> |name| {
            if let Some(name) = name {
                let name = unsafe {CStr::from_ptr(name as *const c_char)};
                let name = name.to_string_lossy().to_string();
                log::info!("Received account name: {name:?}");
                match ACCOUNT_NAME_CELL.set(name) {
                    Ok(_) => (),
                    Err(err) => log::error!("Error with account name cell: {err}"),
                }
            }
        }))
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

    pub const EV_LANGUAGE_CHANGED: Event<()> = unsafe { Event::new("EV_LANGUAGE_CHANGED") };

    // I don't want to store the localization data in either Nexus or communicate it with Nexus,
    // because this would mean entirely being beholden to Nexus as the addon's loader for the
    // rest of all time.
    EV_LANGUAGE_CHANGED.subscribe(event_consume!(
        <()> |_| {
            reload_language();
        }
    )).revert_on_unload();
}

fn detect_language() -> String {
    let index_to_check = "KB_CHANGELOG";
    let mut language_map = HashMap::new();
    language_map.insert("Registro de Alterações", "pt-br");
    language_map.insert("更新日志", "cn");
    language_map.insert("Seznam změn", "cz");
    language_map.insert("Änderungsprotokoll", "de");
    language_map.insert("Changelog", "en");
    language_map.insert("Notas del parche", "es");
    language_map.insert("Journal des modifications", "fr");
    language_map.insert("Registro modifiche", "it");
    language_map.insert("Lista zmian", "pl");
    language_map.insert("Список изменений", "ru");
    let translated_index = translate(index_to_check).expect("Couldn't translate string");
    let language = language_map.get(&translated_index.as_str());
    if let Some(language) = language {
        return language.to_string()
    } else {
        return "en".to_string()
    }
}

fn reload_language() {
    let detected_language = detect_language();
    log::info!("Detected language {detected_language} for internationalization");
    let detected_language_identifier: LanguageIdentifier = detected_language.parse().expect("Cannot parse detected language");
    let get_language = vec![detected_language_identifier];
    i18n_embed::select(&*LANGUAGE_LOADER, &*LOCALIZATIONS, get_language.as_slice()).expect("Couldn't load language!");
    (&*LANGUAGE_LOADER).set_use_isolating(false);
}

fn unload() {
    log::info!("Unloading addon");
    #[cfg(feature = "space")]
    let _ = ENGINE.set(None);
    #[cfg(feature = "space")]
    let _ = TEXTURES.set(Default::default());
    let _ = IMGUI_TEXTURES.set(Default::default());
    /*ENGINE.with_borrow_mut(|e| {
        //#[cfg(todo)]
        //e.cleanup();
    });*/
    let sender = CONTROLLER_SENDER.get().unwrap();
    let event_send = sender.try_send(ControllerEvent::Quit);
    drop(event_send);
}
