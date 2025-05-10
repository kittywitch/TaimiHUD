use {
    super::Alignment, crate::{
        controller::ControllerEvent, marker::{atomic::{LocalPoint, MapPoint, MarkerInputData, ScreenPoint}, format::MarkerFile}, render::{RenderState, TimerWindowState}, settings::{RemoteSource, TimerSettings}, timer::TimerFile, CONTROLLER_SENDER, SETTINGS
    }, glam::{Vec2, Vec3}, glamour::TransformMap, indexmap::IndexMap, nexus::{gamebind::invoke_gamebind_async, imgui::{ChildWindow, Condition, ConfigFlags, Context, Selectable, TreeNode, TreeNodeFlags, Ui, WindowFlags}, wnd_proc::send_wnd_proc_to_game}, std::{collections::HashSet, sync::Arc}, windows::Win32::{Foundation::WPARAM, UI::WindowsAndMessaging::WM_MOUSEMOVE}
};

pub struct MarkerTabState {
    markers: Option<Arc<MarkerFile>>,
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
        ui.text(format!("Mouse Location: {:?}", mouse_pos));
        ui.text(format!("Window Size: {:?}", display_size));
        if let Some(mid) = &mid {
            let world_from_cursor = mid.map_screen_to_map(mouse_pos.into());
            let compass_center_screen = mid.fakespace_minimap_bound().center() * mid.scaling;
            ui.text_wrapped(format!("Compass center: {:?}, Cursor in map: {:?}", compass_center_screen, world_from_cursor));
            ui.text_wrapped(format!("{:?}", &mid));
            let local_position: LocalPoint = mid.local_player_pos.into();
            let map_position: MapPoint = mid.global_player_pos.into();
            ui.text_wrapped(format!(
                "self, L:{:?} G:{:?}, L->G:{:?}, G->L:{:?}",
                mid.local_player_pos,
                mid.global_player_pos,
                mid.map_local_to_map(local_position),
                mid.map_map_to_local(map_position),
            ));
        }

        if let Some(markers) = &self.markers {
            for category in &markers.categories {
                    RenderState::font_text("big", ui, &category.name);
                for marker_set in &category.marker_sets {
                    RenderState::font_text("ui", ui, &marker_set.name);
                    RenderState::font_text("ui", ui, &format!("Author: {}", &marker_set.author));
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
                    for marker in &marker_set.markers {
                        let position: LocalPoint = Vec3::from(marker.position.clone()).into();
                        if let Some(mid) = &mid {
                            let map_position = mid.map_local_to_map(position);
                            let screen_position = mid.map_map_to_screen(map_position);
                            ui.text_wrapped(&format!("{} marker {:?}: L;{:?}, M;{:?}, S;{:?}", marker.marker, marker.id, position, map_position, screen_position));
                                    /*let coordinates_isize = ((screen_position.x as usize) << 16 | screen_position.y as usize) as isize;
                                    log::debug!("coordinates: {:?}, {:?}", coordinates_isize, coordinates_isize.to_ne_bytes());
                                    let coordinates = windows::Win32::Foundation::LPARAM(coordinates_isize);
                                    let wnd_result = send_wnd_proc_to_game(windows::Win32::Foundation::HWND::default(), WM_MOUSEMOVE, WPARAM::default(), coordinates);
                                    // milliseconds
                                    invoke_gamebind_async(marker.marker.to_place_world_gamebind(), 100i32);
                                    log::debug!("set_marker result: {wnd_result:?}");*/
                            }
                    }
                    pushy.pop();

                }
            }
        } else {
            ui.text("No markers loaded.");
        }
    }

    pub fn marker_update(&mut self, markers: Arc<MarkerFile>) {
        self.markers = Some(markers);
    }
}
