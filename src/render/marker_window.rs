use {
    std::sync::Arc,
    glam::Vec3,
    crate::{
        marker::{
            format::MarkerSet,
            atomic::{LocalPoint, ScreenPoint, MarkerInputData},
        },
        fl,
        settings::ProgressBarSettings,
        ControllerEvent, CONTROLLER_SENDER, SETTINGS,
    },
    nexus::imgui::{StyleColor, Ui, Window, TableFlags, Id, TableColumnSetup, TableColumnFlags},
};

pub struct MarkerWindowState {
    pub open: bool,
    pub markers_for_map: Vec<Arc<MarkerSet>>,
}

impl MarkerWindowState {
    pub fn new() -> Self {
        Self {
            markers_for_map: Default::default(),
            open: false,
        }
    }

    pub fn new_map_markers(&mut self, markers: Vec<Arc<MarkerSet>>) {
        self.markers_for_map = markers;
    }

    pub fn draw(&mut self, ui: &Ui) {
        let mut open = self.open;
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            open = settings.markers_window_open;
        };
        if open {
            Window::new(fl!("markers"))
                .size([300.0, 200.0], nexus::imgui::Condition::FirstUseEver)
                .opened(&mut open)
                .build(ui, || {
                    if ui.button(&fl!("clear-markers")) {
                        let sender = CONTROLLER_SENDER.get().unwrap();
                        let event_send = sender.try_send(ControllerEvent::ClearMarkers);
                        drop(event_send);
                    }
                    ui.same_line();
                    if ui.button(&fl!("clear-spent-autoplace")) {
                        let sender = CONTROLLER_SENDER.get().unwrap();
                        let event_send = sender.try_send(ControllerEvent::ClearSpentAutoplace);
                        drop(event_send);
                    }
                    let mid = MarkerInputData::read();
                    if !self.markers_for_map.is_empty() {
                        let table_flags =
                            TableFlags::RESIZABLE | TableFlags::ROW_BG | TableFlags::BORDERS;
                        let table_name = format!("markers_for_map");
                        let table_token = ui.begin_table_header_with_flags(
                            &table_name,
                            [
                                TableColumnSetup {
                                    name: &fl!("name"),
                                    flags: TableColumnFlags::WIDTH_STRETCH,
                                    init_width_or_weight: 0.0,
                                    user_id: Id::Str("name"),
                                },
                                TableColumnSetup {
                                    name: &fl!("category"),
                                    flags: TableColumnFlags::WIDTH_STRETCH,
                                    init_width_or_weight: 0.0,
                                    user_id: Id::Str("category"),
                                },
                                TableColumnSetup {
                                    name: &fl!("description"),
                                    flags: TableColumnFlags::WIDTH_STRETCH,
                                    init_width_or_weight: 0.0,
                                    user_id: Id::Str("description"),
                                },
                                TableColumnSetup {
                                    name: &fl!("actions"),
                                    flags: TableColumnFlags::WIDTH_STRETCH,
                                    init_width_or_weight: 0.0,
                                    user_id: Id::Str("actions"),
                                },
                            ],
                            table_flags,
                        );
                        ui.table_next_column();
                        for marker in &self.markers_for_map {
                            let id_token = ui.push_id(&format!("{}{:?}{:?}", marker.name, marker.author, marker.category));
                            ui.text(format!("{}", marker.name));
                            ui.table_next_column();
                            if let Some(category) = &marker.category {
                                ui.text(format!("{}", category));
                            } else {
                                ui.text("");
                            }
                            ui.table_next_column();
                            ui.text_wrapped(format!("{}", marker.description));
                            ui.table_next_column();
                                if ui.button(&fl!("markers-place")) {
                                    let sender = CONTROLLER_SENDER.get().unwrap();
                                    let event_send = sender.try_send(ControllerEvent::SetMarker(
                                        marker.clone(),
                                    ));
                                    drop(event_send);
                                }
                            ui.table_next_column();
                            id_token.end();
                        }
                            if let Some(token) = table_token {
                                token.end();
                            }
                            } else {
                                ui.text_wrapped(fl!("no-markers-for-map"));
                            }
                });
        }

        if open != self.open {
            let sender = CONTROLLER_SENDER.get().unwrap();
            let event_send = sender.try_send(ControllerEvent::WindowState(
                "markers".to_string(),
                Some(open),
            ));
            drop(event_send);
            self.open = open;
        }
    }
}
