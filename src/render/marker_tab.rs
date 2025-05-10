use {
    super::Alignment, crate::{
        controller::ControllerEvent, marker::{atomic::{LocalPoint, MapPoint, MarkerInputData, ScreenPoint, SignObtainer}, format::{MarkerFile, MarkerFormats, MarkerSet, RuntimeMarkers}}, render::{RenderState, TimerWindowState}, settings::{RemoteSource, TimerSettings}, timer::TimerFile, CONTROLLER_SENDER, SETTINGS
    }, glam::{Vec2, Vec3}, glamour::TransformMap, indexmap::IndexMap, nexus::{gamebind::invoke_gamebind_async, imgui::{ChildWindow, Condition, ConfigFlags, Context, Selectable, TableColumnSetup, TableFlags, TreeNode, TreeNodeFlags, Ui, WindowFlags}, wnd_proc::send_wnd_proc_to_game}, std::{collections::HashSet, sync::Arc}, windows::Win32::{Foundation::WPARAM, UI::WindowsAndMessaging::WM_MOUSEMOVE}
};

pub struct MarkerTabState {
    markers: Vec<Arc<RuntimeMarkers>>,
}

impl MarkerTabState {
    pub fn new() -> Self {
        MarkerInputData::create();

        Self {
            markers: Default::default(),
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        let io = ui.io();
        let mouse_pos = Vec2::from_array(io.mouse_pos);
        let display_size = Vec2::from_array(io.display_size);
        let mid = MarkerInputData::read();
        if let Some(mid) = &mid {
            let sign =  mid.sign_obtainer.sign();
            let meep = SignObtainer::meters_per_feet();
            let sign_unity = sign / meep;
            ui.text_wrapped(format!("Current scaling factor: {:.2},{:.2}", sign.x, sign.y));
            ui.text_wrapped(format!("Scaling factor as multiple of ft per continent unit: {:.2},{:.2}", sign_unity.x, sign_unity.y));
        }

        for marker_pack in  &self.markers {
            match &marker_pack.file {
                MarkerFormats::File(f) => {
            for category in &f.categories {
                    RenderState::font_text("big", ui, &category.name);
                    if let Some(path) = &marker_pack.path {
                        ui.text_wrapped(format!("File: {:?}", path));
                    }
                for marker_set in &category.marker_sets {
                            self.display_marker_set(ui, mid.clone(), marker_set);
                }
            }
                },
                MarkerFormats::Custom(c) => {
                    RenderState::font_text("big", ui, "Custom Markers - No categories");
                    if let Some(path) = &marker_pack.path {
                        ui.text_wrapped(format!("File: {:?}", path));
                    }
                    for marker_set in &c.squad_marker_preset {
                        self.display_marker_set(ui, mid.clone(), marker_set);
                    }
                },
            }

        }
    }

    pub fn marker_update(&mut self, markers: Vec<Arc<RuntimeMarkers>>) {
        self.markers = markers;
    }

    pub fn display_marker_set(&self, ui: &Ui, mid: Option<Arc<MarkerInputData>>, marker_set: &MarkerSet) {
        RenderState::font_text("ui", ui, &marker_set.name);
        if let Some(author) = &marker_set.author {
            RenderState::font_text("ui", ui, &format!("Author: {}", author));
        }
        RenderState::font_text("ui", ui, &format!("{}", &marker_set.description));
        ui.text(&format!("Map ID: {}", &marker_set.map_id));
        ui.text(&format!("Markers: {}", &marker_set.markers.len()));
        let pushy = ui.push_id(&marker_set.name);
        let screen_positions: Vec<ScreenPoint> = marker_set.markers.iter().flat_map(|x| {
            if let Some(mid) = &mid {
                let position: LocalPoint = Vec3::from(x.position.clone()).into();
                let map = mid.map_local_to_map(position);

                mid.map_map_to_screen(map)
            } else {
                None
            }
        }).collect();
        if screen_positions.len() == marker_set.markers.len() {
                    if ui.button("Set markers") {
                        let sender = CONTROLLER_SENDER.get().unwrap();
                        let event_send = sender.try_send(ControllerEvent::SetMarker(screen_positions, marker_set.clone()));
                        drop(event_send);
                    }
        }
        let table_flags = TableFlags::RESIZABLE | TableFlags::ROW_BG | TableFlags::BORDERS;
        let table_name = format!("markers_for_{}", marker_set.name);
        let table_token = ui.begin_table_header_with_flags(
            &table_name,
                [
                TableColumnSetup::new("Marker Type"),
                TableColumnSetup::new("Description"),
                TableColumnSetup::new("Local (XYZ)"),
                TableColumnSetup::new("Map (XY)"),
                TableColumnSetup::new("Screen (XY)"),
            ],
            table_flags,
        );
        ui.table_next_column();
        for marker in &marker_set.markers {
            // marker marker on the table
            ui.text_wrapped(format!("{}", marker.marker));
            ui.table_next_column();
            if let Some(description) = &marker.id {
                ui.text_wrapped(description);
            } else {
                ui.text_wrapped("N/A");
            }
            ui.table_next_column();
            let position: LocalPoint = Vec3::from(marker.position.clone()).into();
            ui.text_wrapped(format!("({:.2}, {:.2}, {:.2})", position.x, position.y, position.z));
            ui.table_next_column();
            if let Some(mid) = &mid {
                let map_position = mid.map_local_to_map(position);
                ui.text_wrapped(format!("({:.2}, {:.2})", map_position.x, map_position.y));
                ui.table_next_column();
                if let Some(screen_position) = mid.map_map_to_screen(map_position) {
                    ui.text_wrapped(format!("({:.2}, {:.2})", screen_position.x, screen_position.y));
                } else {
                    ui.text_wrapped("Not on screen?");
                }
                ui.table_next_column();
            } else {
                ui.text_wrapped("N/A");
                ui.table_next_column();
                ui.text_wrapped("N/A");
                ui.table_next_column();
            }
        }
        drop(table_token);
    }
}
