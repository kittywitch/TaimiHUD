use {
    super::TimerWindowState,
    crate::{
        controller::ProgressBarStyleChange, render::TextFont, ControllerEvent, SETTINGS, TS_SENDER,
    },
    nexus::imgui::{ComboBox, Condition, Selectable, Slider, TreeNode, TreeNodeFlags, Ui},
    strum::IntoEnumIterator,
};

pub struct ConfigTabState {}

impl ConfigTabState {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&mut self, ui: &Ui, timer_window_state: &mut TimerWindowState) {
        ui.text("You can control-click on a slider element, or such, to be able to directly input data to it.");
        let timers_window_closure = || {
            if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
                timer_window_state.progress_bar.stock = settings.progress_bar.stock;
            };
            if ui.checkbox(
                "Stock Imgui Progress Bar",
                &mut timer_window_state.progress_bar.stock,
            ) {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(
                    ProgressBarStyleChange::Stock(timer_window_state.progress_bar.stock),
                ));
                drop(event_send);
            };
            if ui.checkbox("Shadow", &mut timer_window_state.progress_bar.shadow) {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(
                    ProgressBarStyleChange::Shadow(timer_window_state.progress_bar.shadow),
                ));
                drop(event_send);
            }
            if ui.checkbox(
                "Centre text after icon",
                &mut timer_window_state.progress_bar.centre_after,
            ) {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(
                    ProgressBarStyleChange::Centre(timer_window_state.progress_bar.shadow),
                ));
                drop(event_send);
            }
            if Slider::new("Height", 8.0, 256.0)
                .display_format("%.0f")
                .build(ui, &mut timer_window_state.progress_bar.height)
            {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(
                    ProgressBarStyleChange::Height(timer_window_state.progress_bar.height),
                ));
                drop(event_send);
            }
            let font_closure = || {
                let mut selected = timer_window_state.progress_bar.font.clone();
                for font in TextFont::iter() {
                    if Selectable::new(font.to_string())
                        .selected(font == selected)
                        .build(ui)
                    {
                        selected = font.clone()
                    }
                }
                return selected;
            };
            if let Some(selection) = ComboBox::new("Font")
                .preview_value(&timer_window_state.progress_bar.font.to_string())
                .build(ui, font_closure)
            {
                let sender = TS_SENDER.get().unwrap();
                let event_send = sender.try_send(ControllerEvent::ProgressBarStyle(
                    ProgressBarStyleChange::Font(TextFont::from(selection.clone())),
                ));
                drop(event_send);
            }
        };
        let timers_window = TreeNode::new("Timers Window")
            .flags(TreeNodeFlags::empty())
            .opened(true, Condition::Once)
            .tree_push_on_open(true)
            .build(ui, timers_window_closure);
    }
}
