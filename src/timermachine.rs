use {
    crate::{
        bhtimer::{
            TimerFile,
            TimerTriggerType,
            TaimiAlert,
            TimerPhase,
            CombatState,
        },
        RenderThreadEvent,
        geometry::{Position, Polytope},
    },
    glam::f32::Vec3,
    std::{
        sync::Arc,
        ops::Deref,
    },
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
#[derive(Debug, Clone)]
enum TimerMachineState {
    /*
    * Ensolyss: I am awake.
    * Ensolyss: I am aware.
    * Ensolyss: Suffer, mortal things.
    */
    AwakeUnaware,
    OffMap,
    OnMap,
    OnPhase(TimerFilePhase),
    FinishedPhase(TimerFilePhase),
    Finished,
}

#[derive(Debug,Clone)]
struct TimerFilePhase {
    timer: Arc<TimerFile>,
    phase: usize,
}

impl TimerFilePhase {
    fn new(timer: Arc<TimerFile>) -> Option<Self> {
        match timer.phases.is_empty() {
            true => None,
            false => Some(Self {
                timer,
                phase: 0
            }),
        }
    }

    fn reset(mut self) -> Self {
        self.phase = 0;
        self
    }

    fn next(self) -> Option<Self> {
        let phase_len = self.timer.phases.len();
        let phase = (self.phase..phase_len).next()?;
        Some(Self {
            timer: self.timer,
            phase,
        })
    }

    fn phase(&self) -> &TimerPhase {
        &self.timer.phases[self.phase]
    }
}

impl Deref for TimerFilePhase {
    type Target = TimerPhase;

    fn deref(&self) -> &Self::Target {
        self.phase()
    }
}

#[derive(Debug, Clone)]
pub struct TimerMachine {
    state: TimerMachineState,
    timer: Arc<TimerFile>,
    alert_sem: Arc<Mutex<()>>,
    sender: Sender<RenderThreadEvent>,
    combat_state: CombatState,
    tasks: Vec<Arc<JoinHandle<()>>>,
}

impl TimerMachine {
    pub fn new(timer: Arc<TimerFile>, alert_sem: Arc<Mutex<()>>, sender: Sender<RenderThreadEvent>) -> Self {
        TimerMachine {
            state: TimerMachineState::AwakeUnaware,
            timer,
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
        log::info!("Sleeping {:?} seconds for {}: a message with {:?} duration", wait_duration, message, display_duration);
        sleep(wait_duration).await;
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

    fn timer_alert(&mut self, alert: TaimiAlert) {
        let (timestamp, duration) = (alert.timestamp(), alert.duration());
        let join = self.text_alert(alert.text,timestamp, duration);
        let jarc = Arc::new(join);
        self.tasks.push(jarc);
    }


    fn reset_check(&mut self, pos: Position) {
        let trigger = &self.timer.reset;
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
        let reason = format!("Reset triggered for \"{}\"", self.timer.name);
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

    fn start_tasks(&mut self, phase: &TimerFilePhase) {
        let timers = phase.get_alerts();
        for timer in timers {
            self.timer_alert(timer);
        }
    }

    /**
        state_change is about code that should run once, upon a stage or phase change.
    */
    fn state_change(&mut self, state: TimerMachineState) {
        use TimerMachineState::*;
        let final_state = match state {
            FinishedPhase(phase) => {
                // if there are any more phases, we should go back to OnPhase
                if let Some(next_phase) = phase.next() {
                    OnPhase(next_phase)
                }
                // otherwise, we're done
                else {
                    Finished
                }
            },
            _ => state,
        };
        let reason = format!("Switching from state {:?} to {:?}", self.state, final_state);
        self.abort_tasks(reason);
        if let OnPhase(phase) = &final_state {
            self.start_tasks(phase);
        }
        self.state = final_state;
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
                let trigger = &self.timer.phases.first().unwrap().start;
                use TimerTriggerType::*;
                match &trigger.kind {
                    Location => {
                        if trigger.check(pos, self.combat_state) {
                            if let Some(phase) = TimerFilePhase::new(self.timer.clone()) {
                                self.state_change(OnPhase(phase));
                            }
                        }
                    },
                    // Go home clown
                    Key => (),
                }
            },
            // within a phase (nth)
            OnPhase(phase) => {
                // handle the finish check
                if let Some(trigger) = &phase.finish {
                    use TimerTriggerType::*;
                    match &trigger.kind {
                        Location => {
                            if trigger.check(pos, self.combat_state) {
                                self.state_change(FinishedPhase(phase.clone()));
                            }
                        },
                        Key => ()
                    }
                }
            },
            FinishedPhase(_phase) => (),
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
        let machine_map_id = &self.timer.map_id;
        if *machine_map_id == map_id {
            log::info!("On map with ID \"{}\" for \"{}\"", map_id, self.timer.name());
            self.state = TimerMachineState::OnMap;
        } else {
            log::info!("Off map with ID \"{}\" for \"{}\"", map_id, self.timer.name());
            self.state = TimerMachineState::OffMap;
        }
    }
}
