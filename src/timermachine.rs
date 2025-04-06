use {
    crate::{
        bhtimer::{
            TimerFile,
            TimerTriggerType,
            TimerTrigger,
            TimerAlert,
            TimerPhase,
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

pub type PhaseState = (usize, String);

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
    Finished(usize),
    FullFinish,
}

#[derive(Debug, Clone)]
enum TriggerType {
    Start,
    Finish,
}

#[derive(Debug, Clone)]
pub struct TimerMachine {
    state: TimerMachineState,
    timer_file: Arc<TimerFile>,
    phase_id: usize,
    alert_sem: Arc<Mutex<()>>,
    sender: Sender<RenderThreadEvent>,
    combat_state: CombatState,
    tasks: Vec<Arc<JoinHandle<()>>>,
}

#[derive(Debug, Clone, PartialEq)]
enum CombatState {
    Outside,
    Entered,
    Exited,
}

impl TimerMachine {
    pub fn new(timer_file: Arc<TimerFile>, alert_sem: Arc<Mutex<()>>, sender: Sender<RenderThreadEvent>) -> Self {
        TimerMachine {
            state: TimerMachineState::AwakeUnaware,
            phase_id: 0,
            timer_file,
            alert_sem,
            sender,
            combat_state: CombatState::Outside,
            tasks: Default::default(),
        }
    }
    pub fn get_trigger(&self, id: usize, trigger: TriggerType) -> Option<TimerTrigger> {
        let tf_ref = self.timer_file.clone();
        use TriggerType::*;
        match trigger {
            Start => Some(tf_ref.phases[id].start.clone()),
            Finish => {
                if let Some(finish_phase) = &tf_ref.phases[id].finish.clone() {
                    Some(finish_phase.clone())
                } else { None }
            },
        }
    }
    pub async fn alert(ts: f32, alert: TimerAlert) {
        let ts_d = Duration::from_secs_f32(ts);
        sleep(ts_d);
        if let Some(warning) = alert.warning {
            log::info!("{} Start@{}", warning, ts);
            if let Some(dur) = alert.warning_duration {
                log::info!("{} Warning Duration Start@{}", warning, ts);
                let dur_d = Duration::from_secs_f32(dur);
                sleep(dur_d);
                log::info!("{} Warning Duration End@{}", warning, ts);
            };
        }
        if let Some(alert_msg) = alert.alert {
            if let Some(dur) = alert.alert_duration {
                log::info!("{} Alert Duration Start@{}", alert_msg, ts);
                let dur_d = Duration::from_secs_f32(dur);
                sleep(dur_d);
                log::info!("{} Alert Duration End@{}", alert_msg, ts);
            };
        };
    }

    fn spawn_alert(&self, ts: f32, alert: TimerAlert) -> JoinHandle<()> {
        tokio::spawn(Self::alert(ts, alert))
    }

    pub fn tick(&mut self, pos: Position) {
        let tf_ref = self.timer_file.clone();
        let tf_reset = tf_ref.reset.clone();
        // TODO: create reset trigger

        log::info!("Current state: {:?}", self.state);
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
                log::info!("On map for {}", tf_ref.name);
                let trigger = self.get_trigger(self.phase_id, TriggerType::Start).unwrap();
                use TimerTriggerType::*;
                match &trigger.kind {
                    Location => {
                        let shape = trigger.polytope().unwrap();
                        if trigger.require_entry && shape.point_is_within(pos) {
                            if trigger.require_combat || true {
                                if self.combat_state == CombatState::Entered {
                                    self.state = TimerMachineState::OnPhase(self.phase_id);
                                }
                            } else {
                                self.state = TimerMachineState::OnPhase(self.phase_id);
                            }
                        }
                    },
                    // Go home clown
                    Key => (),
                }
            },
            // within a phase (nth)
            OnPhase(phase) => {
                // let's handle the alerts!
                let tf_ref = self.timer_file.clone();
                log::info!("On phase {} for {}", phase, tf_ref.name);
                let phase_proper = tf_ref.phases[*phase].clone();
                let alerts = phase_proper.alerts.clone();
                for alert in alerts {
                    let alert_c = alert.clone();
                    if let Some(timestamps) = alert_c.timestamps {
                        for timestamp in timestamps {
                            let join = self.spawn_alert(timestamp, alert.clone());
                            let jarc = Arc::new(join);
                            self.tasks.push(jarc);
                        }
                    }
                }
                // handle the finish check
                if let Some(trigger) = self.get_trigger(*phase, TriggerType::Finish) {
                    use TimerTriggerType::*;
                    match &trigger.kind {
                        Location => {
                            let shape = trigger.polytope().unwrap();
                            if trigger.require_departure && !shape.point_is_within(pos) {
                                    if self.combat_state == CombatState::Exited {
                                        self.state = TimerMachineState::Finished(self.phase_id);
                                    }
                            } else {
                                    self.state = TimerMachineState::Finished(self.phase_id);
                                }
                        },
                        Key => ()
                    }
                    self.combat_state = CombatState::Outside;
                }
            },
            // We're cooked, son.
            Finished(phase) => {
                let tf_ref = self.timer_file.clone();
                log::info!("Finished phase {} for {}", phase, tf_ref.name);
                // is there a next phase?
                if tf_ref.phases.len() < (phase + 2) {
                    // go to it
                    self.state = TimerMachineState::OnPhase(phase + 1)
                } else {
                    // otherwise, we're DONE DONE
                    self.state = TimerMachineState::FullFinish;
                }
            },
            FullFinish => {
            },
        }
    }

    pub fn combat_entered(&mut self) {
        self.combat_state = CombatState::Entered;
    }

    pub fn combat_exited(&mut self) {
        self.combat_state = CombatState::Exited;
    }

    pub fn update_on_map(&mut self, map_id: u32) {
        let machine_map_id = self.timer_file.clone().map_id;
        if machine_map_id == map_id {
            log::info!("I'm on the map now!");
            self.state = TimerMachineState::OnMap;
        } else {
            log::info!("I'm off the map now!");
            self.state = TimerMachineState::OffMap;
        }
    }
}
