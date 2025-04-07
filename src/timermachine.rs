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
    reset: TimerTrigger,
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
            reset: timer_file.clone().reset.clone(),
            timer_file,
            alert_sem,
            sender,
            combat_state: CombatState::Outside,
            tasks: Default::default(),
        }
    }
    pub fn get_trigger(&mut self, id: usize, trigger: TriggerType) -> Option<TimerTrigger> {
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
    
    async fn send_alert(
        sender: Sender<RenderThreadEvent>,
        lock: Arc<Mutex<()>>,
        message: String,
        sleeb: Duration,
        duration: Duration,
    ) {
        let sleeb_real = sleeb - duration;
        log::info!("Sleeping {:?} seconds for {}: a message with {:?} duration", sleeb, message, duration);
        sleep(sleeb_real).await;
        let alert_handle = lock.lock().await;
        log::info!("Slept {:?} seconds, displaying {}: a message with {:?} duration", sleeb, message, duration);
        let _ = sender.send(RenderThreadEvent::AlertStart(message.clone())).await;
        sleep(duration).await;
        let _ = sender.send(RenderThreadEvent::AlertEnd).await;
        log::info!("Stopping displaying {}: we slept for {:?} a message with {:?} duration", message, sleeb, duration);
        // this is my EMOTIONAL SUPPORT drop
        drop(alert_handle);
    }

    fn text_alert(&self, message: String, duration: Duration) -> JoinHandle<()> {
        tokio::spawn(Self::send_alert(
            self.sender.clone(),
            self.alert_sem.clone(),
            message,
            duration,
            Duration::from_secs(1),
        ))
    }

    fn ambiguous_alert(&mut self, ts: f32, alert: TimerAlert) {
        let ts_d = Duration::from_secs_f32(ts);
        if let Some(warn) = alert.warning {
                if let Some(warn_dur) = alert.warning_duration {
                    let warn_dur_d = Duration::from_secs_f32(warn_dur);
                    let message = format!("{} in {} seconds", warn, warn_dur);
                    if let Some(dury) = ts_d.checked_sub(warn_dur_d) {
                        let join = self.text_alert(message, dury);
                        let jarc = Arc::new(join);
                        self.tasks.push(jarc);
                    } else {
                        let join = self.text_alert(message, ts_d);
                        let jarc = Arc::new(join);
                        self.tasks.push(jarc);
                    }
                }
        } else {
            if let Some(alrt) = alert.alert {
                if let Some(alrt_dur) = alert.alert_duration {
                    let alrt_dur_d = Duration::from_secs_f32(alrt_dur);
                    let message = format!("{} in {} seconds", alrt, alrt_dur);
                    if let Some(dury) = ts_d.checked_sub(alrt_dur_d) {
                        let join = self.text_alert(message, dury);
                        let jarc = Arc::new(join);
                        self.tasks.push(jarc);
                    } else {
                        let join = self.text_alert(message, ts_d);
                        let jarc = Arc::new(join);
                        self.tasks.push(jarc);
                    }
                }
            }
        }
    }


    pub async fn alert(ts: f32, alert: TimerAlert) {
        let ts_d = Duration::from_secs_f32(ts);
        if let Some(warning) = alert.warning {
            log::info!("Sleeping {} for {}", ts, warning);
            sleep(ts_d).await;
            log::info!("{} Start@{}", warning, ts);
            if let Some(dur) = alert.warning_duration {
                log::info!("{} Warning Duration Start@{}, Length: {}", warning, ts, dur);
                let dur_d = Duration::from_secs_f32(dur);
                sleep(dur_d).await;
                log::info!("{} Warning Duration End@{}, Length: {}", warning, ts, dur);
            };
        }
        if let Some(alert_msg) = alert.alert {
            log::info!("Sleeping {} for {}", ts, alert_msg);
            sleep(ts_d).await;
            if let Some(dur) = alert.alert_duration {
                log::info!("{} Alert Duration Start@{}, Length: {}", alert_msg, ts, dur);
                let dur_d = Duration::from_secs_f32(dur);
                sleep(dur_d).await;
                log::info!("{} Alert Duration End@{}, Length: {}", alert_msg, ts, dur);
            };
        };
    }

    fn spawn_alert(&self, ts: f32, alert: TimerAlert) -> JoinHandle<()> {
        tokio::spawn(Self::alert(ts, alert))
    }

    fn start_phase(&mut self, phase: usize) {
        let tf_ref = self.timer_file.clone();
        log::info!("On phase {} for {}", phase, tf_ref.name);
        let phase_proper = tf_ref.phases[phase].clone();
        let alerts = phase_proper.alerts.clone();
        for alert in alerts {
            let alert_c = alert.clone();
            if let Some(timestamps) = alert_c.timestamps {
                for timestamp in timestamps {
                    self.ambiguous_alert(timestamp, alert.clone());
                }
            }
        }
        self.state = TimerMachineState::OnPhase(phase);
    }

    fn reset_check(&mut self, pos: Position) {
        let trigger = &self.reset;
        let shape = trigger.polytope().unwrap();
        use TimerMachineState::*;
        match &self.state {
            OnPhase(_) => {
                if trigger.require_entry && shape.point_is_within(pos) {
                        log::info!("{:?}",self.combat_state);
                    if trigger.require_out_of_combat {
                        if self.combat_state == CombatState::Exited {
                            self.do_reset();
                        }
                    } else {
                        self.do_reset();
                    }
                }
            },
            Finished(_) => { 
                if trigger.require_entry && shape.point_is_within(pos) {
                    if trigger.require_out_of_combat {
                        if self.combat_state == CombatState::Exited {
                            self.do_reset();
                        }
                    } else {
                        self.do_reset();
                    }
                }
            },
            _ => (),
        }
    }

    fn do_reset(&mut self) {
        log::info!("Reset triggered!");
        self.state = TimerMachineState::OnMap;
        for task in &self.tasks {
            log::info!("Aborting task for reset: {:?}", task.id());
            task.abort()
        }
        let alert_handle = self.alert_sem.lock();
        let _ = self.sender.send(RenderThreadEvent::AlertEnd);
        self.combat_state = CombatState::Outside;
        let zero_s = Duration::from_secs(0);
        let one_s = Duration::from_secs(4);
        self.text_alert("Reset triggered!".to_string(), one_s);
    }

    pub fn tick(&mut self, pos: Position) {
        let tf_ref = self.timer_file.clone();

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
                let trigger = self.get_trigger(self.phase_id, TriggerType::Start).unwrap();
                use TimerTriggerType::*;
                match &trigger.kind {
                    Location => {
                        let shape = trigger.polytope().unwrap();
                        if trigger.require_entry && shape.point_is_within(pos) {
                            if trigger.require_combat {
                                if self.combat_state == CombatState::Entered {
                                    self.start_phase(self.phase_id);
                                } 
                            } else {
                                self.start_phase(self.phase_id);
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
                                if trigger.require_entry && shape.point_is_within(pos) {
                                    if trigger.require_out_of_combat && self.combat_state == CombatState::Exited {
                                        self.state = TimerMachineState::Finished(self.phase_id);
                                    }
                                }
                            }
                        },
                        Key => ()
                    }
                }
            },
            // We're cooked, son.
            Finished(phase) => {
                log::info!("Finished phase {} for {}", phase, tf_ref.name);
                // is there a next phase?
                if tf_ref.phases.len() < (phase + 1) {
                    // go to it
                    self.state = TimerMachineState::OnPhase(phase + 1)
                } else {
                    // otherwise, we're DONE DONE
                    self.state = TimerMachineState::FullFinish;
                    self.combat_state = CombatState::Outside;
                }
            },
            FullFinish => {
                log::info!("damn bitch you really finished there huh");
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
