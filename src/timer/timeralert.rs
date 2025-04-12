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
    Both,
}

impl DeserializeAlert {
    pub fn kind(&self) -> TimerAlertType {
        use TimerAlertType::*;
        match (&self.warning, &self.alert) {
            (Some(_warn), Some(_alrt)) => Both,
            (Some(_warn), None) => Warning,
            (None, Some(_alrt)) => Alert,
            (None, None) => {
                panic!("A timer alert that has neither an alert or a warning was defined!")
            }
        }
    }

    pub fn get_alerts(&self) -> Vec<TimerAlert> {
        let kind = self.kind();
        use TimerAlertType::*;
        if kind != Both {
            let (text, colour, duration) = match kind {
                Warning => (
                    self.warning.as_ref().unwrap(),
                    self.warning_color,
                    self.warning_duration.unwrap(),
                ),
                Alert => (
                    self.alert.as_ref().unwrap(),
                    self.alert_color,
                    self.alert_duration.unwrap(),
                ),
                Both => (
                    &Default::default(),
                    Default::default(),
                    Default::default(),
                ),
            };
            self.timestamps
                .iter()
                .map(|&timestamp| TimerAlert {
                    kind,
                    text: text.clone(),
                    colour,
                    duration,
                    fill_colour: self.fill_color,
                    timestamp,
                    icon: self.icon.clone(),
                })
                .collect()
        } else {
            self.timestamps
                .iter()
                .flat_map(|&timestamp| {
            let alert = TimerAlert {
                    kind: Alert,
                    text: self.alert.as_ref().unwrap().clone(),
                    colour: self.alert_color,
                    duration: self.alert_duration.unwrap(),
                    fill_colour: self.fill_color,
                    timestamp,
                    icon: self.icon.clone(),
                };
            let warning = TimerAlert {
                    kind: Warning,
                    text: self.warning.as_ref().unwrap().clone(),
                    colour: self.warning_color,
                    duration: self.warning_duration.unwrap(),
                    fill_colour: self.fill_color,
                    timestamp,
                    icon: self.icon.clone(),
                };
                vec![alert, warning]
            }).collect()
        }
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
