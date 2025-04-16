use {
    crate::{
        render::{
            TimerWindowState,
            RenderState,
        },
        settings::TimerSettings,
        controller::ControllerEvent,
        timer::TimerFile,
        SETTINGS, TS_SENDER,
    },
    indexmap::IndexMap,
    nexus::{
        imgui::{
            ChildWindow,
            Selectable,
            TreeNodeFlags,
            Ui,
            WindowFlags,
        },
    },
    std::sync::Arc,
};

pub struct TimerTabState {
    timers: Vec<Arc<TimerFile>>,
    categories: IndexMap<String, Vec<Arc<TimerFile>>>,
    timer_selection: Option<Arc<TimerFile>>,
}

impl TimerTabState {
    pub fn new() -> Self {
        Self {
            timers: Default::default(),
            categories: Default::default(),
            timer_selection: Default::default(),
        }
    }

    pub fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
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
                            RenderState::font_text("big", ui, text);
                        } else {
                            RenderState::font_text("ui", ui, text);
                        }
                    }
                    RenderState::font_text("font", ui, &format!("Author: {}", selected_timer.author()));
                    RenderState::font_text("font", ui, &selected_timer.description);
                    if let Some(settings) =
                        SETTINGS.get().and_then(|settings| settings.try_read().ok())
                    {
                        let settings_for_timer = settings.timers.get(&selected_timer.id);
                        let button_text = match settings_for_timer {
                            Some(TimerSettings { disabled: true, .. }) => "Enable",
                            _ => "Disable",
                        };
                        if ui.button(button_text) {
                            let sender = TS_SENDER.get().unwrap();
                            let event_send = sender
                                .try_send(ControllerEvent::TimerToggle(selected_timer.id.clone()));
                            drop(event_send);
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
        self.categories.sort_keys();
    }
}


