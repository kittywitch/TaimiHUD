use {
    crate::render::RenderState,
    nexus::imgui::Ui,
};

pub struct InfoTabState {}

impl InfoTabState {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn draw(&self, ui: &Ui) {
        let name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let version = env!("CARGO_PKG_VERSION");
        let profile = match () {
            #[cfg(debug_assertions)]
            _ => "debug",
            #[cfg(not(debug_assertions))]
            _ => "release",
        };
        let project_heading = format!("{}, v{} by {}", name, version, authors);
        RenderState::font_text("big", ui, &project_heading);
        let profile_info = format!("Built in the {} profile.", profile);
        ui.text(profile_info);
        ui.new_line();
        let description = env!("CARGO_PKG_DESCRIPTION");
        ui.text(description);
        ui.new_line();
        ui.text("If you need keybind based timer triggers, please bind the appropriate keys in the Nexus settings.");
    }
}


