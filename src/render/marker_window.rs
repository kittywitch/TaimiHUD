use {
    super::RenderState,
    crate::{
        fl,
        marker::{
            atomic::MarkerInputData,
            format::{MarkerEntry, MarkerSet, MarkerType},
        },
        settings::RemoteState,
        timer::{PhaseState, TimerAlert, TimerFile},
        util::{PositionInput, UiExt},
        ControllerEvent, ACCOUNT_NAME_CELL, CONTROLLER_SENDER, SETTINGS,
    },
    glam::{Vec2, Vec3},
    nexus::{
        imgui::{
            Id, InputTextFlags, ProgressBar, StyleColor, TableColumnFlags, TableColumnSetup,
            TableFlags, Ui, Window,
        },
        paths::get_addon_dir,
        rtapi::{GroupType, RealTimeApi},
    },
    relative_path::{RelativePath, RelativePathBuf},
    std::{f32, path::Path, sync::Arc},
};

pub struct EditMarkerWindowState {
    pub open: bool,
    pub name: String,
    pub description: String,
    pub author: String,
    pub trigger: PositionInput,
    pub map_id: i32,
    pub markers: [IndividualMarkerState; 8],
}

pub struct IndividualMarkerState {
    pub position: PositionInput,
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
        self.position.position = Some(pos);
    }
    pub fn set_description(&mut self, desc: String) {
        self.description = desc;
    }
    pub fn to_marker_entry(&self, marker: MarkerType) -> Option<MarkerEntry> {
        let id = match self.description.is_empty() {
            true => None,
            false => Some(self.description.clone()),
        };
        Some(MarkerEntry {
            marker,
            id,
            position: self.position.position?.into(),
        })
    }
}

impl EditMarkerWindowState {
    pub fn new() -> Self {
        Self {
            open: false,
            name: Default::default(),
            trigger: Default::default(),
            description: Default::default(),
            map_id: Default::default(),
            author: Default::default(),
            markers: Default::default(),
        }
    }

    pub fn to_marker_set(&self) -> Option<MarkerSet> {
        let marker_types = MarkerType::iter_real_values();
        let enabled = true;
        let markers = marker_types
            .enumerate()
            .flat_map(|(i, k)| self.markers[i].to_marker_entry(k))
            .collect();
        Some(MarkerSet {
            enabled,
            markers,
            trigger: self.trigger.position?.into(),
            name: self.name.clone(),
            author: Some(self.author.clone()),
            map_id: self.map_id as u32, // thanks imgui types o.o
            description: self.description.clone(),
            path: None,
        })
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
                    }
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
            let closed = Window::new(&fl!("markers"))
                .size([300.0, 200.0], nexus::imgui::Condition::FirstUseEver)
                .opened(&mut open)
                .build(ui, || {
                    let name_name = fl!("name");
                    let name_input = ui.input_text(&name_name, &mut self.name);
                    name_input.build();
                    let author_name = fl!("author");
                    let author_input = ui.input_text(&author_name, &mut self.author);
                    author_input.build();
                    ui.dummy([4.0; 2]);
                    let map_id_name = fl!("map-id");
                    let map_id_input = ui.input_int(&map_id_name, &mut self.map_id);
                    map_id_input.build();
                    if ui.button(&fl!("set-map-id")) {
                        if let Some(mid) = MarkerInputData::read() {
                            self.map_id = mid.map_id as i32;
                        }
                    }
                    ui.dummy([4.0; 2]);
                    let description_name = fl!("description");
                    let description_input = ui.input_text_multiline(
                        &description_name,
                        &mut self.description,
                        [0.0, 0.0],
                    );
                    description_input.build();
                    self.trigger.draw_display(ui, true);
                    self.trigger.draw_take_current(ui);
                    self.trigger.draw_edit_manual(ui);
                    ui.dummy([4.0; 2]);
                    if let Some(rtapi) = RealTimeApi::get() {
                        if let Some(group) = rtapi.read_group() {
                            let is_squad = matches!(
                                group.group_type,
                                Ok(GroupType::Squad | GroupType::RaidSquad)
                            );
                            if is_squad {
                                if ui.button(&fl!("take-squad-markers")) {
                                    for (i, marker) in group.squad_markers.iter().enumerate() {
                                        if *marker != [f32::INFINITY; 3] {
                                            self.markers[i].set_position(Vec3::from_array(*marker));
                                        }
                                    }
                                }
                            } else {
                                ui.text_colored(
                                    [1.0, 1.0, 0.0, 1.0],
                                    &fl!("cannot-take-squad-markers"),
                                );
                            }
                        } else {
                            ui.text_colored(
                                [1.0, 1.0, 0.0, 1.0],
                                &fl!("cannot-take-squad-markers"),
                            );
                        }
                    } else {
                        ui.text_colored(
                            [1.0, 1.0, 0.0, 1.0],
                            &fl!("rt-api-required-squad-markers"),
                        );
                    }
                    ui.dummy([4.0; 2]);
                    let table_flags =
                        TableFlags::RESIZABLE | TableFlags::ROW_BG | TableFlags::BORDERS;
                    let table = ui.begin_table_header_with_flags(
                        "edit_markers",
                        [
                            TableColumnSetup {
                                name: &fl!("icon"),
                                flags: TableColumnFlags::WIDTH_FIXED,
                                init_width_or_weight: 0.0,
                                user_id: Id::Str("marker_icon"),
                            },
                            TableColumnSetup {
                                name: &fl!("description"),
                                flags: TableColumnFlags::WIDTH_STRETCH,
                                init_width_or_weight: 0.0,
                                user_id: Id::Str("marker_desc"),
                            },
                            TableColumnSetup {
                                name: &fl!("local-header"),
                                flags: TableColumnFlags::WIDTH_STRETCH,
                                init_width_or_weight: 0.0,
                                user_id: Id::Str("marker_pos"),
                            },
                            TableColumnSetup {
                                name: &fl!("controls"),
                                flags: TableColumnFlags::WIDTH_STRETCH,
                                init_width_or_weight: 0.0,
                                user_id: Id::Str("marker_pos"),
                            },
                        ],
                        table_flags,
                    );
                    ui.table_next_column();
                    for (i, value) in MarkerType::iter_real_values().enumerate() {
                        let pushy = ui.push_id(Id::Str(&format!("{}", value)));
                        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
                        let alert_str = format!("cmdr{value}.png");
                        let alert_icon = Path::new(&alert_str);
                        let alert_icon = RelativePathBuf::from_path(alert_icon)
                            .expect("Can't make path relative");
                        let path = addon_dir.join("markers").join("icons");
                        let path = alert_icon.to_path(path);
                        RenderState::icon(ui, Some(32.0), Some(&alert_icon), Some(&path));
                        ui.table_next_column();
                        let label_size = ui.push_item_width(-1.0);
                        let label = format!("##Marker Description {value}");
                        let meep = ui.push_id(&label);
                        let description_input =
                            ui.input_text(&label, &mut self.markers[i].description);
                        description_input.hint(&fl!("no-description")).build();
                        label_size.pop(ui);
                        meep.pop();
                        ui.table_next_column();
                        self.markers[i].position.draw_display(ui, false);
                        ui.table_next_column();
                        self.markers[i].position.draw_take_current(ui);
                        self.markers[i].position.draw_edit_manual(ui);
                        ui.table_next_column();
                        pushy.pop();
                    }
                    if let Some(token) = table {
                        token.end();
                    }
                    ui.dummy([4.0; 2]);
                    if ui.button(&fl!("save")) {
                        return true;
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
