use {
    crate::{
        render::{
            PrimaryWindowState,
            TimerWindowState,
        },
        controller::ControllerEvent,
        timer::{PhaseState, TextAlert, TimerFile},
        RENDER_STATE, TS_SENDER,
        settings::ProgressBarSettings,
    }, glam::Vec2, nexus::{
        data_link::read_nexus_link,
        imgui::{
            internal::RawCast, Condition, Font, FontId, Io, StyleColor, Ui, Window, WindowFlags, Image,
            Context
        },
        texture::get_texture,
    },
    strum_macros::{EnumIter, Display},
    serde::{Serialize,Deserialize},
    std::{
        path::PathBuf,
        sync::{Arc, MutexGuard}
    }, tokio::sync::mpsc::Receiver,
    relative_path::RelativePathBuf,
};

pub enum RenderEvent {
    TimerData(Vec<Arc<TimerFile>>),
    AlertFeed(PhaseState),
    AlertReset(Arc<TimerFile>),
    AlertStart(TextAlert),
    AlertEnd(Arc<TimerFile>),
    CheckingForUpdates(bool),
    ProgressBarUpdate(ProgressBarSettings),
}

#[derive(Display,Default,Clone,Debug,Deserialize,Serialize,EnumIter,PartialEq)]
#[serde(rename_all="snake_case")]
pub enum TextFont {
    #[default]
    Fontless,
    Font,
    Ui,
    Big,
}

pub struct RenderState {
    pub primary_window: PrimaryWindowState,
    timer_window: TimerWindowState,
    receiver: Receiver<RenderEvent>,
    alert: Option<TextAlert>,
}

impl RenderState {
    pub fn new(receiver: Receiver<RenderEvent>) -> Self {
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
                use RenderEvent::*;
                match event {
                    ProgressBarUpdate(settings) => {
                        self.timer_window.progress_bar = settings;
                    },
                    CheckingForUpdates(checking_for_updates) => {
                        self.primary_window.data_sources_tab.checking_for_updates =
                            checking_for_updates;
                    },
                    TimerData(timers) => {
                        self.primary_window.timer_tab.timers_update(timers);
                    },
                    AlertStart(alert) => {
                        self.alert = Some(alert);
                    },
                    AlertEnd(timer_file) => {
                        if let Some(alert) = &self.alert {
                            if Arc::ptr_eq(&alert.timer, &timer_file) {
                                self.alert = None;
                            }
                        }
                    },
                    AlertFeed(phase_state) => {
                        self.timer_window.new_phase(phase_state);
                    },
                    AlertReset(timer) => {
                        self.timer_window.remove_phase(timer);
                    },
                }
            }
            Err(_error) => (),
        }
        self.handle_alert(ui, io);
        self.timer_window.draw(ui);
        self.primary_window.draw(ui, &mut self.timer_window);
    }
    pub fn icon(ui: &Ui, height: Option<f32>, alert_icon: Option<&RelativePathBuf>, path: Option<&PathBuf>) {
            if let Some(icon) = alert_icon {
                    if let Some(path) = path {
                        if let Some(icon) = get_texture(icon.as_str()) {
                        let size = match height {
                            Some(height) => [height,height],
                            None => icon.size(),
                        };
                        Image::new(icon.id(),size).build(ui);
                        ui.same_line();
                    } else {
                        let sender = TS_SENDER.get().unwrap();
                        let event_send = sender.try_send(ControllerEvent::LoadTexture(icon.clone(), path.to_path_buf()));
                        drop(event_send);
                    }
                }
            };
    }
    pub fn font_text(font: &str, ui: &Ui, text: &str) {
        let mut font_handles = Vec::new();
        let nexus_link = read_nexus_link().unwrap();
        let imfont_pointer = match font {
            "big" => Some(nexus_link.font_big),
            "ui" => Some(nexus_link.font_ui),
            "font" => Some(nexus_link.font),
            _ => None,
        };
        if let Some(ptr) = imfont_pointer {
            let font = unsafe { Font::from_raw(&*ptr) };
            let font_handle = ui.push_font(font.id());
            font_handles.push(font_handle);
        }
        ui.text(text);
        for font_handle in font_handles {
            font_handle.pop();
        }
    }
    pub fn offset_font_text(font: &str, ui: &Ui, position: Vec2, bounding_size: Vec2, shadow: bool, text: &str) {
        let mut font_handles = Vec::new();
        let nexus_link = read_nexus_link().unwrap();
        let imfont_pointer = match font {
            "big" => Some(nexus_link.font_big),
            "ui" => Some(nexus_link.font_ui),
            "font" => Some(nexus_link.font),
            _ => None,
        };
        if let Some(ptr) = imfont_pointer {
            let font = unsafe { Font::from_raw(&*ptr) };
            let font_handle = ui.push_font(font.id());
            font_handles.push(font_handle);
        }
        let text_size = Vec2::from(ui.calc_text_size(text));
        let cursor_pos = Alignment::get_position(Alignment::CENTRE_MIDDLE, position, bounding_size, text_size);
        if shadow {
            let cursor_pos_shadow = cursor_pos + Vec2 {
                x: 2.0,
                y: text_size.y / 8.0,
            };
            ui.set_cursor_pos(cursor_pos_shadow.into());
            let token = ui.push_style_color(StyleColor::Text, [0.0, 0.0, 0.0, 1.0]);
            ui.text(text);
            token.pop();
        }
        ui.set_cursor_pos(cursor_pos.into());
        ui.text(text);
        for font_handle in font_handles {
            font_handle.pop();
        }
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

pub struct Alignment {
}


#[allow(dead_code)]
impl Alignment {
    pub const LEFT_TOP: Vec2 = Vec2::new(0.0, 0.0);
    pub const LEFT_MIDDLE: Vec2 = Vec2::new(0.0, 0.5);
    pub const LEFT_BOTTOM: Vec2 = Vec2::new(0.0, 1.0);
    pub const CENTRE_TOP: Vec2 = Vec2::new(0.5, 0.0);
    pub const CENTRE_MIDDLE: Vec2 = Vec2::new(0.5, 0.5);
    pub const CENTRE_BOTTOM: Vec2 = Vec2::new(0.5, 1.0);
    pub const RIGHT_TOP: Vec2 = Vec2::new(1.0, 0.0);
    pub const RIGHT_MIDDLE: Vec2 = Vec2::new(1.0, 0.5);
    pub const RIGHT_BOTTOM: Vec2 = Vec2::new(1.0, 1.0);

    pub fn get_position(scaler: Vec2, position: Vec2, bounding_size: Vec2, element_size: Vec2) -> Vec2 {
        let scaled_size = (bounding_size - element_size) * scaler;
        position + scaled_size

    }

    pub fn set_cursor(ui: &Ui, scaler: Vec2, position: Vec2, bounding_size: Vec2, element_size: Vec2) {
        ui.set_cursor_pos(Self::get_position(scaler, position, bounding_size, element_size).into());
    }

    pub fn set_cursor_with_offset(ui: &Ui, scaler: Vec2, position: Vec2, bounding_size: Vec2, element_size: Vec2, offset: Vec2) {
        let position = position + offset;
        Self::set_cursor(ui, scaler, position, bounding_size, element_size);
    }
}
