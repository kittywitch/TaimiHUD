use {
    crate::xnacolour::XNAColour,
    serde::{Deserialize, Serialize},
    strum_macros::Display,
    tokio::time::{
        Duration,
        Instant,
    },
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

#[derive(Serialize, Deserialize, Debug, Clone, Display, Copy, PartialEq)]
pub enum TimerAlertType {
    Alert,
    Warning,
}

impl DeserializeAlert {
    pub fn alert(&self, timestamp: f32) -> Option<TimerAlert> {
        Some(TimerAlert {
                kind: TimerAlertType::Alert,
                text: self.alert.clone()?,
                colour: self.alert_color,
                duration: self.alert_duration?,
                fill_colour: self.fill_color,
                timestamp,
                icon: self.icon.clone(),
        })
    }

    pub fn warning(&self, timestamp: f32) -> Option<TimerAlert> {
        Some(TimerAlert {
                kind: TimerAlertType::Warning,
                text: self.warning.clone()?,
                colour: self.warning_color,
                duration: self.warning_duration?,
                fill_colour: self.fill_color,
                timestamp,
                icon: self.icon.clone(),
        })
    }

    pub fn get_alerts(&self) -> Vec<TimerAlert> {
            self.timestamps
                .iter()
                .flat_map(|&timestamp| self.alert(timestamp)
                        .into_iter()
                        .chain(self.warning(timestamp))
                ).collect()
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
        self.raw_timestamp()
            .checked_sub(self.duration())
            .unwrap_or_default()
    }
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f32(self.duration)
    }
    pub fn end_time(&self, now: Instant) -> Instant {
               now + self.duration() 
    }
}
