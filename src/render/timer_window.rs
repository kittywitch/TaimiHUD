use {
    super::RenderState, crate::{
        timer::{
            PhaseState, TimerAlert, TimerFile
        }, ControllerEvent, SETTINGS, TS_SENDER
    }, glam::Vec2, nexus::{imgui::{
        Image, ProgressBar, StyleColor, Ui, Window
    }, texture::get_texture}, std::sync::Arc
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
                                Self::stock_progress_bar(alert, ui, ps);
                        } else {
                                Self::progress_bar(alert, ui, ps);
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

    fn progress_bar(alert: &TimerAlert, ui: &Ui, ps: &PhaseState) {
        let start = ps.start;
        let height = 24.0;
        if let Some(percent) = alert.percentage(start) {
            if let Some(icon) = &alert.icon {
                    if let Some(path) = &ps.timer.path {
                        if let Some(icon) = get_texture(icon.as_str()) {
                        Image::new(icon.id(),[height,height]).build(ui);
                        ui.same_line();
                    } else {
                        let sender = TS_SENDER.get().unwrap();
                        let event_send = sender.try_send(ControllerEvent::LoadTexture(icon.clone(), path.to_path_buf()));
                        drop(event_send);
                    }
                }
            };
            let mut colour_tokens = Vec::new();
            if let Some(fill_colour) = alert.fill_colour {
                colour_tokens
                    .push(ui.push_style_color(StyleColor::PlotHistogram, fill_colour.imgcolor()));
            }
            if let Some(colour) = alert.colour {
                colour_tokens.push(ui.push_style_color(StyleColor::Text, colour.imgcolor()));
            }
            let original_position = Vec2::from_array(ui.cursor_pos());
            ProgressBar::new(percent)
                .size([-1.0, height])
                .overlay_text("")
                .build(ui);
            let window_size = Vec2::from(ui.window_size());
            for token in colour_tokens {
                token.pop();
            }
            let text = alert.progress_bar_text(start);
            let widget_centre = window_size.with_y(height) / 2.0;
            let centre = original_position + widget_centre;
            RenderState::offset_font_text("ui", ui, centre, &text);
            ui.dummy([0.0,height/4.0]);
        }

    }

    fn stock_progress_bar(alert: &TimerAlert, ui: &Ui, ps: &PhaseState) {
        let start = ps.start;
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


