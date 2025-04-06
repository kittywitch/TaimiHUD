use {
    crate::{
        geometry::{BlishVec3, DeserializePosition, Polytope, Position},
        xnacolour::XNAColour,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TimerTriggerType {
    Location,
    Key,
}

impl Default for TimerTriggerType {
    fn default() -> Self {
        Self::Location
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerTrigger {
    #[serde(rename = "type", default)]
    pub kind: TimerTriggerType,
    pub key_bind: Option<String>,
    pub position: Option<DeserializePosition>,
    pub antipode: Option<DeserializePosition>,
    pub radius: Option<f32>,
    #[serde(default)]
    pub require_combat: bool,
    #[serde(default)]
    pub require_out_of_combat: bool,
    #[serde(default)]
    pub require_entry: bool,
    #[serde(default)]
    pub require_departure: bool,
}

impl TimerTrigger {
    pub fn position(&self) -> Option<Position> {
        self.position.map(Into::into)
    }
    pub fn antipode(&self) -> Option<Position> {
        self.antipode.map(Into::into)
    }
    pub fn polytope(&self) -> Option<Polytope> {
        match self {
            &Self {
                radius: Some(radius),
                position: Some(center),
                ..
            } => Some(Polytope::NSphere {
                radius,
                center: center.into(),
            }),
            &Self {
                antipode: Some(antipode),
                position: Some(pode),
                ..
            } => Some(Polytope::NCuboid {
                antipode: antipode.into(),
                pode: pode.into(),
            }),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerFile {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub icon: String,
    // I probably don't need to do this, but it's convenient :o
    #[serde(rename = "map")]
    pub map_id: u32,
    pub reset: TimerTrigger,
    pub phases: Vec<TimerPhase>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerPhase {
    pub name: String,
    pub start: TimerTrigger,
    #[serde(default)]
    pub finish: Option<TimerTrigger>,
    #[serde(default)]
    pub alerts: Vec<TimerAlert>,
    #[serde(default)]
    pub actions: Vec<TimerAction>,
    /*
     * Not yet implemented:
     * - directions
     * - markers
     * - sounds
     */
    #[serde(skip, default)]
    directions: Vec<String>,
    #[serde(skip, default)]
    markers: Vec<TimerMarker>,
    #[serde(skip, default)]
    sounds: Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerMarker {
    pub position: BlishVec3,
    pub size: Option<f32>,
    #[serde(default)]
    pub fade_center: bool,
    pub opacity: Option<f32>,
    pub texture: String,
    pub duration: f32,
    pub timestamps: Option<Vec<f32>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum TimerActionType {
    SkipTime,
}

impl Default for TimerActionType {
    fn default() -> Self {
        Self::SkipTime
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerAction {
    pub name: String,
    #[serde(rename = "type", default)]
    pub kind: TimerActionType,
    pub sets: Option<Vec<String>>,
    pub trigger: TimerTrigger,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerAlert {
    pub warning_duration: Option<f32>,
    pub alert_duration: Option<f32>,
    pub warning: Option<String>,
    pub warning_color: Option<XNAColour>,
    pub alert: Option<String>,
    pub alert_color: Option<XNAColour>,
    pub icon: Option<String>,
    pub fill_color: Option<XNAColour>,
    pub timestamps: Option<Vec<f32>>,
}
