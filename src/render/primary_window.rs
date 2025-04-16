use {
    crate::render::{
        TimerWindowState,
        TimerTabState,
        DataSourceTabState,
        InfoTabState,
    },
    nexus::imgui::{
        Ui,
        Window,
    },
};


pub struct PrimaryWindowState {
    pub timer_tab: TimerTabState,
    pub data_sources_tab: DataSourceTabState,
    pub info_tab: InfoTabState,
    open: bool,
}

impl PrimaryWindowState {
 pub fn new() -> Self {
        Self {
            timer_tab: TimerTabState::new(),
            data_sources_tab: DataSourceTabState::new(),
            info_tab: InfoTabState::new(),
            open: true,
        }
    }

    pub fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        let mut open = self.open;
        if self.open {
            Window::new("Taimi").opened(&mut open).build(ui, || {
                if let Some(_token) = ui.tab_bar("modules") {
                    if let Some(_token) = ui.tab_item("Timers") {
                        self.timer_tab.draw(ui, timer_window_state);
                    };
                    if let Some(_token) = ui.tab_item("Markers") {
                        ui.text("To-do!");
                    }
                    if let Some(_token) = ui.tab_item("Data Sources") {
                        self.data_sources_tab.draw(ui);
                    }
                    if let Some(_token) = ui.tab_item("Info") {
                        self.info_tab.draw(ui);
                    }
                }
            });
        }
        self.open = open;
    }

    pub fn keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            self.open = !self.open;
        }
    }
}

