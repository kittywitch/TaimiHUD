use glam::f32::Vec3;
use palette::rgb::Rgb;
use palette::convert::{FromColorUnclamped, IntoColorUnclamped};
use palette::{Srgba};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::xnacolour::XNAColour;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
struct BlishVec3 {
    child: Vec3
}

impl BlishVec3 {
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.child.x, self.child.z, self.child.y)
    }

    pub fn from_vec3(vec3: Vec3) -> Self {
        BlishVec3 {
            child: Vec3::new(vec3.x, vec3.z, vec3.y),
        }
    }

    pub fn from_raw_vec3(vec3: Vec3) -> Self {
        BlishVec3 {
            child: vec3,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub position: Option<BlishVec3>,
    pub antipode: Option<BlishVec3>,
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
    pub fn position(&self) -> Option<Vec3> {
        self.position.as_ref().map(BlishVec3::to_vec3)
    }

    pub fn antipode(&self) -> Option<Vec3> {
        self.antipode.as_ref().map(BlishVec3::to_vec3)
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
    pub finish: TimerTrigger,
    pub alerts: Vec<TimerAlert>,
    #[serde(default)]
    pub actions: Vec<TimerAction>,
    /*
    * Not yet implemented:
    * - directions
    * - markers
    * - sounds
    */
    #[serde(skip_serializing,default)]
    directions: String,
    #[serde(skip_serializing,default)]
    markers: String,
    #[serde(skip_serializing,default)]
    sounds: String,
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
}

