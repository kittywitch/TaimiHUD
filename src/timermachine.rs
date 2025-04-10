use {
    crate::{
        bhtimer::{
            TimerFile,
            TimerTriggerType,
            TimerTrigger,
            TimerAlert,
            TimerPhase,
            CombatState,
        },
        RenderThreadEvent,
        geometry::{Position, Polytope},
    },
    glam::f32::Vec3,
    std::sync::Arc,
    tokio::{
        task::JoinHandle,
        time::{sleep, Duration},
        sync::{
            mpsc::{Receiver, Sender},
            Mutex,
        },
    },
};

/*
* A timer can be:
* - existent without knowledge of current map
* - off the map
* - on the map, first phase untriggered
* - phase triggered, cycling through alerts
* - nth phase done, next phase
* - finished, denoted by a different area, departure, out of combat, ...
* - failed, with reset condition
*/
#[derive(Debug, Clone, PartialEq, Eq)]
enum TimerMachineState {
    /*
    * Ensolyss: I am awake.
    * Ensolyss: I am aware.
    * Ensolyss: Suffer, mortal things.
    */
    AwakeUnaware,
    OffMap,
    OnMap,
    OnPhase(usize),
    FinishedPhase(usize),
    Finished,
}

#[derive(Debug, Clone)]
pub struct TimerMachine {
    state: TimerMachineState,
    timer_file: Arc<TimerFile>,
    phases: Vec<TimerPhase>,
    reset: TimerTrigger,
    alert_sem: Arc<Mutex<()>>,
    sender: Sender<RenderThreadEvent>,
    combat_state: CombatState,
    tasks: Vec<Arc<JoinHandle<()>>>,
}

impl TimerMachine {
    pub fn new(timer_file: Arc<TimerFile>, alert_sem: Arc<Mutex<()>>, sender: Sender<RenderThreadEvent>) -> Self {
        TimerMachine {
            state: TimerMachineState::AwakeUnaware,
            phases: timer_file.clone().phases.clone(),
            reset: timer_file.clone().reset.clone(),
            timer_file,
            alert_sem,
            sender,
            combat_state: CombatState::Outside,
            tasks: Default::default(),
        }
    }

    async fn send_alert_event(
        sender: Sender<RenderThreadEvent>,
        lock: Arc<Mutex<()>>,
        message: String,
        wait_duration: Duration,
        display_duration: Duration,
    ) {
        if let Some(wait_duration_real) = wait_duration.checked_sub(display_duration) {
            log::info!("Sleeping {:?} seconds for {}: a message with {:?} duration", wait_duration, message, display_duration);
            sleep(wait_duration_real).await;
        } else {
            log::info!("Immediate {}: a message with {:?} duration", message, display_duration);
        }
        let alert_handle = lock.lock().await;
        log::info!("Slept {:?} seconds, displaying {}: a message with {:?} duration", wait_duration, message, display_duration);
        let _ = sender.send(RenderThreadEvent::AlertStart(message.clone())).await;
        sleep(display_duration).await;
        let _ = sender.send(RenderThreadEvent::AlertEnd).await;
        log::info!("Stopping displaying {}: we slept for {:?} a message with {:?} duration", message, wait_duration, display_duration);
        // this is my EMOTIONAL SUPPORT drop
        drop(alert_handle);
    }

    fn text_alert(&self, message: String, wait_duration: Duration, display_duration: Duration) -> JoinHandle<()> {
        tokio::spawn(Self::send_alert_event(
            self.sender.clone(),
            self.alert_sem.clone(),
            message,
            wait_duration,
            display_duration,
        ))
    }

    fn timer_alert(&mut self, ts: f32, alert: TimerAlert) {
        let alert = alert.ambiguous();
        let ts_d = Duration::from_secs_f32(ts);
        let text = format!("{}: {}", alert.kind, alert.text);

        let join = self.text_alert(text, ts_d, alert.duration());
        let jarc = Arc::new(join);
        self.tasks.push(jarc);
    }


    fn reset_check(&mut self, pos: Position) {
        let trigger = &self.reset;
        let shape = trigger.polytope().unwrap();
        use TimerMachineState::*;
        match &self.state {
            OnPhase(_) | FinishedPhase(_) => {
                if trigger.check(pos, self.combat_state) {
                    self.do_reset();
                }
            },
            _ => (),
        }
    }

    fn do_reset(&mut self) {
        let reason = format!("Reset triggered for \"{}\"", self.timer_file.name);
        log::info!("Reset triggered!");
        self.combat_state = CombatState::Outside;
        self.state_change(TimerMachineState::OnMap);
        self.abort_tasks(reason.clone());
        let zero_s = Duration::from_secs(0);
        let one_s = Duration::from_secs(1);
        self.text_alert(reason, zero_s, one_s);
    }

