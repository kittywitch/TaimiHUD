use {crate::bhtimer, glam::f32::Vec3};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TimerMachineState {
    // This possibly shouldn't happen?
    OffMap,
    // These can be met
    OnMap,
    OnMapWithinBoundaryUntriggered,
    Started,
    Finished,
}

#[derive(Debug, Clone)]
struct TimerMachine {
    // TODO: this should be an Arc<TimerFile>
    timer_file: bhtimer::TimerFile,
    current_phase: String,
    machine_state: TimerMachineState,
    time_elapsed: tokio::time::Duration,
    in_combat: bool,
}

impl TimerMachine {
    async fn process_state(&mut self, map_id: u32, position: Vec3, combat: bool) {
        match self.machine_state {
            TimerMachineState::OnMap => {}
            _ => (),
        }
    }
}
