use {
    crate::{
        timer::{
            TimerAlert,
            TimerFile,
            PhaseState,
        },
    },
    nexus::{
        imgui::{
            ProgressBar,
            StyleColor,
            Ui,
            Window,
        },
    },
    std::sync::Arc,
    tokio::time::Instant,
};

pub struct TimerWindowState {
    pub open: bool,
    phase_states: Vec<PhaseState>,
}

impl TimerWindowState {
    pub fn new() -> Self {
        Self {
            open: true,
            phase_states: Default::default(),
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        if self.open {
            Window::new("Timers").opened(&mut self.open).build(ui, || {
                for ps in &self.phase_states {
                    for alert in ps.alerts.iter() {
                        Self::progress_bar(alert, ui, ps.start)
                    }
                }
            });
        }
    }

    fn progress_bar(alert: &TimerAlert, ui: &Ui, start: Instant) {
        if let Some(percent) = alert.percentage(start) {
            let mut colour_tokens = Vec::new();
            if let Some(fill_colour) = alert.fill_colour {
                colour_tokens
                    .push(ui.push_style_color(StyleColor::PlotHistogram, fill_colour.imgcolor()));
            }
            if let Some(colour) = alert.colour {
                colour_tokens.push(ui.push_style_color(StyleColor::Text, colour.imgcolor()));
            }
            ProgressBar::new(percent)
                .size([-1.0, 12.0])
                .overlay_text(alert.progress_bar_text(start))
                .build(ui);
            for token in colour_tokens {
                token.pop();
            }
        }
    }

    pub fn new_phase(&mut self, phase_state: PhaseState) {
        self.phase_states.push(phase_state);
    }
    pub fn remove_phase(&mut self, timer: Arc<TimerFile>) {
        self.phase_states.retain(|p| !Arc::ptr_eq(&p.timer, &timer));
    }
    pub fn reset_phases(&mut self) {
        self.phase_states.clear();
    }
}


