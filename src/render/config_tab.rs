use {
    crate::{
        ControllerEvent,
        SETTINGS,
        TS_SENDER,
    },
    nexus::imgui::Ui,
};

pub struct ConfigTabState {
    stock_progress_bar: bool,
}

impl ConfigTabState {
    pub fn new() -> Self {
        Self {
            stock_progress_bar: false,
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            self.stock_progress_bar = settings.stock_progress_bar;
        };
        if ui.checkbox("Stock Progress Bar", &mut self.stock_progress_bar)  {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(self.stock_progress_bar));
                drop(event_send);
        }
    }
}



