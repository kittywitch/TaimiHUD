use {
    super::TimerWindowState,
    crate::{built_info, render::RenderState, SETTINGS},
    nexus::imgui::{TableColumnSetup, Ui},
};

#[cfg(feature = "space")]
use crate::{ENGINE, ENGINE_INITIALIZED, TEXTURES};

pub struct InfoTabState {}

impl InfoTabState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&self, ui: &Ui, timer_window_state: &TimerWindowState) {
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let version = env!("CARGO_PKG_VERSION");
        let version = env!("CARGO_PKG_VERSION");
        let profile = match () {
            #[cfg(debug_assertions)]
            _ => "debug",
            #[cfg(not(debug_assertions))]
            _ => "release",
        };

        let project_heading = format!("{}, {} by {}", name, version, authors);
        RenderState::font_text("big", ui, &project_heading);

        let in_ci = match built_info::CI_PLATFORM {
            Some(platform) => format!(" using {platform}"),
            None => "".to_string(),
        };
        if let (Some(git_head_ref), Some(git_hash)) = (built_info::GIT_HEAD_REF, built_info::GIT_COMMIT_HASH_SHORT) {
            let mut build = format!("Built from {}@{}", git_head_ref, git_hash);
            build.push_str(&in_ci);
            build.push_str(&format!(", in profile \"{profile}\""));
            build.push('.');
            ui.text_wrapped(build);
        }
        ui.dummy([4.0, 4.0]);
        let description = env!("CARGO_PKG_DESCRIPTION");
        ui.text_wrapped(description);
        ui.dummy([4.0, 4.0]);
        ui.text_wrapped("If you need keybind-based timer triggers, please bind the appropriate keys in the Nexus settings.");
        ui.separator();
        RenderState::font_text("ui", ui, "Active Phase States");
        let table_token = ui.begin_table_header(
            "phase_states",
            [
                TableColumnSetup::new("Timer"),
                TableColumnSetup::new("Phase"),
            ],
        );
        ui.table_next_column();
        for phase_state in &timer_window_state.phase_states {
            let phase = phase_state.phase.phase();
            ui.text_wrapped(phase_state.timer.hypheny_name());
            ui.table_next_column();
            ui.text_wrapped(&phase.name);
            ui.table_next_column();
        }
        drop(table_token);
        #[cfg(feature = "space")]
        self.space_info(ui);
    }

    #[cfg(feature = "space")]
    pub fn space_info(&self, ui: &Ui) {
        RenderState::font_text("big", ui, "Engine");
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            if settings.enable_katrender && ENGINE_INITIALIZED.get() {
                ENGINE.with_borrow(|e| {
                    if let Some(engine) = e {
                        RenderState::font_text("ui", ui, "ECS Data");
                        let entities = engine.world.entities();
                        let used_entities = entities.used_count();
                        let total_entities = entities.total_count();
                        ui.text(format!("Used: {}", used_entities));
                        ui.text(format!("Total: {}", total_entities));
                        RenderState::font_text("ui", ui, "Object Data");
                        let table_token = ui.begin_table_header(
                            "object_types",
                            [TableColumnSetup::new("Object Kind")],
                        );
                        ui.table_next_column();
                        for object in engine.object_kinds.keys() {
                            ui.text(object);
                            ui.table_next_column();
                        }
                        drop(table_token);
                        RenderState::font_text("ui", ui, "Model Files");
                        let table_token = ui.begin_table_header(
                            "model_files",
                            [
                                TableColumnSetup::new("Name"),
                                TableColumnSetup::new("Path"),
                                TableColumnSetup::new("Vertices"),
                            ],
                        );
                        ui.table_next_column();
                        for (path, file) in &engine.model_files {
                            for model in &file.models {
                                ui.text(format!("{:?}", path));
                                ui.table_next_column();
                                ui.text(&model.0.name);
                                ui.table_next_column();
                                ui.text(format!("{}", model.0.mesh.positions.len() / 3));
                                ui.table_next_column();
                            }
                        }
                        drop(table_token);
                    }
                });
                let tex_store = TEXTURES.get().unwrap();
                let tex_lock = tex_store.read().unwrap();
                ui.text(format!("Textures: {}", tex_lock.keys().len()));
                ui.text(format!("Mouse Location: {:?}", ui.io().mouse_pos));
                ui.text(format!("Window Size: {:?}", ui.io().display_size));
            }
        }
    }
}
