use {
    crate::xnacolour::XNAColour,
    nexus::imgui::{ProgressBar, StyleColor, Ui},
    serde::{Deserialize, Serialize},
    strum_macros::Display,
    tokio::time::{Duration, Instant},
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
            .flat_map(|&timestamp| {
                self.alert(timestamp)
                    .into_iter()
                    .chain(self.warning(timestamp))
            })
            .collect()
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
    pub fn end(&self, start: Instant) -> Instant {
        self.start(start) + self.duration()
    }
    pub fn start(&self, start: Instant) -> Instant {
        start + self.timestamp()
    }
    pub fn percentage(&self, start: Instant) -> Option<f32> {
        let elapsed = Instant::now()
            .checked_duration_since(self.start(start))?
            .as_secs_f32();
        if elapsed > self.duration {
            None
        } else {
            Some(elapsed / self.duration)
        }
    }
    pub fn remaining(&self, start: Instant) -> Duration {
        self.end(start).saturating_duration_since(Instant::now())
    }
    pub fn progress_bar_text(&self, start: Instant) -> String {
        format!(
            "{} - in {:.1}s",
            self.text,
            self.remaining(start).as_secs_f32()
        )
    }
    pub fn progress_bar(&self, ui: &Ui, start: Instant) {
        if let Some(percent) = self.percentage(start) {
            let mut colour_tokens = Vec::new();
            if let Some(fill_colour) = self.fill_colour {
                colour_tokens
                    .push(ui.push_style_color(StyleColor::PlotHistogram, fill_colour.imgcolor()));
            }
            if let Some(colour) = self.colour {
                colour_tokens.push(ui.push_style_color(StyleColor::Text, colour.imgcolor()));
            }
            ProgressBar::new(percent)
                .size([-1.0, 12.0])
                .overlay_text(self.progress_bar_text(start))
                .build(ui);
            for token in colour_tokens {
                token.pop();
            }
        }
    }
}
