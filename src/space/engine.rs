use {
    super::{
        dx11::{perspective_input_data::PERSPECTIVEINPUTDATA, InstanceBufferData, RenderBackend},
        object::{ObjectBacking, ObjectLoader},
        pack::Pack,
        render_list::{MapFrustum, RenderList},
    },
    crate::{
        marker::atomic::MarkerInputData,
        space::{
            pack::{loader::DirectoryLoader, trail::ActiveTrail},
            resources::ObjFile,
        },
        timer::{PhaseState, RotationType, TimerFile, TimerMarker},
    },
    anyhow::anyhow,
    bevy_ecs::prelude::*,
    glam::{Mat4, Vec3, Vec3Swizzles},
    itertools::Itertools,
    nexus::{imgui::Ui, paths::get_addon_dir},
    std::{collections::HashMap, path::PathBuf, sync::Arc},
    tokio::{sync::mpsc::Receiver, time::Instant},
};

#[derive(Component)]
struct Render {
    disabled: bool,
    backing: Arc<ObjectBacking>,
    rotation: RotationType,
}
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
#[allow(unused)]
struct Marker {
    phase: Arc<PhaseState>,
    start: Instant,
    marker: TimerMarker,
}

#[derive(Bundle)]
struct MarkerBundle {
    position: Position,
    render: Render,
}

pub enum SpaceEvent {
    MarkerFeed(PhaseState),
    MarkerReset(Arc<TimerFile>),
}

fn handle_marker_timings(mut commands: Commands, mut query: Query<(Entity, &Marker, &mut Render)>) {
    let now = Instant::now();
    for (entity, marker, mut render) in &mut query {
        if now > marker.marker.end(marker.start) {
            log::info!(
                "Entity {} reached end after {}, despawning.",
                entity,
                marker.marker.duration
            );
            commands.entity(entity).despawn();
        } else if now > marker.marker.start(marker.start) && render.disabled {
            log::info!(
                "Entity {} reached start at {}!",
                entity,
                marker.marker.timestamp
            );
            render.disabled = false;
        }
    }
}

pub struct Engine {
    receiver: Receiver<SpaceEvent>,
    pub render_backend: RenderBackend,
    pub model_files: HashMap<PathBuf, ObjFile>,
    pub object_kinds: HashMap<String, Arc<ObjectBacking>>,
    phase_states: Vec<Arc<PhaseState>>,
    associated_entities: HashMap<String, Vec<Entity>>,

    schedule: Schedule,

    // ECS stuff
    pub world: World,

    pub test_pack: Pack,
    test_trail: usize,
    active_test_trail: ActiveTrail,
    render_list: Option<RenderList>,
}

impl Engine {
    pub fn initialise(ui: &Ui, receiver: Receiver<SpaceEvent>) -> anyhow::Result<Engine> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");

        let render_backend = RenderBackend::setup(&addon_dir, ui.io().display_size)?;

        let models_dir = addon_dir.join("models");
        let object_descs = ObjectLoader::load_desc(&models_dir)?;
        log::debug!("{:?}", object_descs);
        let model_files = ObjFile::load(&models_dir, &object_descs)?;

        let object_kinds = object_descs.to_backings(
            &render_backend.device,
            &model_files,
            &render_backend.shaders.0,
            &render_backend.shaders.1,
        );

        let world = World::new();

        let mut schedule = Schedule::default();

        schedule.add_systems(handle_marker_timings);

        let mut test_pack = Pack::load(DirectoryLoader::new(
            addon_dir.join("pathing/tw_ALL_IN_ONE"),
        ))?;
        const TEST_TRAIL: &str = "tw_guides.tw_mc_soto.tw_mc_soto_trails.tw_mc_soto_trails_thewizardstower.tw_mc_soto_trails_thewizardstower_toggletrail";
        let test_trail = test_pack
            .trails
            .iter()
            .enumerate()
            .find(|(_, trail)| trail.category == TEST_TRAIL)
            .map(|(idx, _)| idx)
            .ok_or_else(|| anyhow::anyhow!("Can't find test trail"))?;
        let active_test_trail =
            ActiveTrail::build(&mut test_pack, test_trail, &render_backend.device)?;

        let mut engine = Engine {
            model_files,
            receiver,
            render_backend,
            object_kinds,
            schedule,
            world,
            associated_entities: Default::default(),
            phase_states: Default::default(),
            test_pack,
            test_trail,
            active_test_trail,
            render_list: None,
        };

