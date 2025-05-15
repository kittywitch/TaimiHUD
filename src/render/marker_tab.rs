use {
    super::{Alignment, RenderEvent}, crate::{fl,
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
        #[cfg(feature = "markers-edit")]
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
                    let sign_x = format!("{:.2}", sign.x);
                    let sign_y = format!("{:.2}", sign.y);
                    ui.text_wrapped(&fl!("current-scaling-factor", x = sign_x, y = sign_y));
                    let sign_unity_x = format!("{:.2}", sign_unity.x);
                    let sign_unity_y = format!("{:.2}", sign_unity.y);
                    ui.text_wrapped(&fl!("current-scaling-factor-multiple", x = sign_unity_x, y = sign_unity_y));
                    if ui.button(&fl!("scaling-factor-reset")) {
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
                            RenderState::font_text("ui", ui, &fl!("author-arg", author = author));
                        }
                        if let Some(path) = &selected_marker_set.path {
                            let path_display = format!("{:?}", path);
                            ui.text_wrapped(&fl!("location", path = path_display));
                        }
                        RenderState::font_text("ui", ui, &format!("{}", &selected_marker_set.description));
                        ui.text(&fl!("map-id-arg", id = selected_marker_set.map_id.clone()));
                        ui.text(&fl!("markers-arg", count = selected_marker_set.markers.len()));
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
                                TableColumnSetup::new(&fl!("marker-type")),
                                TableColumnSetup::new(&fl!("description")),
                                TableColumnSetup::new(&fl!("local-header")),
                                TableColumnSetup::new(&fl!("map-header")),
                                TableColumnSetup::new(&fl!("screen-header")),
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
                                ui.text_wrapped(&fl!("not-applicable"));
                                }

                            } else {
                                ui.text_wrapped(&fl!("not-applicable"));
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
                                    ui.text_wrapped(&fl!("marker-not-on-screen"));
                                }
                                ui.table_next_column();
                            } else {
                                ui.text_wrapped(&fl!("not-applicable"));
                                ui.table_next_column();
                                ui.text_wrapped(&fl!("not-applicable"));
                                ui.table_next_column();
                            }
                        }
                        drop(table_token);
                        if screen_positions.len() == selected_marker_set.markers.len() {
                                ui.dummy([4.0; 2]);
                                if ui.button(&fl!("markers-place")) {
                                    let sender = CONTROLLER_SENDER.get().unwrap();
                                    let event_send = sender.try_send(ControllerEvent::SetMarker(screen_positions, selected_marker_set.clone()));
                                    drop(event_send);
                                }
                            pushy.pop();
                        }
                        // TODO: add confirm ^^;
                        if ui.button(&fl!("marker-set-delete")) {
                        }
                } else {
                    ui.text(&fl!("select-a-marker"));
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
}
