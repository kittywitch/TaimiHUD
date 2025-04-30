use {
    crate::timer::BlishVec3,
    serde::{Deserialize, Serialize}, std::path::PathBuf,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerMarker {
    pub position: BlishVec3,
    pub size: Option<f32>,
    #[serde(default)]
    pub fade_center: bool,
    pub opacity: Option<f32>,
    pub texture: PathBuf,
    pub duration: f32,
    pub timestamps: Option<Vec<f32>>,
}
