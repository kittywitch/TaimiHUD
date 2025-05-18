use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
pub struct TimerSettings {
    #[serde(default)]
    pub disabled: bool,
}

impl TimerSettings {
    pub fn disable(&mut self) {
        self.disabled = true;
    }
    pub fn enable(&mut self) {
        self.disabled = false;
    }
    pub fn toggle(&mut self) -> bool {
        self.disabled = !self.disabled;
        self.disabled
    }
}
