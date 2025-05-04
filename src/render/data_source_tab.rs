use {
    crate::{controller::ControllerEvent, render::RenderState, settings::{NeedsUpdate, RemoteState}, CONTROLLER_SENDER, SETTINGS},
    nexus::imgui::{im_str, PopupModal, StyleColor, TableColumnSetup, TableFlags, Ui}, std::{collections::HashMap, ffi::OsStr},
};

pub struct DataSourceTabState {
    pub checking_for_updates: bool,
    pub state_errors: HashMap<String, anyhow::Error>,
}

impl DataSourceTabState {
    pub fn new() -> Self {
        Self {
            checking_for_updates: false,
            state_errors: Default::default(),
        }
    }

    pub fn draw_uninstall(&self, ui: &Ui, rs: &RemoteState) {
        let source_text = &rs.source.repo_string();
        let modal_name = format!("{}: Uninstall?", source_text);
        if ui.button("Uninstall") {
            ui.open_popup(&modal_name);
        }
        if ui.is_item_hovered() {
            if let Some(path) = &rs.installed_path {
                ui.tooltip_text(format!("Location: {:?}", &path));
            }
        }
        if let Some(_token) = PopupModal::new(&modal_name)
            .always_auto_resize(true)
            .begin_popup(ui) {
            ui.text_wrapped(format!("Uninstall {source_text}?"));
            ui.dummy([4.0, 4.0]);
            if let Some(path) = &rs.installed_path {
                ui.text(format!("Installed folder: {path:?}."));
            }
            ui.dummy([4.0, 4.0]);
            let token = ui.push_style_color(StyleColor::Text, [1.0, 0.0, 0.0, 1.0]);
            ui.text("Please be careful! This will delete the folder and anything it contains.");
            token.pop();
            ui.dummy([4.0, 4.0]);
            if ui.button("Uninstall") {
                let sender = CONTROLLER_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::UninstallAddon(rs.source.clone()
                ));
                drop(event_send);
                
                ui.close_current_popup();
            }
            ui.same_line();
            if ui.button("Close") {
                ui.close_current_popup();
            }
        }
    }

    pub fn draw_open_button<
            S: AsRef<str> + std::fmt::Display,
        >(&mut self, ui: &Ui, text: S, openable: String) {
        let entry_name = format!("{text}: {openable:?}");
        let modal_name = format!("{entry_name} Error");
        if ui.button(&text) {
            log::info!("Triggered open {openable:?} for {text}");
            let sender = CONTROLLER_SENDER.get().unwrap();
            let event_send = sender.try_send(
                ControllerEvent::OpenOpenable(entry_name.clone(), openable.clone()));
            drop(event_send);
            match open::that(&openable) {
                Ok(_) => {
                },
                Err(err) => {
                    self.state_errors.insert(entry_name.clone(), err.into());
                }
            }
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!("Location: {:?}", openable));
        }
        if let Some(errory) = &self.state_errors.get(&entry_name) {
            ui.open_popup(&modal_name);
            if let Some(_token) = PopupModal::new(&modal_name)
            .always_auto_resize(true)
            .begin_popup(ui) {
                ui.text_wrapped(format!("Open error for {text}, {openable:?}!"));
                ui.dummy([4.0, 4.0]);
                ui.text_wrapped(format!("{:?}", errory));
                ui.dummy([4.0, 4.0]);
                if ui.button("OK") {
                    self.state_errors.remove(&entry_name);
                    ui.close_current_popup();
                }
            } else {
                ui.close_current_popup();
            }
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
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
                ui.dummy([8.0, 8.0]);
                ui.text_wrapped("I suggest only installing one of the two options for Hero-Timers. Please do your research to determine which you would prefer.");
                ui.dummy([8.0, 8.0]);
                /*for download_data in &settings.remotes {
                    let source = download_data.source.clone();
                    let source_text = source.to_string();
                    RenderState::font_text("big", ui, &source_text);
                    ui.text(format!("Status: {}", download_data.needs_update));
                    ui.dummy([4.0, 4.0]);
                    ui.text_wrapped(format!("Description: {}", download_data.source.description));
                    ui.dummy([4.0, 4.0]);
                    use NeedsUpdate::*;
                    let button_text = match &download_data.needs_update {
                        Unknown => Some("Attempt to update anyway?"),
                        Known(true, _id) => Some("Update"),
                        Known(false, _id) => None,
                        Error(err) => None,
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
                }*/
                let table_flags = TableFlags::RESIZABLE | TableFlags::ROW_BG | TableFlags::BORDERS;
                let table_token = ui.begin_table_header_with_flags(
                    "remotes",
                    [
                        TableColumnSetup::new("Remote"),
                        TableColumnSetup::new("Description"),
                        TableColumnSetup::new("Update Status"),
                        TableColumnSetup::new("Actions"),
                    ],
                    table_flags,
                );
                ui.table_next_column();
                for download_data in &settings.remotes {
                    let source = download_data.source.clone();
                    let source_text = source.to_string();
                    let pushy = ui.push_id(&source_text);
                    ui.text(format!("{}", source));
                    ui.table_next_column();
                    ui.text_wrapped(&download_data.source.description);
                    ui.table_next_column();
                    if let Some(installed) = &download_data.installed_tag {
                        ui.text_wrapped(format!("Installed: {}", installed));
                    } else {
                        ui.text_wrapped("Not installed");
                    }
                    download_data.needs_update.draw(ui);
                    ui.table_next_column();
                    use NeedsUpdate::*;
                    let button_text = match &download_data.needs_update {
                        Unknown => Some("Attempt to update anyway?"),
                        Known(true, _id) => Some("Update"),
                        Known(false, _id) => None,
                        Error(_err) => None,
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
                    self.draw_open_button(ui, "Open Repository", source.repo_url());
                    if let Some(path) = &download_data.installed_path {
                        if let Some (path) = path.to_str() {
                            self.draw_open_button(ui, "Open Folder", path.to_string());
                        }
                        self.draw_uninstall(ui, download_data);
                    }

                    ui.table_next_column();
                    pushy.pop();
                }
                drop(table_token);
            }
        } else {
            ui.text("SettingsLock have not yet loaded!");
        }
    }
}
