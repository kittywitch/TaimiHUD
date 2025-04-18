use {
    crate::{
        controller::ControllerEvent, render::{
            RenderState, TimerWindowState
        }, settings::TimerSettings, timer::TimerFile, SETTINGS, TS_SENDER
    }, glam::Vec2, indexmap::IndexMap, nexus::imgui::{
            ChildWindow, Selectable, TreeNodeFlags, Ui, WindowFlags
        }, std::{
        sync::Arc,
        collections::HashMap,
    },
};

pub struct TimerTabState {
    timers: Vec<Arc<TimerFile>>,
    categories: IndexMap<String, Vec<Arc<TimerFile>>>,
    timer_selection: Option<Arc<TimerFile>>,
    category_status: HashMap<String,bool>,
}

impl TimerTabState {
    pub fn new() -> Self {
        Self {
            timers: Default::default(),
            categories: Default::default(),
            timer_selection: Default::default(),
            category_status: Default::default(),
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
                    let sender = TS_SENDER.get().unwrap();
                    let event_send = sender.try_send(ControllerEvent::WindowState("timers".to_string(), timer_window_state.open));
                    drop(event_send);
                }
                ui.same_line();
                if ui.button("Reset Timers") {
                    timer_window_state.reset_phases();
                }
                ui.same_line();

                if ui.button("Expand All") {
                
                };
                let header_flags = TreeNodeFlags::FRAMED;
                // interface design is my passion
                let height = Vec2::from_array(ui.calc_text_size("U\nI"));
                let height = height.y;
                // SOMETHING HORRIBLE HAPPENED HERE. THIS GRAVE IS COMMEMORATION
                /*let _height = self.timers
                    .iter()
                    .map(|timer| Vec2::from_array(ui.calc_text_size(&timer.name)).y)
                    .max_by(|x, y| x.partial_cmp(y).unwrap() )
                .unwrap_or(32.0);*/
                for (category_name, category) in &mut self.categories {
                    // Header for category
                    /*let collapsing_header = CollapsingHeader
                        ::new(category_name)
                        .flags(header_flags)
                        //.default_open();
                    //  .build(ui); */
                    if ui.collapsing_header(category_name, header_flags) {
                        ui.dummy([0.0, 4.0]);
                        for timer in category {
                            let mut selected = false;
                            if let Some(selected_timer) = &self.timer_selection {
                                selected = Arc::ptr_eq(selected_timer, timer);
                            }
                            let group_token = ui.begin_group();
                            RenderState::icon(ui, Some(height), Some(&timer.icon), timer.path.as_ref());
                            if Selectable::new(&timer.combined())
                                .selected(selected)
                                .build(ui)
                            {
                                self.timer_selection = Some(timer.clone());
                            }
                            ui.dummy([0.0, 4.0]);
                            group_token.end();
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
                    RenderState::icon(ui, None, Some(&selected_timer.icon), selected_timer.path.as_ref());
                    ui.same_line();
                    let split_name = selected_timer.name.split("\n");
                    let layout_group = ui.begin_group();
                    for (i, text) in split_name.into_iter().enumerate() {
                        if i == 0 {
                            RenderState::font_text("big", ui, text);
                        } else {
                            RenderState::font_text("ui", ui, text);
                        }
                    }
                    layout_group.end();
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


