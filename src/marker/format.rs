use super::atomic::{CurrentPerspective, MinimapPlacement};

use {
    crate::timer::{BlishVec3, Polytope, Position}, chrono::{DateTime, Utc}, glam::{Mat4, Vec3}, nexus::gamebind::GameBind, serde::{Deserialize, Serialize}, std::{path::PathBuf, sync::Arc}
};
use glam::{Vec2, Vec2Swizzles, Vec3Swizzles};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum_macros::Display;
use tokio::fs::{create_dir_all, read_to_string, File};
use tokio::io::AsyncWriteExt;


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkerFile {
    pub last_edit: DateTime<Utc>,
    pub path: Option<PathBuf>,
    pub categories: Vec<MarkerCategory>,
}

impl MarkerFile {
    pub async fn load(path: &PathBuf) -> anyhow::Result<Arc<Self>> {
        log::debug!("Attempting to load the markers file at \"{path:?}\".");
        let mut file_data = read_to_string(path).await?;
        json_strip_comments::strip(&mut file_data)?;
        let mut data: Self = serde_json::from_str(&file_data)?;
        data.path = Some(path.to_path_buf());
        log::debug!("Successfully loaded the markers file at \"{path:?}\".");
        Ok(Arc::new(data))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkerCategory {
    #[serde(alias="categoryName")]
    pub name: String,
    pub marker_sets: Vec<MarkerSet>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkerSet {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub author: String,
    pub name: String,
    pub description: String,
    pub map_id: u32,
    pub trigger: MarkerPosition,
    pub markers: Vec<MarkerEntry>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarkerEntry {
    #[serde(alias="i")]
    pub marker: MarkerType,
    #[serde(alias="d")]
    pub id: Option<String>,
    #[serde(flatten)]
    pub position: MarkerPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarkerPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
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



#[derive(Serialize_repr, Deserialize_repr, Display, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum MarkerType {
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
    pub fn to_place_world_gamebind(&self) -> GameBind {
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
    pub fn to_set_agent_gamebind(&self) -> GameBind {
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
