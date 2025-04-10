use {
    crate::{
        geometry::{BlishVec3, DeserializePosition, Polytope, Position},
        xnacolour::XNAColour,
    },
    serde::{Deserialize, Serialize},
    serde_json::Value,
    strum_macros::Display,
    tokio::time::Duration,
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

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum CombatState {
    Outside,
    Entered,
    Exited,
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
    pub fn check(&self, pos: Position, cb: CombatState) -> bool {
        let shape = match self.polytope() {
            Some(s) => s,
            None => return false,
        };
        let position_check = shape.point_is_within(pos);
        let combat_entered_check = !self.require_combat || cb == CombatState::Entered;
        let combat_exited_check = !self.require_out_of_combat || cb == CombatState::Exited;
        let combat_check = combat_entered_check && combat_exited_check;
        let entry_check = !self.require_entry || position_check;
        let departure_check = !self.require_departure || !position_check;
        entry_check && departure_check && combat_check
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

impl TimerFile {
    pub fn name(&self) -> String {
        self.name.replace("\n", " ")
    }
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
    directions: Value,
    #[serde(skip, default)]
    markers: Value,
    #[serde(skip, default)]
    sounds: Value,
}

impl TimerPhase {
    pub fn get_alerts(&self) -> Vec<TaimiAlert> {
        self.alerts.iter().flat_map(TimerAlert::get_alerts).collect()
    }
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
    #[serde(default)]
    pub warning_duration: Option<f32>,
    #[serde(default)]
    pub alert_duration: Option<f32>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub warning_color: Option<XNAColour>,
    #[serde(default)]
    pub alert: Option<String>,
    #[serde(default)]
    pub alert_color: Option<XNAColour>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub fill_color: Option<XNAColour>,
    #[serde(default)]
    pub timestamps: Vec<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaimiAlert {
    pub kind: TimerAlertType,
    pub text: String,
    pub colour: Option<XNAColour>,
    pub fill_colour: Option<XNAColour>,
    pub icon: Option<String>,
    pub timestamp: f32,
    pub duration: f32,
}

impl TimerAlert {
    pub fn kind(&self) -> TimerAlertType {
        use TimerAlertType::*;
        match (&self.warning, &self.alert) {
            (Some(_warn), Some(_alrt)) => panic!("A timer alert that is both an alert and a warning was defined!"),
            (Some(_warn), None) => Warning,
            (None, Some(_alrt)) => Alert,
(None, None) => panic!("A timer alert that has neither an alert or a warning was defined!"),
        }
    }

    pub fn get_alerts(&self) -> Vec<TaimiAlert> {
        let kind = self.kind();
        use TimerAlertType::*;
        let (text, colour, duration) = match kind {
            Warning => (self.warning.as_ref().unwrap(), self.warning_color, self.warning_duration.unwrap()),
            Alert => (self.alert.as_ref().unwrap(), self.alert_color, self.alert_duration.unwrap()),
        };
        self.timestamps.iter().map(|&timestamp| {
            TaimiAlert {
                kind,
                text: text.clone(),
                colour,
                duration,
                fill_colour: self.fill_color,
                timestamp,
                icon: self.icon.clone(),
            }
        }).collect()
    }

}

#[derive(Serialize, Deserialize, Debug, Clone, Display, Copy)]
pub enum TimerAlertType {
    Alert,
    Warning
}

impl TaimiAlert {
    pub fn descriptor(&self, ts: f32) -> String {
        return format!("\"{}\"${}@{}[{}]", self.text, self.kind, ts, self.duration)
    }
    pub fn raw_timestamp(&self) -> Duration {
        Duration::from_secs_f32(self.timestamp)
    }
    pub fn timestamp(&self) -> Duration {
        self.raw_timestamp().checked_sub(self.duration()).unwrap_or_default()
    }
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.duration)
    }
}
