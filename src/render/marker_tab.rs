use {
    super::{Alignment, RenderEvent}, crate::{
        controller::ControllerEvent, marker::{atomic::{LocalPoint, MapPoint, MarkerInputData, ScreenPoint, SignObtainer}, format::{MarkerFile, MarkerFormats, MarkerSet, RuntimeMarkers}}, render::{RenderState, TimerWindowState}, settings::{RemoteSource, TimerSettings}, timer::TimerFile, CONTROLLER_SENDER, RENDER_SENDER, SETTINGS
    }, glam::{Vec2, Vec3}, glamour::TransformMap, indexmap::IndexMap, nexus::{gamebind::invoke_gamebind_async, imgui::{ChildWindow, Condition, ConfigFlags, Context, Selectable, TableColumnSetup, TableFlags, TreeNode, TreeNodeFlags, Ui, WindowFlags}, wnd_proc::send_wnd_proc_to_game}, std::{collections::{HashMap, HashSet}, sync::Arc}, windows::Win32::{Foundation::WPARAM, UI::WindowsAndMessaging::WM_MOUSEMOVE}
};

pub struct MarkerTabState {
    markers: IndexMap<String, Vec<Arc<MarkerSet>>>,
    marker_selection: Option<Arc<MarkerSet>>,
    category_status: HashSet<String>,
}

