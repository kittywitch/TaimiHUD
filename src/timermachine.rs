use {
    crate::{
        geometry::Position,
        timer::{
            TimerAlert,
            TimerFile,
            TimerPhase,
            CombatState,
        },
        RenderThreadEvent,
    },
    std::{fmt::Display, ops::Deref, sync::Arc},
    tokio::{
        sync::{mpsc::Sender, Mutex},
        task::JoinHandle,
        time::{sleep, Duration, Instant},
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

impl Display for TimerMachineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TimerMachineState::*;
        match self {
            AwakeUnaware => write!(f, "AwakeUnaware"),
            OffMap => write!(f, "OffMap"),
            OnMap => write!(f, "OnMap"),
            OnPhase(tfp) => write!(f, "OnPhase {}", tfp.name),
            FinishedPhase(tfp) => write!(f, "FinishedPhase {}", tfp.name),
            Finished => write!(f, "Finished"),
        }
    }
}

#[derive(Debug, Clone)]
struct TimerFilePhase {
    timer: Arc<TimerFile>,
    phase: usize,
}

impl TimerFilePhase {
    fn new(timer: Arc<TimerFile>) -> Option<Self> {
        match timer.phases.is_empty() {
            true => None,
            false => Some(Self { timer, phase: 0 }),
        }
    }

    #[allow(dead_code)]
    fn reset(mut self) -> Self {
        self.phase = 0;
        self
    }

    fn next(self) -> Option<Self> {
        let phase_len = self.timer.phases.len();
        let phase = (self.phase + 1..phase_len).next()?;
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
    pub timer: Arc<TimerFile>,
    alert_sem: Arc<Mutex<()>>,
    sender: Sender<RenderThreadEvent>,
    combat_state: CombatState,
    tasks: Vec<Arc<JoinHandle<()>>>,
    key_pressed: bool,
}

#[derive(Clone)]
pub struct PhaseState {
    pub timer: Arc<TimerFile>,
    pub start: Instant,
    pub alerts: Vec<TimerAlert>,
}

#[derive(Clone)]
pub struct TextAlert {
    pub timer: Arc<TimerFile>,
    pub message: String,
}

impl TimerMachine {
    pub fn new(
        timer: Arc<TimerFile>,
        alert_sem: Arc<Mutex<()>>,
        sender: Sender<RenderThreadEvent>,
    ) -> Self {
        TimerMachine {
            state: TimerMachineState::AwakeUnaware,
            timer,
            alert_sem,
            sender,
            combat_state: CombatState::Outside,
            tasks: Default::default(),
            key_pressed: false,
        }
    }

    async fn send_alert_event(
        sender: Sender<RenderThreadEvent>,
        lock: Arc<Mutex<()>>,
        timer: Arc<TimerFile>,
        message: String,
        wait_duration: Duration,
        display_duration: Duration,
    ) {
        log::info!(
            "Sleeping {:?} for {}: a message with {:?} duration",
            wait_duration,
            message,
            display_duration
        );
        sleep(wait_duration).await;
        let alert_handle = lock.lock().await;
        log::info!(
            "Slept {:?}, displaying {}: a message with {:?} duration",
            wait_duration,
            message,
            display_duration
        );
        let _ = sender
            .send(RenderThreadEvent::AlertStart(TextAlert {
                timer: timer.clone(),
                message: message.clone(),
            }))
            .await;
        sleep(display_duration).await;
        let _ = sender
            .send(RenderThreadEvent::AlertEnd(timer.clone()))
            .await;
        log::info!(
            "Stopping displaying {}: we slept for {:?} a message with {:?} duration",
            message,
            wait_duration,
            display_duration
        );
        // this is my EMOTIONAL SUPPORT drop
        drop(alert_handle);
    }

    fn text_alert(
        &self,
        message: String,
        wait_duration: Duration,
        display_duration: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(Self::send_alert_event(
            self.sender.clone(),
            self.alert_sem.clone(),
            self.timer.clone(),
            message,
            wait_duration,
            display_duration,
        ))
    }

    #[cfg(whee)]
    fn timer_alert(&mut self, alert: TimerAlert) {
        let (timestamp, duration) = (alert.timestamp(), alert.duration());
        let join = self.text_alert(alert.text, timestamp, duration);
        let jarc = Arc::new(join);
        self.tasks.push(jarc);
    }

    async fn reset_check(&mut self, pos: Position) {
        let trigger = &self.timer.reset;
        use TimerMachineState::*;
        match &self.state {
            OnPhase(_) | FinishedPhase(_) => {
                if trigger.check(pos, self.combat_state, self.key_pressed) {
                    self.do_reset().await;
                }
            }
            _ => (),
        }
    }