        if let Some(backing) = engine.object_kinds.get("Cat") {
            engine.world.spawn((
                Position(Vec3::new(0.0, 130.0, 0.0)),
                Render {
                    disabled: false,
                    backing: backing.clone(),
                    rotation: RotationType::Rotation(Vec3::ZERO),
                },
            ));
        }
        Ok(engine)
    }

    pub fn new_phase(&mut self, phase_state: PhaseState) -> anyhow::Result<()> {
        let phase_state = Arc::new(phase_state);
        let markers = &phase_state.markers;
        let entry = self
            .associated_entities
            .entry(phase_state.timer.name.clone())
            .or_default();
        for marker in markers {
            if let Some(base_path) = &phase_state.timer.path {
                let backing = Arc::new(ObjectBacking::create_marker(
                    &self.render_backend,
                    marker,
                    base_path.clone(),
                )?);
                let entity = self.world.spawn((
                    Position(marker.position),
                    Marker {
                        phase: phase_state.clone(),
                        start: phase_state.start,
                        marker: marker.clone(),
                    },
                    Render {
                        rotation: marker.kind.clone(),
                        disabled: true,
                        backing,
                    },
                ));
                let id = entity.id();
                log::debug!(
                    "Creating entity {id} at {} from timer {} markers, phase {}",
                    marker.position,
                    phase_state.timer.name(),
                    phase_state.phase.name
                );
                entry.push(id);
            }
        }
        self.phase_states.push(phase_state);
        Ok(())
    }
    pub fn remove_phase(&mut self, timer: Arc<TimerFile>) -> anyhow::Result<()> {
        if let Some(entry) = self.associated_entities.remove(&timer.name.clone()) {
            entry.iter().for_each(|entity| {
                log::debug!("Despawning {entity} from timer {} markers", timer.name());
                self.world.despawn(*entity);
            });
        }
        self.phase_states.retain(|p| !Arc::ptr_eq(&p.timer, &timer));
        Ok(())
    }
    #[allow(dead_code)]
    pub fn reset_phases(&mut self) {
        for entities in self.associated_entities.values() {
            for entity in entities {
                self.world.despawn(*entity);
            }
        }
        self.associated_entities.clear();
        self.phase_states.clear();
    }

    pub fn process_event(&mut self) -> anyhow::Result<()> {
        match self.receiver.try_recv() {
            Ok(event) => {
                use SpaceEvent::*;
                match event {
                    MarkerFeed(phase_state) => self.new_phase(phase_state)?,
                    MarkerReset(timer) => self.remove_phase(timer)?,
                }
            }
            Err(_error) => (),
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn check_phase_ends() {
        todo!("this is supposed to terminate a phase when there are no more markers, ideally we should actually make something that finds the latest timestamp between sounds, directions, markers, alerts etc");
    }

    pub fn render(&mut self, ui: &Ui) -> anyhow::Result<()> {
        let display_size = ui.io().display_size;
        self.process_event()?;
        self.schedule.run(&mut self.world);
        let backend = &mut self.render_backend;
        backend.prepare(&display_size);
        let device_context =
            unsafe { backend.device.GetImmediateContext() }.expect("I lost my context!");
        let slot = 0;
        backend.perspective_handler.set(&device_context, slot);
        backend.depth_handler.setup(&device_context);
        backend.blending_handler.set(&device_context);
        let pdata = PERSPECTIVEINPUTDATA.get().unwrap().load();
        let mut query = self.world.query::<(&mut Render, &Position)>();
        for (_k, c) in &query
            .iter(&self.world)
            .chunk_by(|(r, _p)| r.backing.name.clone())
        {
            let mut itery = c.into_iter();
            let slice = itery.next().ok_or(anyhow!("empty slice!"))?;
            let (r, p) = slice;
            if !r.disabled {
                let rot = match r.rotation {
                    RotationType::Billboard => {
                        let mark2d = (p.0.xz() - pdata.pos.xz()).to_angle();
                        let y = Mat4::from_rotation_y(-90.0f32.to_radians() - mark2d);
                        y
                        //Mat4::IDENTITY
                    }
                    _ => Mat4::IDENTITY,
                };
                let ibd: Vec<_> = vec![slice]
                    .into_iter()
                    .chain(itery)
                    .map(|(_r, p)| {
                        //  r.backing.render.metadata.model_matrix *
                        let affy = Mat4::from_translation(p.0)
                            * rot
                            * r.backing.render.metadata.model_matrix;
                        InstanceBufferData {
                            world: affy,
                            //world_position: affy.translation,
                            colour: Vec3::new(1.0, 1.0, 1.0),
                        }
                    })
                    .collect();
                r.backing
                    .set_and_draw(slot, &backend.device, &device_context, &ibd)?;
            }
        }
        if let Some(render_list) = &mut self.render_list {
            let frustum = MapFrustum::from_camera_data(
                &pdata,
                display_size[0] / display_size[1],
                0.1,
                1000.0,
            );
            let cam_origin = pdata.pos.into();
            let cam_dir = pdata.front.into();
            for entity in render_list.get_entities_for_drawing(cam_origin, cam_dir, &frustum) {
                // TODO: Draw.
            }
        }
        let mid = MarkerInputData::read().unwrap();
        if mid.map_id as i32 == self.test_pack.trails[self.test_trail].data.map_id {
            backend.shaders.0["trail"].set(&device_context);
            backend.shaders.1["trail"].set(&device_context);
            for i in 0..self.active_test_trail.section_bounds.len() {
                self.active_test_trail.draw_section(&device_context, i);
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn cleanup(&self) {
        todo!("Please clean up the engine when the program quits");
    }
}
