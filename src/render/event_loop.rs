use {
    crate::{
        render::{
            PrimaryWindowState,
            TimerWindowState,
        },
        timer::TimerFile,
        timermachine::{PhaseState, TextAlert},
        RENDER_STATE,
    },
    nexus::{
        data_link::read_nexus_link,
        imgui::{
            internal::RawCast, Condition, Font, FontId, Io,
            Ui, Window, WindowFlags,
        },
    },
    std::sync::{Arc, MutexGuard},
    tokio::sync::mpsc::Receiver,
};

pub enum RenderThreadEvent {
    TimerData(Vec<Arc<TimerFile>>),
    AlertFeed(PhaseState),
    AlertReset(Arc<TimerFile>),
    AlertStart(TextAlert),
    AlertEnd(Arc<TimerFile>),
    DataSourceUpdates,
    CheckingForUpdates(bool),
}

pub struct RenderState {
    pub primary_window: PrimaryWindowState,
    timer_window: TimerWindowState,
    receiver: Receiver<RenderThreadEvent>,
    alert: Option<TextAlert>,
}

impl RenderState {
    pub fn new(receiver: Receiver<RenderThreadEvent>) -> Self {
        Self {
            receiver,
            alert: Default::default(),
            primary_window: PrimaryWindowState::new(),
            timer_window: TimerWindowState::new(),
        }
    }

    pub fn draw(&mut self, ui: &Ui) {
        let io = ui.io();
        match self.receiver.try_recv() {
            Ok(event) => {
                use RenderThreadEvent::*;
                match event {
                    DataSourceUpdates => {}
                    CheckingForUpdates(checking_for_updates) => {
                        self.primary_window.data_sources_tab.checking_for_updates =
                            checking_for_updates;
                    }
                    TimerData(timers) => {
                        self.primary_window.timer_tab.timers_update(timers);
                    }
                    AlertStart(alert) => {
                        self.alert = Some(alert);
                    }
                    AlertEnd(timer_file) => {
                        if let Some(alert) = &self.alert {
                            if Arc::ptr_eq(&alert.timer, &timer_file) {
                                self.alert = None;
                            }
                        }
                    }
                    AlertFeed(phase_state) => {
                        self.timer_window.new_phase(phase_state);
                    }
                    AlertReset(timer) => {
                        self.timer_window.remove_phase(timer);
                    }
                }
            }
            Err(_error) => (),
        }
        self.handle_alert(ui, io);
        self.timer_window.draw(ui);
        self.primary_window.draw(ui, &mut self.timer_window);
    }
    pub fn font_text(font: &str, ui: &Ui, text: &str) {
        let nexus_link = read_nexus_link().unwrap();
        let imfont_pointer = match font {
            "big" => nexus_link.font_big,
            "ui" => nexus_link.font_ui,
            _ => nexus_link.font,
        };
        let font = unsafe { Font::from_raw(&*imfont_pointer) };
        let font_handle = ui.push_font(font.id());
        ui.text(text);
        font_handle.pop();
    }
    fn handle_alert(&mut self, ui: &Ui, io: &Io) {
        if let Some(alert) = &self.alert {
            let message = &alert.message;
            let nexus_link = read_nexus_link().unwrap();
            let imfont_pointer = nexus_link.font_big;
            let imfont = unsafe { Font::from_raw(&*imfont_pointer) };
            Self::render_alert(ui, io, message, imfont.id(), imfont.scale);
        }
    }
    pub fn render_alert(
        ui: &Ui,
        io: &nexus::imgui::Io,
        text: &String,
        font: FontId,
        font_scale: f32,
    ) {
        use WindowFlags;
        let font_handle = ui.push_font(font);
        let fb_scale = io.display_framebuffer_scale;
        let [text_width, text_height] = ui.calc_text_size(text);
        let text_width = text_width * font_scale;
        let offset_x = text_width / 2.0;
        let [game_width, game_height] = io.display_size;
        let centre_x = game_width / 2.0;
        let centre_y = game_height / 2.0;
        let above_y = game_height * 0.2;
        let text_x = (centre_x - offset_x) * fb_scale[0];
        let text_y = (centre_y - above_y) * fb_scale[1];
        Window::new("TAIMIHUD_ALERT_AREA")
            .flags(
                WindowFlags::ALWAYS_AUTO_RESIZE
                    | WindowFlags::NO_TITLE_BAR
                    | WindowFlags::NO_RESIZE
                    | WindowFlags::NO_BACKGROUND
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_SCROLLBAR
                    | WindowFlags::NO_INPUTS
                    | WindowFlags::NO_FOCUS_ON_APPEARING
                    | WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS,
            )
            .position([text_x, text_y], Condition::Always)
            .size([text_width * 1.25, text_height * 2.0], Condition::Always)
            .build(ui, || {
                ui.text(text);
            });
        font_handle.pop();
    }

    pub fn lock() -> MutexGuard<'static, RenderState> {
        RENDER_STATE.get().unwrap().lock().unwrap()
    }
}

