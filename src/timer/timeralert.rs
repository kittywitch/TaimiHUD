
use {
    crate::xnacolour::XNAColour,
    serde::{Deserialize, Serialize},
    tokio::time::Duration,
    strum_macros::Display,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeserializeAlert {
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

#[derive(Serialize, Deserialize, Debug, Clone, Display, Copy)]
pub enum TimerAlertType {
    Alert,
    Warning
}

impl DeserializeAlert {
    pub fn kind(&self) -> TimerAlertType {
        use TimerAlertType::*;
        match (&self.warning, &self.alert) {
            (Some(_warn), Some(_alrt)) => panic!("A timer alert that is both an alert and a warning was defined!"),
            (Some(_warn), None) => Warning,
            (None, Some(_alrt)) => Alert,
(None, None) => panic!("A timer alert that has neither an alert or a warning was defined!"),
        }
    }

    pub fn get_alerts(&self) -> Vec<TimerAlert> {
        let kind = self.kind();
        use TimerAlertType::*;
        let (text, colour, duration) = match kind {
            Warning => (self.warning.as_ref().unwrap(), self.warning_color, self.warning_duration.unwrap()),
            Alert => (self.alert.as_ref().unwrap(), self.alert_color, self.alert_duration.unwrap()),
        };
        self.timestamps.iter().map(|&timestamp| {
            TimerAlert {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimerAlert {
    pub kind: TimerAlertType,
    pub text: String,
    pub colour: Option<XNAColour>,
    pub fill_colour: Option<XNAColour>,
    pub icon: Option<String>,
    pub timestamp: f32,
    pub duration: f32,
}


impl TimerAlert {
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
