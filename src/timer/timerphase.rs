use {
    crate::timer::{
        timeraction::TimerAction,
        timeralert::{DeserializeAlert, TimerAlert},
        timertrigger::TimerTrigger,
    },
    serde::{Deserialize, Serialize},
    serde_json::Value,
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
    #[allow(dead_code)]
    directions: Value,
    #[serde(skip, default)]
    #[allow(dead_code)]
    markers: Value,
    #[serde(skip, default)]
    #[allow(dead_code)]
    sounds: Value,
}

impl TimerPhase {
    pub fn get_alerts(&self) -> Vec<TimerAlert> {
        self.alerts
            .iter()
            .flat_map(DeserializeAlert::get_alerts)
            .collect()
    }
}
