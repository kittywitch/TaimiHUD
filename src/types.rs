use glam::f32::Vec3;
use palette::rgb::Rgb;
use palette::convert::{FromColorUnclamped, IntoColorUnclamped};
use palette::{Srgba};
use serde::{Deserialize, Serialize};

use crate::xnacolour::XNAColour;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
enum TimerTriggerType {
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
struct TimerTrigger {
    #[serde(rename = "type", default)]
    kind: TimerTriggerType,
    key_bind: Option<String>,
    position: Option<Vec3>,
    antipode: Option<Vec3>,
    radius: Option<f32>,
    #[serde(default)]
    require_combat: bool,
    #[serde(default)]
    require_out_of_combat: bool,
    #[serde(default)]
    require_entry: bool,
    #[serde(default)]
    require_departure: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct TimerFile {
    id: String,
    name: String,
    category: String,
    description: String,
    author: String,
    icon: String,
    map: u32,
    reset: TimerTrigger,
    phases: Vec<TimerPhase>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct TimerPhase {
    name: String,
    start: TimerTrigger,
    alerts: Vec<TimerAlert>,
    #[serde(default)]
    actions: Vec<TimerAction>,
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
enum TimerActionType {
    SkipTime,
}

impl Default for TimerActionType {
    fn default() -> Self {
        Self::SkipTime
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct TimerAction {
    name: String,
    #[serde(rename = "type", default)]
    kind: TimerActionType,
    sets: Option<Vec<String>>,
    trigger: TimerTrigger,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct TimerAlert {
    warning_duration: f32,
    alert_duration: f32,
    warning: String,
    warning_color: XNAColour,
    alert: String,
    alert_color: XNAColour,
    icon: String,
    fill_color: XNAColour,
}

