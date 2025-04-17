use {
    super::TimerWindowState, crate::{
        ControllerEvent,
        SETTINGS,
        TS_SENDER,
    }, nexus::imgui::Ui
};

pub struct ConfigTabState {
}

impl ConfigTabState {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            timer_window_state.stock_progress_bar = settings.stock_progress_bar;
        };
        if ui.checkbox("Stock Imgui Progress Bar", &mut timer_window_state.stock_progress_bar)  {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(timer_window_state.stock_progress_bar));
                drop(event_send);
        }
    }
}



