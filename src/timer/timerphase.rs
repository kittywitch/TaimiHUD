use {
    serde_json::Value,
    crate::{
        timer::{
            timertrigger::TimerTrigger,
            timeraction::TimerAction,
            timeralert::{DeserializeAlert, TimerAlert},
        },
    },
    serde::{Serialize, Deserialize},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerPhase {
    pub name: String,
    pub start: TimerTrigger,
    #[serde(default)]
    pub finish: Option<TimerTrigger>,
    #[serde(default)]
    pub alerts: Vec<DeserializeAlert>,
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
    pub fn get_alerts(&self) -> Vec<TimerAlert> {
        self.alerts.iter().flat_map(DeserializeAlert::get_alerts).collect()
    }
}


