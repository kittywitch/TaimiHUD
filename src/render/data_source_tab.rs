use {
    crate::{controller::ControllerEvent, settings::NeedsUpdate, CONTROLLER_SENDER, SETTINGS},
    nexus::imgui::Ui,
};

pub struct DataSourceTabState {
    pub checking_for_updates: bool,
}

impl DataSourceTabState {
    pub fn new() -> Self {
        Self {
            checking_for_updates: false,
        }
    }

    pub fn draw(&self, ui: &Ui) {
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            if self.checking_for_updates {
                ui.text("Checking for updates! Please hold.")
            } else {
                if ui.button("Check for updates") {
                    let sender = CONTROLLER_SENDER.get().unwrap();
                    let event_send = sender.try_send(ControllerEvent::CheckDataSourceUpdates);
                    drop(event_send);
                }
                ui.same_line();
                if let Some(last_checked) = &settings.last_checked {
                    ui.text(format!(
                        "Last checked for updates: {}",
                        last_checked.format("%F %T %Z")
                    ));
                } else {
                    ui.text("Last checked for updates: Never");
                }
                for download_data in &settings.remotes {
                    let source = download_data.source.clone();
                    ui.text(format!("Remote: {}", source));
                    ui.text(format!("Status: {}", download_data.needs_update));
                    use NeedsUpdate::*;
                    let button_text = match &download_data.needs_update {
                        Unknown => Some("Attempt to update anyway?"),
                        Known(true, _id) => Some("Update"),
                        Known(false, _id) => None,
                    };
                    if let Some(button_text) = button_text {
                        if ui.button(button_text) {
                            let sender = CONTROLLER_SENDER.get().unwrap();
                            let source = source.clone();
                            let event_send =
                                sender.try_send(ControllerEvent::DoDataSourceUpdate { source });
                            drop(event_send);
                        }
                    }
                }
            }
        } else {
            ui.text("SettingsLock have not yet loaded!");
        }
    }
}
