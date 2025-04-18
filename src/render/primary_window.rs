use {
    crate::{
        SETTINGS,
        TS_SENDER,
        ControllerEvent,
        render::{
            ConfigTabState,
            TimerWindowState,
            TimerTabState,
            DataSourceTabState,
            InfoTabState,
        },
    },
    nexus::imgui::{
        Ui,
        Window,
    },
};


pub struct PrimaryWindowState {
    pub config_tab: ConfigTabState,
    pub timer_tab: TimerTabState,
    pub data_sources_tab: DataSourceTabState,
    pub info_tab: InfoTabState,
    open: bool,
}

impl PrimaryWindowState {
 pub fn new() -> Self {
        Self {
            config_tab: ConfigTabState::new(),
            timer_tab: TimerTabState::new(),
            data_sources_tab: DataSourceTabState::new(),
            info_tab: InfoTabState::new(),
            open: false,
        }
    }

    pub fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        let mut open = self.open;
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            open = settings.primary_window_open;
        };
        if open {
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
                    if let Some(_token) = ui.tab_item("Config") {
                        self.config_tab.draw(ui, timer_window_state);
                    }
                    if let Some(_token) = ui.tab_item("Info") {
                        self.info_tab.draw(ui, timer_window_state);
                    }
                }
            });
        }
        if open != self.open {
            let sender = TS_SENDER.get().unwrap();
            let event_send = sender.try_send(ControllerEvent::WindowState("primary".to_string(), open));
            drop(event_send);
            self.open = open;
        }
    }

    pub fn keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            let sender = TS_SENDER.get().unwrap();
            let event_send = sender.try_send(ControllerEvent::WindowState("primary".to_string(), !self.open));
            drop(event_send);
        }
    }
}

