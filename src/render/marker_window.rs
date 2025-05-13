use {
    super::RenderState, crate::{
        marker::{atomic::MarkerInputData, format::MarkerType}, settings::RemoteState, timer::{PhaseState, TimerAlert, TimerFile}, ControllerEvent, ACCOUNT_NAME_CELL, CONTROLLER_SENDER, SETTINGS
    }, glam::{Vec2, Vec3}, nexus::{imgui::{Id, InputTextFlags, ProgressBar, StyleColor, TableColumnFlags, TableColumnSetup, TableFlags, Ui, Window}, paths::get_addon_dir, rtapi::{GroupType, RealTimeApi}}, relative_path::{RelativePath, RelativePathBuf}, std::{f32, path::Path, sync::Arc}
};

pub struct EditMarkerWindowState {
    pub open: bool,
    pub title: String,
    pub description: String,
    pub author: String,
    pub map_id: i32,
    pub markers: [IndividualMarkerState; 8],
}

pub struct IndividualMarkerState {
    pub position: Option<Vec3>,
    pub description: String,
}

impl Default for IndividualMarkerState {
    fn default() -> Self {
        Self {
            position: Default::default(),
            description: "".to_string(),
        
        }
    }
}
impl IndividualMarkerState {
    pub fn set_position(&mut self, pos: Vec3) {
        self.position = Some(pos);
    }
    pub fn set_description(&mut self, desc: String) {
        self.description = desc;
    }
}

impl EditMarkerWindowState {
    pub fn new() -> Self {
        Self {
            open: false,
            title: Default::default(),
            description: Default::default(),
            map_id: Default::default(),
            author: Default::default(),
            markers: Default::default(),
        }
    }

    pub fn open(&mut self, ui: &Ui) {
        *self = Self::new();
        if !self.open {
            let author = match ACCOUNT_NAME_CELL.get() {
                Some(a) => a.clone(),
                None => match RealTimeApi::get() {
                    Some(rtapi) => {
                        if let Some(player_data) = rtapi.read_player() {
                            player_data.account_name
                        } else {
                            "".to_string()
                        }
                    },
                    None => "".to_string(),
                },
            };
            let map_id = if let Some(mid) = MarkerInputData::read() {
                mid.map_id as i32
            } else {
                Default::default()
            };
            self.author = author;
            self.map_id = map_id;
            self.open = true;
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        let mut open = self.open;
        if open {
            let closed = Window::new("Markers")
                .size([300.0, 200.0], nexus::imgui::Condition::FirstUseEver)
                .opened(&mut open).build(ui, || {
                    let title_input = ui.input_text("Title", &mut self.title);
                    title_input.build();
                    let author_input = ui.input_text("Author", &mut self.author);
                    author_input.build();
                    let map_id_input = ui.input_int("Map ID", &mut self.map_id);
                    map_id_input.build();
                    if ui.button("Set Map ID to current map") {
                        if let Some(mid) = MarkerInputData::read() {
                            self.map_id = mid.map_id as i32;
                        }
                    }
                    let description_input = ui.input_text_multiline(
                        "Description",
                        &mut self.description,
                        [0.0, 0.0]);
                    description_input.build();
                    ui.dummy([4.0; 2]);
                    if let Some(rtapi) = RealTimeApi::get() {
                            if let Some(group) = rtapi.read_group() {
                                let is_squad = matches!(group.group_type, Ok(GroupType::Squad | GroupType::RaidSquad));
                                if is_squad {
                                if ui.button("Take from current squad markers") {
                                    for (i, marker) in group.squad_markers.iter().enumerate() {
                                        if *marker != [f32::INFINITY; 3] {
                                            self.markers[i].set_position(Vec3::from_array(*marker));
                                        }
                                    }
                                }
                                } else {
                                    ui.text_colored([1.0, 1.0, 0.0, 1.0], "Cannot take from current squad markers; not in a squad.");
                                }
                            } else {
                                    ui.text_colored([1.0, 1.0, 0.0, 1.0], "Cannot take from current squad markers; not in a squad.");
                            }
                    } else {
                        ui.text_colored([1.0, 1.0, 0.0, 1.0], "RTAPI is required for taking squad marker locations.");
                    }
                    ui.dummy([4.0; 2]);
                    let table_flags = TableFlags::RESIZABLE | TableFlags::ROW_BG | TableFlags::BORDERS;
                    let table = ui.begin_table_header_with_flags("edit_markers", [
                        TableColumnSetup {
                            name: "Icon",
                            flags: TableColumnFlags::WIDTH_FIXED,
                            init_width_or_weight: 0.0,
                            user_id: Id::Str("marker_icon"),
                        },
                        TableColumnSetup {
                            name: "Description",
                            flags: TableColumnFlags::WIDTH_STRETCH,
                            init_width_or_weight: 0.0,
                            user_id: Id::Str("marker_desc"),
                        },
                        TableColumnSetup {
                            name: "Position (XYZ)",
                            flags: TableColumnFlags::WIDTH_STRETCH,
                            init_width_or_weight: 0.0,
                            user_id: Id::Str("marker_pos"),
                        },
                        TableColumnSetup {
                            name: "Controls",
                            flags: TableColumnFlags::WIDTH_STRETCH,
                            init_width_or_weight: 0.0,
                            user_id: Id::Str("marker_pos"),
                        }
                        ],
                        table_flags,
                    );
                    ui.table_next_column();
                    for (i, value) in MarkerType::iter_real_values().enumerate() {
                        let pushy = ui.push_id(Id::Str(&format!("{}", value)));
                        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
                        let alert_str = format!("cmdr{value}.png");
                        let alert_icon = Path::new(&alert_str);
                        let alert_icon = RelativePathBuf::from_path(alert_icon).expect("Can't make path relative");
                        let path = addon_dir.join("markers").join("icons");
                        let path = alert_icon.to_path(path);
                        RenderState::icon(ui, Some(32.0), Some(&alert_icon), Some(&path));
                        ui.table_next_column();
                        let label_size = ui.push_item_width(-1.0);
                        let label = format!("##Marker Description {value}");
                        let meep = ui.push_id(&label);
                        let description_input = ui.input_text(
                            &label,
                            &mut self.markers[i].description);
                        description_input
                            .hint("No description provided")
                            .build();
                        label_size.pop(ui);
                        meep.pop();
                        ui.table_next_column();
                        if let Some(position) = self.markers[i].position {
                            ui.text_wrapped(format!("({}, {}, {})", position.x, position.y, position.z));
                        } else {
                            ui.text_wrapped("No position provided.");
                        }
                        ui.table_next_column();
                            if ui.button("Get current position") {
                                if let Some(mid) = MarkerInputData::read() {
                                    log::info!("debug");

                                    self.markers[i].set_position(mid.local_player_pos);
                                }
                            }
                            if ui.button("Set manually") {
                            }
                        ui.table_next_column();
                        pushy.pop();
                    }
                    if let Some(token) = table {
                        token.end();
                    }
                    ui.dummy([4.0; 2]);
                    if ui.button("Save") {
                        return true
                    }
                    false
                });
            self.open = match closed {
                Some(true) => false,
                _ => open,
            };

        }
    }
}
