use {
    crate::{
        settings::{Settings, TimerSettings},
        taimistate::TaimiThreadEvent,
        timer::{timeralert::TimerAlert, timerfile::TimerFile},
        timermachine::{PhaseState, TextAlert},
        RENDER_STATE, SETTINGS, TS_SENDER,
    },
    nexus::{
        data_link::read_nexus_link,
        imgui::{
            internal::RawCast, ChildWindow, Condition, Font, FontId, Io, ProgressBar, Selectable,
            StyleColor, TreeNodeFlags, Ui, Window, WindowFlags,
        },
        // TODO
        //texture::{load_texture_from_file, texture_receive, Texture},
    },
    std::{
        collections::HashMap,
        sync::{Arc, MutexGuard},
    },
    tokio::{sync::mpsc::Receiver, time::Instant},
};

pub enum RenderThreadEvent {
    TimerData(Vec<Arc<TimerFile>>),
    AlertFeed(PhaseState),
    AlertReset(Arc<TimerFile>),
    AlertStart(TextAlert),
    AlertEnd(Arc<TimerFile>),
}

pub struct RenderState {
    receiver: Receiver<RenderThreadEvent>,
    primary_window_open: bool,
    timers_window_open: bool,
    alert: Option<TextAlert>,
    phase_states: Vec<PhaseState>,
    timers: Vec<Arc<TimerFile>>,
    categories: HashMap<String, Vec<Arc<TimerFile>>>,
    timer_selection: Option<Arc<TimerFile>>,
    settings: Settings,
}

impl RenderState {
    pub fn new(receiver: Receiver<RenderThreadEvent>) -> Self {
        Self {
            receiver,
            settings: SETTINGS.get().unwrap().clone(),
            primary_window_open: true,
            timers_window_open: true,
            alert: Default::default(),
            phase_states: Default::default(),
            timers: Default::default(),
            categories: Default::default(),
            timer_selection: Default::default(),
        }
    }