    fn abort_tasks(&mut self, reason: String) {
        log::info!("Aborting {} tasks for reason: \"{}\".", self.tasks.len(), reason);
        // Kill currently running timers
        for task in &self.tasks {
            log::debug!("Aborting task with ID \"{:?}\".", task.id());
            task.abort()
        }
        // Clean up HUD text
        let alert_handle = self.alert_sem.lock();
        let _ = self.sender.send(RenderThreadEvent::AlertEnd);
        // The usual emotional support drop
        drop(alert_handle);
    }

    fn phase_count(&mut self) -> usize {
        self.phases.len()
    }

    fn next_phase(&mut self, current_phase: usize) -> Option<(TimerPhase, usize)> {
        let next_phase_id = self.get_next_phase_id(current_phase);
        if let Some(phase_id) = next_phase_id {
            Some((self.phases[phase_id].clone(), phase_id))
        } else {
            None
        }
    }

    fn get_next_phase_id(&mut self, current_phase: usize) -> Option<usize> {
        if current_phase + 1 < self.phase_count() - 1 {
            let next_phase_id = current_phase + 1;
            Some(next_phase_id)
        } else {
            None
        }
    }

    fn start_tasks(&mut self, phase: usize) {
        let mut phase = self.timer_file.phases.get(phase).unwrap().clone();
            for alert in &mut phase.alerts {
                if let Some(timestamps) = &alert.timestamps {
                    for timestamp in timestamps {
                        self.timer_alert(*timestamp, alert.clone())
                    }
                }
            }
    }

    fn set_state(&mut self, state: TimerMachineState) {
        let reason = format!("Switching from state {:?} to {:?}", self.state, state);
        log::info!("{}", reason);
        self.abort_tasks(reason);
        self.state = state;
    }

    /**
        state_change is about code that should run once, upon a stage or phase change.
    */
    fn state_change(&mut self, state: TimerMachineState) {
        self.set_state(state);
        use TimerMachineState::*;
        match self.state {
            AwakeUnaware => (),
            OffMap => (),
            OnMap => (),
            OnPhase(current_phase) => {
                self.start_tasks(current_phase);
            },
            FinishedPhase(current_phase) => {
                // if there are any more phases, we should go back to OnPhase
                if let Some((_phase, phase_id)) = self.next_phase(current_phase) {
                    self.set_state(OnPhase(phase_id));
                }
                // otherwise, we're done
                else {
                    self.set_state(Finished);
                }
            },
            Finished => (),
        }
    }

    /**
    * tick, in comparison to state_change, runs perpetually and is used for
    * checking to see if conditions for a next phase are met
    */
    pub fn tick(&mut self, pos: Position) {
        // It is always important to check if we have met the conditions for resetting the timer
        self.reset_check(pos);

        use TimerMachineState::*;
        match &self.state {
            // We exist, but is there anything to do about that?
            // Nothing, without the current map. Lost adrift in the void.
            AwakeUnaware => (),
            // We're off map, this means the timer conditions cannot be met.
            OffMap => (),
            // OnMap means time to start looking for our conditions, with location and
            // (unimplemented) key first.
            OnMap => {
                // All timers have a start trigger and a zeroth (first) phase
                let trigger = &self.timer_file.phases.first().unwrap().start;
                use TimerTriggerType::*;
                match &trigger.kind {
                    Location => {
                        if trigger.check(pos, self.combat_state) {
                            self.state_change(OnPhase(0));
                        }
                    },
                    // Go home clown
                    Key => (),
                }
            },
            // within a phase (nth)
            &OnPhase(phase) => {
                // handle the finish check
                if let Some(trigger) = &self.timer_file.phases[phase].finish {
                    use TimerTriggerType::*;
                    match &trigger.kind {
                        Location => {
                            if trigger.check(pos, self.combat_state) {
                                self.state_change(FinishedPhase(phase));
                            }
                        },
                        Key => ()
                    }
                }
            },
            &FinishedPhase(_phase) => (),
            Finished => (),
        }
    }

    pub fn combat_entered(&mut self) {
        self.combat_state = CombatState::Entered;
    }

    pub fn combat_exited(&mut self) {
        self.combat_state = CombatState::Exited;
    }

    pub fn update_on_map(&mut self, map_id: u32) {
        let machine_map_id = &self.timer_file.map_id;
        if *machine_map_id == map_id {
            log::info!("On map with ID \"{}\" for \"{}\"", map_id, self.timer_file.name());
            self.state = TimerMachineState::OnMap;
        } else {
            log::info!("Off map with ID \"{}\" for \"{}\"", map_id, self.timer_file.name());
            self.state = TimerMachineState::OffMap;
        }
    }
}
