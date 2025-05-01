pub mod action;
pub mod alert;
pub mod blishcolour;
pub mod file;
pub mod geometry;
pub mod marker;
pub mod phase;
pub mod state_machine;
pub mod trigger;

#[allow(unused_imports)]
pub use {
    action::{TimerAction, TimerActionType},
    alert::{BlishAlert, TimerAlert, TimerAlertType},
    blishcolour::BlishColour,
    file::TimerFile,
    geometry::{BlishPosition, BlishVec3, Polytope, Position},
    marker::{BlishMarker, TimerMarker, RotationType},
    phase::TimerPhase,
    state_machine::{PhaseState, TextAlert, TimerKeybinds, TimerMachine},
    trigger::{CombatState, TimerTrigger, TimerTriggerType},
};
