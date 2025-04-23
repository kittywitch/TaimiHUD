use {
    super::TimerWindowState,
    crate::{built_info, render::RenderState},
    nexus::imgui::{TableColumnSetup, Ui},
};

pub struct InfoTabState {}

impl InfoTabState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&self, ui: &Ui, timer_window_state: &TimerWindowState) {
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let version: String;
        if let Some(git_version) = built_info::GIT_VERSION {
            version = format!("v{}, {}", env!("CARGO_PKG_VERSION"), git_version);
        } else {
            version = env!("CARGO_PKG_VERSION").to_string();
        }
        let profile = match () {
            #[cfg(debug_assertions)]
            _ => "debug",
            #[cfg(not(debug_assertions))]
            _ => "release",
        };

        let project_heading = format!("{}, {} by {}", name, version, authors);
        RenderState::font_text("big", ui, &project_heading);
        let profile_info = format!("Built in the {} profile.", profile);
        ui.text(profile_info);
        let description = env!("CARGO_PKG_DESCRIPTION");
        ui.text(description);
        ui.text("If you need keybind based timer triggers, please bind the appropriate keys in the Nexus settings.");
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
            ui.text(phase_state.timer.hypheny_name());
            ui.table_next_column();
            ui.text(&phase.name);
            ui.table_next_column();
        }
        drop(table_token);
    }
}
