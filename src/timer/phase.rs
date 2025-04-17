use {
    crate::timer::{
        BlishAlert, TimerAction, TimerAlert, TimerTrigger
    }, relative_path::RelativePathBuf, serde::{Deserialize, Serialize}, serde_json::Value, std::path::PathBuf
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerPhase {
    pub name: String,
    pub start: TimerTrigger,
    #[serde(default)]
    pub finish: Option<TimerTrigger>,
    #[serde(default)]
    pub alerts: Vec<BlishAlert>,
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
            .flat_map(BlishAlert::get_alerts)
            .collect()
    }
}
