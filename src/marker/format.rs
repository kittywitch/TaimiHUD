use {
    crate::timer::{BlishVec3, Polytope, Position}, chrono::{DateTime, Utc}, glam::Vec3, nexus::gamebind::GameBind, serde::{Deserialize, Serialize}
};


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct MarkerFile {
    last_edit: DateTime<Utc>,
    categories: Vec<MarkerCategory>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct MarkerCategory {
    #[serde(alias="categoryName")]
    name: String,
    markers: Vec<MarkerSet>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct MarkerSet {
    author: String,
    name: String,
    description: String,
    map_id: u32,
    trigger: BlishVec3,
    markers: Vec<MarkerPosition>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MarkerEntry {
    #[serde(alias="i")]
    marker: MarkerType,
    #[serde(alias="d")]
    id: String,
    #[serde(flatten)]
    positiion: MarkerPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MarkerPosition {
    x: f32,
    y: f32,
    z: f32,
}

impl From<MarkerPosition> for Vec3 {
    // it's pre-swizzled
    fn from(local: MarkerPosition) -> Self {
        Self::new(local.x, local.z, local.y)
    }
}

impl From<MarkerPosition> for BlishVec3 {
    fn from(local: MarkerPosition) -> Self {
        Self::from_vec3(local.into())
    }
}

impl From<MarkerPosition> for Position {
    fn from(local: MarkerPosition) -> Self {
        let local_vec3: Vec3 = local.into();
        Self::from(local_vec3)
    }
}

impl From<MarkerPosition> for Polytope {
    fn from(local: MarkerPosition) -> Self {
        Polytope::NSphere { center: local.into(), radius: 15.0 }
    }
}

enum MinimapPlacement {
    Top,
    Bottom,
}

enum UiSize {
    Small = 0,
    Normal = 1,
    Large = 2,
    Larger = 3,
}

impl From<UiSize> for f32 {
    fn from(local: UiSize) -> Self {
        match local {
            UiSize::Small => 0.81,
            UiSize::Normal => 0.897,
            UiSize::Large => 1.0,
            UiSize::Larger => 1.103,
        }
    }
}

struct MinimapState {
    width: u16,
    height: u16,
    rotation: f32,
    map_scale: f32,
    map_center_x: f32,
    map_center_y: f32,
    placement: MinimapPlacement,
}

impl MinimapState {
    fn boundary(&self, window_size: &[f32; 2], ui_scale: UiSize) {
        let ui_scaler: f32 = ui_scale.into();
        let vertical_offset = match &self.placement {
            MinimapPlacement::Top => 0,
            MinimapPlacement::Bottom => 40,
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum MarkerType {
    // Schema reference: https://github.com/manlaan/BlishHud-CommanderMarkers/blob/bhud-static/Manlaan.CommanderMarkers/README.md?plain=1#L69-L78
    // According to the schema, 0 and 9 are both Clear Markers.
    // Code reference: https://github.com/manlaan/BlishHud-CommanderMarkers/blob/7c3b2081596f7b8746e5e57d65213711aafa938c/Library/Enums/SquadMarker.cs#L6-L28
    // According to their code, 0 is None and 9 is Clear Markers.
    // This is why I have trust issues, man.
    Blank = 0,
    Arrow = 1,
    Circle = 2,
    Heart = 3,
    Square = 4,
    Star = 5,
    Spiral = 6,
    Triangle = 7,
    Cross = 8,
    ClearMarkers = 9,
}

impl MarkerType {
    fn to_place_world_gamebind(&self) -> GameBind {
        match self {
            Self::Blank => panic!("i can't believe you've done this"),
            Self::Arrow => GameBind::SquadMarkerPlaceWorldArrow,
            Self::Circle => GameBind::SquadMarkerPlaceWorldCircle,
            Self::Heart => GameBind::SquadMarkerPlaceWorldHeart,
            Self::Square => GameBind::SquadMarkerPlaceWorldSquare,
            Self::Star => GameBind::SquadMarkerPlaceWorldStar,
            // what in the fuck my dudes
            Self::Spiral => GameBind::SquadMarkerPlaceWorldSwirl,
            Self::Triangle => GameBind::SquadMarkerPlaceWorldTriangle,
            Self::Cross => GameBind::SquadMarkerPlaceWorldCross,
            Self::ClearMarkers => GameBind::SquadMarkerClearAllWorld,
        }
    }
    fn to_set_agent_gamebind(&self) -> GameBind {
        match self {
            Self::Blank => panic!("i can't believe you've done this"),
            Self::Arrow => GameBind::SquadMarkerSetAgentArrow,
            Self::Circle => GameBind::SquadMarkerSetAgentCircle,
            Self::Heart => GameBind::SquadMarkerSetAgentHeart,
            Self::Square => GameBind::SquadMarkerSetAgentSquare,
            Self::Star => GameBind::SquadMarkerSetAgentStar,
            // what in the fuck my dudes
            Self::Spiral => GameBind::SquadMarkerSetAgentSwirl,
            Self::Triangle => GameBind::SquadMarkerSetAgentTriangle,
            Self::Cross => GameBind::SquadMarkerSetAgentCross,
            Self::ClearMarkers => GameBind::SquadMarkerClearAllWorld,
        }
    }
}
