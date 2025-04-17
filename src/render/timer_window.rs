use {
    crate::{
        timer::{
            PhaseState, TimerAlert, TimerFile
        }, ControllerEvent, SETTINGS, TS_SENDER
    }, glam::Vec2, nexus::imgui::{
        ProgressBar,
        StyleColor,
        Ui,
        Window,
    }, std::sync::Arc, tokio::time::Instant
};

pub struct TimerWindowState {
    pub open: bool,
    pub stock_progress_bar: bool,
    pub phase_states: Vec<PhaseState>,
}

impl TimerWindowState {
    pub fn new() -> Self {
        Self {
            open: false,
            stock_progress_bar: false,
            phase_states: Default::default(),
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        let mut open = self.open;
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            open = settings.timers_window_open;
            self.stock_progress_bar = settings.stock_progress_bar;
        };
        if open {
            Window::new("Timers").opened(&mut open).build(ui, || {
                for ps in &self.phase_states {
                    for alert in ps.alerts.iter() {
                        if self.stock_progress_bar {
                                Self::stock_progress_bar(alert, ui, ps.start);
                        } else {
                                Self::progress_bar(alert, ui, ps.start);
                        }
                    }
                }
            });
        }

        if open != self.open {
            let sender = TS_SENDER.get().unwrap();
            let event_send = sender.try_send(ControllerEvent::WindowState("timers".to_string(), open));
            drop(event_send);
            self.open = open;
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
            let original_position = Vec2::from_array(ui.cursor_pos());
            let height = 24.0;
            ProgressBar::new(percent)
                .size([-1.0, height])
                .overlay_text("")
                .build(ui);
            let window_size = Vec2::from(ui.window_size());
            let window_width = window_size.x;
            for token in colour_tokens {
                token.pop();
            }
            let text = alert.progress_bar_text(start);
            let text_size = Vec2::from_array(ui.calc_text_size(&text));
            let centre_x = original_position.x + (window_width / 2.0);
            let centre_y = original_position.y + (height / 2.0);
            let new_cursor_pos_x = centre_x - (text_size.x / 2.0);
            let new_cursor_pos_y = centre_y - (text_size.y / 2.0);
            let new_cursor_pos = Vec2 {
                x: new_cursor_pos_x,
                y: new_cursor_pos_y,
            };
            ui.set_cursor_pos(new_cursor_pos.into());
            ui.text(text);
            ui.dummy([0.0,height/4.0]);
        }

    }

    fn stock_progress_bar(alert: &TimerAlert, ui: &Ui, start: Instant) {
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