    async fn do_reset(&mut self) {
        let reason = format!("Reset triggered for \"{}\"", self.timer.name);
        log::info!("Reset triggered!");
        self.combat_state = CombatState::Outside;
        self.state_change(TimerMachineState::OnMap).await;
        self.abort_tasks(reason.clone()).await;
        let zero_s = Duration::from_secs(0);
        let one_s = Duration::from_secs(1);
        self.text_alert(reason, zero_s, one_s);
    }

    pub async fn cleanup(&mut self) {
        let reason = format!(
            "\"{}\" is being told to cleanup, about to be deleted!",
            self.timer.name
        );
        self.abort_tasks(reason).await;
        let event_send = self
            .sender
            .send(RenderThreadEvent::AlertEnd(self.timer.clone()))
            .await;
        drop(event_send);
    }

    #[cfg(whee)]
    fn abort_tasks_old(&mut self, reason: String) {
        log::info!(
            "Aborting {} tasks for reason: \"{}\".",
            self.tasks.len(),
            reason
        );
        // Kill currently running timers
        for task in &self.tasks {
            log::debug!("Aborting task with ID \"{:?}\".", task.id());
            task.abort()
        }
        // Clean up HUD text
        let alert_handle = self.alert_sem.lock();
        let event_send = self.sender.send(RenderThreadEvent::AlertEnd);
        // The usual emotional support drop
        drop(alert_handle);
        drop(event_send);
    }

    async fn abort_tasks(&self, reason: String) {
        log::info!(
            "Aborting {} tasks for reason: \"{}\".",
            self.tasks.len(),
            reason
        );
        self.sender
            .send(RenderThreadEvent::AlertReset(self.timer.clone()))
            .await
            .unwrap();
    }

    #[cfg(whee)]
    fn start_tasks_old(&mut self, phase: &TimerFilePhase) {
        let timers = phase.get_alerts();
        for timer in timers {
            self.timer_alert(timer);
        }
    }

    async fn start_tasks(&self, phase: &TimerFilePhase) {
        let alerts = phase.get_alerts();
        let phase_state = PhaseState {
            timer: self.timer.clone(),
            start: Instant::now(),
            alerts,
        };
        self.sender
            .send(RenderThreadEvent::AlertFeed(phase_state))
            .await
            .unwrap();
    }

    /**
        state_change is about code that should run once, upon a stage or phase change.
    */
    async fn state_change(&mut self, state: TimerMachineState) {
        use TimerMachineState::*;
        let final_state = match state {
            FinishedPhase(ref phase) if phase.clone().next().is_none() => Finished,
            _ => state,
        };
        let reason = format!("Switching from state {} to {}", self.state, final_state);
        self.abort_tasks(reason).await;
        if let OnPhase(phase) = &final_state {
            self.start_tasks(phase).await;
        }
        self.state = final_state;
    }

    /**
     * tick, in comparison to state_change, runs perpetually and is used for
     * checking to see if conditions for a next phase are met
     */
    pub async fn tick(&mut self, pos: Position) {
        // It is always important to check if we have met the conditions for resetting the timer
        self.reset_check(pos).await;

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
                if trigger.check(pos, self.combat_state, self.key_pressed) {
                    if let Some(phase) = TimerFilePhase::new(self.timer.clone()) {
                        self.state_change(OnPhase(phase)).await;
                    }
                }
            }
            // within a phase (nth)
            OnPhase(phase) => {
                // handle the finish check
                if let Some(trigger) = &phase.finish {
                    if trigger.check(pos, self.combat_state, self.key_pressed) {
                        self.state_change(FinishedPhase(phase.clone())).await;
                    }
                }
            }
            FinishedPhase(phase) => {
                // check the next phase's start trigger
                if let Some(next_phase) = &phase.clone().next() {
                    let trigger = &next_phase.start;
                    if trigger.check(pos, self.combat_state, self.key_pressed) {
                        self.state_change(OnPhase(next_phase.clone())).await;
                    }
                }
            }
            Finished => (),
        }
    }

    pub fn key_pressed(&mut self, id: String) {
        log::info!("{} was pressed!", id);
        self.key_pressed = true;
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
            log::info!(
                "On map with ID \"{}\" for \"{}\"",
                map_id,
                self.timer.name()
            );
            self.state = TimerMachineState::OnMap;
        } else {
            log::info!(
                "Off map with ID \"{}\" for \"{}\"",
                map_id,
                self.timer.name()
            );
            self.state = TimerMachineState::OffMap;
        }
    }
}