    pub fn progress_bar(alert: &TimerAlert, ui: &Ui, start: Instant) {
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

    pub fn main_window_keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            self.primary_window_open = !self.primary_window_open;
        }
    }
    pub fn render(&mut self, ui: &Ui) {
        let io = ui.io();
        match self.receiver.try_recv() {
            Ok(event) => {
                use RenderThreadEvent::*;
                match event {
                    TimerData(timers) => {
                        self.timers = timers;
                        for timer in &self.timers {
                            self.categories.entry(timer.category.clone()).or_default();
                            if let Some(val) = self.categories.get_mut(&timer.category) {
                                val.push(timer.clone());
                            };
                        }
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
                        log::info!("I received an alert feed event!");
                        self.phase_states.push(phase_state);
                    }
                    AlertReset(timer_file) => {
                        log::info!("I received an alert reset event!");
                        self.phase_states
                            .retain(|p| !Arc::ptr_eq(&p.timer, &timer_file));
                    }
                }
            }
            Err(_error) => (),
        }
        self.handle_alert(ui, io);
        self.handle_taimi_main_window(ui);
        self.handle_timers_window(ui);
    }
    fn handle_timer_sidebar(&mut self, ui: &Ui) {
        let child_window_flags = WindowFlags::HORIZONTAL_SCROLLBAR;
        ChildWindow::new("timer_sidebar")
            .flags(child_window_flags)
            .size([0.0, 0.0])
            .build(ui, || {
                let button_text = match self.timers_window_open {
                    true => "Close Timers",
                    false => "Open Timers",
                };
                if ui.button(button_text) {
                    self.timers_window_open = !self.timers_window_open;
                }
                ui.same_line();
                if ui.button("Reset Timers") {
                    self.phase_states.clear();
                }
                let header_flags = TreeNodeFlags::FRAMED;
                for (category_name, category) in &mut self.categories {
                    // Header for category
                    ui.collapsing_header(category_name, header_flags);
                    for timer in category {
                        let mut selected = false;
                        if let Some(selected_timer) = &self.timer_selection {
                            selected = Arc::ptr_eq(selected_timer, timer);
                        }
                        if Selectable::new(timer.name.clone())
                            .selected(selected)
                            .build(ui)
                        {
                            self.timer_selection = Some(timer.clone());
                        }
                    }
                }
            });
    }
    fn handle_timer_main(&mut self, ui: &Ui) {
        let child_window_flags = WindowFlags::HORIZONTAL_SCROLLBAR;
        ChildWindow::new("timer_main")
            .flags(child_window_flags)
            .size([0.0, 0.0])
            .build(ui, || {
                if let Some(selected_timer) = &self.timer_selection {
                    let split_name = selected_timer.name.split("\n");
                    for (i, text) in split_name.into_iter().enumerate() {
                        if i == 0 {
                            Self::big_header(ui, text);
                        } else {
                            Self::ui_header(ui, text);
                        }
                    }
                    Self::fonted_text(ui, &format!("Author: {}", selected_timer.author()));
                    Self::fonted_text(ui, &selected_timer.description);
                    let settings_lock = self.settings.blocking_read();
                    let settings_for_timer = settings_lock.timers.get(&selected_timer.id);
                    let state = match settings_for_timer {
                        Some(setting) => setting.disabled,
                        None => false,
                    };
                    drop(settings_lock);
                    let button_text = match state {
                        true => "Enable",
                        false => "Disable",
                    };
                    if ui.button(button_text) {
                        let mut settings_lock = self.settings.blocking_write();
                        settings_lock.toggle_timer(selected_timer.id.clone());
                        let sender = TS_SENDER.get().unwrap();
                        match !state {
                            true => {
                                let event_send = sender.try_send(TaimiThreadEvent::TimerEnable(
                                    selected_timer.id.clone(),
                                ));
                                drop(event_send);
                            }
                            false => {
                                let event_send = sender.try_send(TaimiThreadEvent::TimerEnable(
                                    selected_timer.id.clone(),
                                ));
                                drop(event_send);
                            }
                        }
                        drop(settings_lock);
                    }
                } else {
                    ui.text("Please select a timer to configure!");
                }
            });
    }
    fn big_header(ui: &Ui, text: &str) {
        let nexus_link = read_nexus_link().unwrap();
        let imfont_pointer = nexus_link.font_big;
        let font = unsafe { Font::from_raw(&*imfont_pointer) };
        let font_handle = ui.push_font(font.id());
        ui.text(text);
        font_handle.pop();
    }
    fn ui_header(ui: &Ui, text: &str) {
        let nexus_link = read_nexus_link().unwrap();
        let imfont_pointer = nexus_link.font_ui;
        let font = unsafe { Font::from_raw(&*imfont_pointer) };
        let font_handle = ui.push_font(font.id());
        ui.text(text);
        font_handle.pop();
    }
    fn fonted_text(ui: &Ui, text: &str) {
        let nexus_link = read_nexus_link().unwrap();
        let imfont_pointer = nexus_link.font;
        let font = unsafe { Font::from_raw(&*imfont_pointer) };
        let font_handle = ui.push_font(font.id());
        ui.text(text);
        font_handle.pop();
    }
    fn handle_taimi_main_window(&mut self, ui: &Ui) {
        let mut primary_window_open = self.primary_window_open;
        if self.primary_window_open {
            Window::new("Taimi")
                .opened(&mut primary_window_open)
                .build(ui, || {
                    if let Some(_token) = ui.tab_bar("modules") {
                        if let Some(_token) = ui.tab_item("Timers") {
                            //let table_token = ui.begin_table("timers_main", 2);
                            ui.columns(2, "timers_main", true);
                            //let window_size = ui.window_size();
                            //let sidebar_width = window_size[1] * 0.3;
                            //ui.set_current_column_width(sidebar_width);
                            self.handle_timer_sidebar(ui);
                            ui.next_column();
                            //let main_width = window_size[1] - sidebar_width;
                            //ui.set_current_column_width(main_width);
                            self.handle_timer_main(ui);
                            //drop(table_token);
                            ui.columns(1, "timers_main_end", false)
                        };
                        if let Some(_token) = ui.tab_item("Markers") {
                            ui.text("To-do!");
                        }
                        if let Some(_token) = ui.tab_item("Pathing") {
                            ui.text("To-do!");
                        }
                    }
                });
        }
        self.primary_window_open = primary_window_open;
    }
    fn handle_timers_window(&mut self, ui: &Ui) {
        if self.timers_window_open {
            Window::new("Timers")
                .opened(&mut self.timers_window_open)
                .build(ui, || {
                    for ps in &self.phase_states {
                        for alert in ps.alerts.iter() {
                            Self::progress_bar(alert, ui, ps.start)
                        }
                    }
                });
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
