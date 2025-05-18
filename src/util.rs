/*
* Derived from belst; https://github.com/belst/nexus-wingman-uploader/blob/master/src/util.rs
*/

use {
    crate::{fl, marker::atomic::MarkerInputData},
    glam::Vec3,
    nexus::imgui::{InputFloat3, StyleColor, Ui},
};

#[allow(dead_code)]
pub trait UiExt {
    fn help_marker<F: FnOnce()>(&self, f: F) -> bool;
    fn attention_marker<F: FnOnce()>(&self, f: F) -> bool;
    fn link(&self, label: impl AsRef<str>, url: impl AsRef<str>);
}

impl UiExt for Ui<'_> {
    fn help_marker<F: FnOnce()>(&self, f: F) -> bool {
        let mut clicked = false;
        self.same_line();
        self.text_disabled("(?)");
        if self.is_item_hovered() && self.is_item_clicked() {
            clicked = true;
        }
        if self.is_item_hovered() {
            f();
        }
        clicked
    }
    fn attention_marker<F: FnOnce()>(&self, f: F) -> bool {
        let mut clicked = false;
        self.same_line();
        self.text_disabled("(!)");
        if self.is_item_hovered() && self.is_item_clicked() {
            clicked = true;
        }
        if self.is_item_hovered() {
            f();
        }
        clicked
    }
    fn link(&self, label: impl AsRef<str>, url: impl AsRef<str>) {
        let blue = self.push_style_color(StyleColor::Text, [0.0, 0.0, 1.0, 1.0]);
        self.text(label);
        blue.pop();
        let mut min = self.item_rect_min();
        let max = self.item_rect_max();
        min[1] = max[1];
        self.get_window_draw_list()
            .add_line(min, max, [0.0, 0.0, 1.0, 1.0])
            .build();
        if self.is_item_hovered() {
            if self.is_item_clicked() {
                if let Err(e) = open::that_detached(url.as_ref()) {
                    log::error!("Failed to open {}: {e}", url.as_ref());
                }
            }
            self.tooltip_text(fl!("open-button", kind = url.as_ref()));
        }
    }
}

#[derive(Default)]
pub struct PositionInput {
    pub position: Option<Vec3>,
    position_before_edit: Option<Vec3>,
    opened: bool,
}

impl PositionInput {
    pub fn draw_display(&self, ui: &Ui, trigger: bool) {
        if let Some(position) = self.position {
            let position = format!("({}, {}, {})", position.x, position.y, position.z);
            if trigger {
                ui.text_wrapped(&fl!("trigger", position = position));
                ui.help_marker(|| {
                    ui.tooltip_text(fl!("trigger-explanation"));
                });
            } else {
                ui.text_wrapped(position);
            }
        } else {
            let position = fl!("no-position");
            if trigger {
                ui.text_wrapped(&fl!("trigger", position = position));
                ui.help_marker(|| {
                    ui.tooltip_text(fl!("trigger-explanation"));
                });
            } else {
                ui.text_wrapped(position);
            }
        }
    }
    pub fn draw_take_current(&mut self, ui: &Ui) {
        if ui.button(&fl!("position-get")) {
            if let Some(mid) = MarkerInputData::read() {
                self.position = Some(mid.local_player_pos);
            }
        }
    }
    pub fn draw_edit_manual(&mut self, ui: &Ui) {
        let button_text = match self.opened {
            true => fl!("set-manually-save"),
            false => fl!("set-manually"),
        };
        if ui.button(&button_text) {
            self.opened = !self.opened;
            if self.opened {
                self.position_before_edit = self.position;
            }
        }
        if self.opened {
            let position_as_type = self.position.get_or_insert_default().as_mut();
            let position_input = InputFloat3::new(ui, fl!("manual-position"), position_as_type);
            position_input.build();
            if ui.button(&fl!("revert")) {
                self.opened = false;
                self.position = self.position_before_edit;
            }
        }
    }
}
