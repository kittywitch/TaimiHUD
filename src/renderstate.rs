use {
    crate::{
        settings::{DownloadData, NeedsUpdate, Settings, TimerSettings},
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
    DataSourceUpdates,
}

pub struct TimerWindowState {
    open: bool,
    phase_states: Vec<PhaseState>,
}

impl TimerWindowState {
    pub fn new() -> Self {
        Self {
            open: true,
            phase_states: Default::default(),
        }
    }

    fn draw(&mut self, ui: &Ui) {
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

pub struct TimerTabState {
    timers: Vec<Arc<TimerFile>>,
    categories: HashMap<String, Vec<Arc<TimerFile>>>,
    timer_selection: Option<Arc<TimerFile>>,
}

impl TimerTabState {
    fn new() -> Self {
        Self {
            timers: Default::default(),
            categories: Default::default(),
            timer_selection: Default::default(),
        }
    }

    fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        ui.columns(2, "timers_tab_start", true);
        self.draw_sidebar(ui, timer_window_state);
        ui.next_column();
        self.draw_main(ui);
        ui.columns(1, "timers_tab_end", false)
    }

    fn draw_sidebar(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        let child_window_flags = WindowFlags::HORIZONTAL_SCROLLBAR;
        ChildWindow::new("timer_sidebar")
            .flags(child_window_flags)
            .size([0.0, 0.0])
            .build(ui, || {
                let button_text = match timer_window_state.open {
                    true => "Close Timers",
                    false => "Open Timers",
                };
                if ui.button(button_text) {
                    timer_window_state.open = !timer_window_state.open;
                }
                ui.same_line();
                if ui.button("Reset Timers") {
                    timer_window_state.reset_phases();
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
    fn draw_main(&mut self, ui: &Ui) {
        let child_window_flags = WindowFlags::HORIZONTAL_SCROLLBAR;
        ChildWindow::new("timer_main")
            .flags(child_window_flags)
            .size([0.0, 0.0])
            .build(ui, || {
                if let Some(selected_timer) = &self.timer_selection {
                    let split_name = selected_timer.name.split("\n");
                    for (i, text) in split_name.into_iter().enumerate() {
                        if i == 0 {
                            RenderState::big_header(ui, text);
                        } else {
                            RenderState::ui_header(ui, text);
                        }
                    }
                    RenderState::fonted_text(ui, &format!("Author: {}", selected_timer.author()));
                    RenderState::fonted_text(ui, &selected_timer.description);
                    if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
                        let settings_for_timer = settings.timers.get(&selected_timer.id);
                        let button_text = match settings_for_timer {
                            Some(v) if v.disabled => Some("Enable"),
                            Some(_v) => Some("Disable"),
                            None => None,
                        };

                        if let Some(button_text) = button_text {
                            if ui.button(button_text) {
                                let sender = TS_SENDER.get().unwrap();
                                let event_send = sender.try_send(TaimiThreadEvent::TimerToggle(
                                    selected_timer.id.clone(),
                                ));
                                drop(event_send);
                            }
                        }
                    }
                } else {
                    ui.text("Please select a timer to configure!");
                }
            });
    }
    pub fn timers_update(&mut self, timers: Vec<Arc<TimerFile>>) {
        self.timers = timers;
        for timer in &self.timers {
            self.categories.entry(timer.category.clone()).or_default();
            if let Some(val) = self.categories.get_mut(&timer.category) {
                val.push(timer.clone());
            };
        }
    }
}

pub struct DataSourceTabState {}

impl DataSourceTabState {
    fn new() -> Self {
        Self {}
    }

    fn draw(&self, ui: &Ui) {
            if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            if ui.button("Check for updates") {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(TaimiThreadEvent::CheckDataSourceUpdates);
                drop(event_send);
            }
            let settings = settings_lock.blocking_read();
            for source in &settings.downloaded_releases {
                let source_update = format!(
                    "{}/{}: {}",
                    source.owner, source.repository, source.needs_update
                );
                ui.text(source_update);
                use NeedsUpdate::*;
                let button_text = match &source.needs_update {
                    Unknown => Some("Attempt to update anyway?"),
                    Known(needs, _id) if *needs => Some("Update"),
                    Known(_needs, _id) => None,
                };
                if let Some(button_text) = button_text {
                    ui.same_line();
                    if ui.button(button_text) {
                        let sender = TS_SENDER.get().unwrap();
                        let event_send = sender.try_send(TaimiThreadEvent::DoDataSourceUpdate {
                            owner: source.owner.clone(),
                            repository: source.repository.clone(),
                        });
                        drop(event_send);
                    }
                }
            }
            drop(settings_lock);
        } else {
            ui.text("Settings have not yet loaded!");
        }
    }
}

pub struct PrimaryWindowState {
    timer_tab: TimerTabState,
    data_sources_tab: DataSourceTabState,
    open: bool,
}

impl PrimaryWindowState {
    pub fn new() -> Self {
        Self {
            timer_tab: TimerTabState::new(),
            data_sources_tab: DataSourceTabState::new(),
            open: true,
        }
    }

    pub fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        let mut open = self.open;
        if self.open {
            Window::new("Taimi").opened(&mut open).build(ui, || {
                if let Some(_token) = ui.tab_bar("modules") {
                    if let Some(_token) = ui.tab_item("Timers") {
                        self.timer_tab.draw(ui, timer_window_state);
                    };
                    if let Some(_token) = ui.tab_item("Markers") {
                        ui.text("To-do!");
                    }
                    if let Some(_token) = ui.tab_item("Pathing") {
                        ui.text("To-do!");
                    }
                    if let Some(_token) = ui.tab_item("Data Sources") {
                        self.data_sources_tab.draw(ui);
                    }
                }
            });
        }
        self.open = open;
    }

    pub fn keybind_handler(&mut self, _id: &str, is_release: bool) {
        if !is_release {
            self.open = !self.open;
        }
    }
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