impl MarkerTabState {
    pub fn new() -> Self {
        MarkerInputData::create();

        Self {
            markers: Default::default(),
            marker_selection: Default::default(),
            category_status: Default::default(),
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        ui.columns(2, "marker_tab_start", true);
        self.draw_sidebar(ui);
        ui.next_column();
        self.draw_main(ui);
        ui.columns(1, "marker_tab_end", false)
    }

    fn draw_sidebar(&mut self, ui: &Ui) {
        self.draw_sidebar_header(ui);
        self.draw_sidebar_child(ui);
    }
    fn draw_sidebar_header(&mut self, ui: &Ui) {
        if ui.button("Create Marker Set") {
            
            let _ = RENDER_SENDER.get()
                .unwrap()
                .try_send(RenderEvent::OpenEditMarkers);
        }
        if self.category_status.len() != self.markers.keys().len() {
            if ui.button("Expand All") {
                self.category_status.extend(self.markers.keys().cloned());
            }
        }
        if self.category_status.len() != self.markers.keys().len() && !self.category_status.is_empty() {
            ui.same_line();
        }
        if !self.category_status.is_empty() {
            if ui.button("Collapse All") {
                self.category_status.clear();
            }
        }
    }
    fn draw_sidebar_child(&mut self, ui: &Ui) {
        let child_window_flags = WindowFlags::HORIZONTAL_SCROLLBAR;
        ChildWindow::new("marker_sidebar")
            .flags(child_window_flags)
            .size([0.0, 0.0])
            .build(ui, || {
                let header_flags = TreeNodeFlags::FRAMED;
                // interface design is my passion
                let height = Vec2::from_array(ui.calc_text_size("U\nI"));
                let height = height.y;
                for idx in 0..self.markers.len() {
                    self.draw_category(ui, header_flags, height, idx);
                }
            });
    }

    fn draw_category(&mut self, ui: &Ui, header_flags: TreeNodeFlags, height: f32, idx: usize) {
        let (category_name, category) = self
            .markers
            .get_index(idx)
            .expect("given an incorrect index for the category");
        let category_closure = || {
            ui.dummy([0.0, 4.0]);
            for marker in category {
                let mut selected = false;
                if let Some(selected_marker) = &self.marker_selection {
                    selected = Arc::ptr_eq(selected_marker, marker);
                }
                let element_selected = Self::draw_marker_set_in_sidebar(ui, height, marker, selected);
                if element_selected && element_selected != selected {
                    self.marker_selection = Some(marker.clone());
                }
            }
        };
        let tree_node = TreeNode::new(category_name)
            .flags(header_flags)
            .opened(
                self.category_status.contains(category_name),
                Condition::Always,
            )
            .tree_push_on_open(false)
            .build(ui, category_closure);
        match tree_node {
            Some(_) => {
                self.category_status.insert(category_name.to_string());
            }
            None => {
                self.category_status.remove(category_name);
            }
        }
    }

    fn draw_marker_set_in_sidebar(ui: &Ui, height: f32, marker: &Arc<MarkerSet>, selected_in: bool) -> bool {
        let mut selected = selected_in;
        let group_token = ui.begin_group();
        let widget_pos = Vec2::from(ui.cursor_pos());
        let window_size = Vec2::from(ui.window_content_region_max());
        let widget_size = window_size.with_y(height);
        if Selectable::new(&marker.combined())
            .selected(selected)
            .build(ui)
        {
            selected = true;
        }
        /*if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            let settings_for_marker = settings.markers.get(&marker.id);
            ui.same_line();
            let (color, text) = match settings_for_marker {
                Some(markerSettings { disabled: true, .. }) => ([1.0, 0.0, 0.0, 1.0], "Disabled"),
                _ => ([0.0, 1.0, 0.0, 1.0], "Enabled"),
            };
            let text_size = Vec2::from(ui.calc_text_size(text));
            Alignment::set_cursor(
                ui,
                Alignment::RIGHT_MIDDLE,
                widget_pos,
                widget_size,
                text_size,
            );
            ui.text_colored(color, text);
        }*/
        ui.dummy([0.0, 4.0]);
        group_token.end();
        selected
    }

    fn draw_main(&mut self, ui: &Ui) {
        let child_window_flags = WindowFlags::HORIZONTAL_SCROLLBAR;
        ChildWindow::new("timer_main")
            .flags(child_window_flags)
            .size([0.0, 0.0])
            .build(ui, || {
                let mid = MarkerInputData::read();
                if let Some(mid) = &mid {
                    let sign =  mid.sign_obtainer.sign();
                    let meep = SignObtainer::meters_per_feet();
                    let sign_unity = sign / meep;
                    ui.text_wrapped(format!("Current scaling factor: ({:.2}, {:.2})", sign.x, sign.y));
                    ui.text_wrapped(format!("Scaling factor as multiple of ft per continent unit: ({:.2}, {:.2})", sign_unity.x, sign_unity.y));
                    if ui.button("Reset detected scaling factor") {
                        MarkerInputData::reset_signobtainer();
                    }
                    ui.dummy([4.0; 2]);
                    ui.separator();
                    ui.dummy([4.0; 2]);
                }
                if let Some(selected_marker_set) = &self.marker_selection {
                        let pushy = ui.push_id(&selected_marker_set.name);
                        RenderState::font_text("big", ui, &selected_marker_set.name);
                        if let Some(author) = &selected_marker_set.author {
                            RenderState::font_text("ui", ui, &format!("Author: {}", author));
                        }
                        if let Some(path) = &selected_marker_set.path {
                            ui.text_wrapped(format!("From file: {path:?}"));
                        } else {
                            ui.text_wrapped(format!("Couldn't find associated file"));
                        
                        }
                        RenderState::font_text("ui", ui, &format!("{}", &selected_marker_set.description));
                        ui.text(&format!("Map ID: {}", &selected_marker_set.map_id));
                        ui.text(&format!("Markers: {}", &selected_marker_set.markers.len()));
                        let screen_positions: Vec<ScreenPoint> = selected_marker_set.markers.iter().flat_map(|x| {
                            if let Some(mid) = &mid {
                                let position: LocalPoint = Vec3::from(x.position.clone()).into();
                                let map = mid.map_local_to_map(position);

                                mid.map_map_to_screen(map)
                            } else {
                                None
                            }
                        }).collect();
                        ui.dummy([4.0; 2]);
                        let table_flags = TableFlags::RESIZABLE | TableFlags::ROW_BG | TableFlags::BORDERS;
                        let table_name = format!("markers_for_{}", selected_marker_set.name);
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
                        for marker in &selected_marker_set.markers {
                            // marker marker on the table
                            ui.text_wrapped(format!("{}", marker.marker));
                            ui.table_next_column();
                            if let Some(description) = &marker.id {
                                if !description.is_empty() {
                                    ui.text_wrapped(description);
                                } else {
                                ui.text_wrapped("N/A");
                                }

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
                        if screen_positions.len() == selected_marker_set.markers.len() {
                                ui.dummy([4.0; 2]);
                                if ui.button("Place Markers") {
                                    let sender = CONTROLLER_SENDER.get().unwrap();
                                    let event_send = sender.try_send(ControllerEvent::SetMarker(screen_positions, selected_marker_set.clone()));
                                    drop(event_send);
                                }
                            pushy.pop();
                        }
                        // TODO: add confirm ^^;
                        if ui.button("Delete Marker Set") {
                        }
                    /*ui.dummy([4.0; 2]);
                    ui.separator();
                    ui.dummy([4.0; 2]);
                    if let Some(settings) =
                        SETTINGS.get().and_then(|settings| settings.try_read().ok())
                    {
                        let settings_for_marker = settings.markers.get(&selected_marker.id);
                        let button_text = match settings_for_marker {
                            Some(markerSettings { disabled: true, .. }) => "Enable",
                            _ => "Disable",
                        };
                        if ui.button(button_text) {
                            let sender = CONTROLLER_SENDER.get().unwrap();
                            let event_send = sender
                                .try_send(ControllerEvent::markerToggle(selected_marker.id.clone()));
                            drop(event_send);
                        }
                    }*/
                } else {
                    ui.text("Please select a marker to configure!");
                }
            });
    }
    pub fn marker_update(&mut self, markers: HashMap<String, Vec<Arc<MarkerSet>>>) {
        self.markers.clear();
        for (category, markers) in markers {
            self.markers.insert(category, markers);
        }
        self.markers.sort_keys();
    }
/*
    pub fn draw(&mut self, ui: &Ui) {
        let io = ui.io();
        let mouse_pos = Vec2::from_array(io.mouse_pos);
        let display_size = Vec2::from_array(io.display_size);
        let mid = MarkerInputData::read();
        if let Some(mid) = &mid {
            let sign =  mid.sign_obtainer.sign();
            let meep = SignObtainer::meters_per_feet();
            let sign_unity = sign / meep;
            ui.text_wrapped(format!("Current scaling factor: ({:.2}, {:.2})", sign.x, sign.y));
            ui.text_wrapped(format!("Scaling factor as multiple of ft per continent unit: ({:.2}, {:.2})", sign_unity.x, sign_unity.y));
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


    pub fn display_marker_set(&self, ui: &Ui, mid: Option<Arc<MarkerInputData>>, marker_set: &MarkerSet) {
        let pushy = ui.push_id(&marker_set.name);
        RenderState::font_text("ui", ui, &marker_set.name);
        if let Some(author) = &marker_set.author {
            RenderState::font_text("ui", ui, &format!("Author: {}", author));
        }
        RenderState::font_text("ui", ui, &format!("{}", &marker_set.description));
        ui.text(&format!("Map ID: {}", &marker_set.map_id));
        ui.text(&format!("Markers: {}", &marker_set.markers.len()));
        let screen_positions: Vec<ScreenPoint> = marker_set.markers.iter().flat_map(|x| {
            if let Some(mid) = &mid {
                let position: LocalPoint = Vec3::from(x.position.clone()).into();
                let map = mid.map_local_to_map(position);

                mid.map_map_to_screen(map)
            } else {
                None
            }
        }).collect();
        ui.dummy([4.0; 2]);
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
                if !description.is_empty() {
                    ui.text_wrapped(description);
                } else {
                ui.text_wrapped("N/A");
                }

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
        if screen_positions.len() == marker_set.markers.len() {
                ui.dummy([4.0; 2]);
                if ui.button("Place Markers") {
                    let sender = CONTROLLER_SENDER.get().unwrap();
                    let event_send = sender.try_send(ControllerEvent::SetMarker(screen_positions, marker_set.clone()));
                    drop(event_send);
                }
            pushy.pop();
        }
    }*/
}
