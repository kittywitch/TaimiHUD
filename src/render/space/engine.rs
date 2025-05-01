use {
    super::{
        dx11::{InstanceBufferData, RenderBackend},
        object::{ObjectBacking, ObjectLoader},
    },
    crate::{render::space::resources::ObjFile, timer::{PhaseState, TimerFile, TimerMarker}},
    anyhow::anyhow,
    bevy_ecs::prelude::*,
    glam::{Mat4, Vec3},
    itertools::Itertools,
    nexus::{imgui::Ui, paths::get_addon_dir},
    tokio::sync::mpsc::Receiver,
    std::{collections::HashMap, path::PathBuf, sync::Arc}, tokio::time::Instant,
};

#[derive(Component)]
struct Render {
    backing: Arc<ObjectBacking>,
}
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Rotation(Vec3);

#[derive(Clone)]
pub enum RotationType {
    Rotation(Vec3),
    Billboard,
}

struct Marker {
}

struct CreateMarker {
    marker: TimerMarker,
    start: Instant,
}

#[derive(Event)]
enum EngineEvent {
    CreateMarker,
}

#[derive(Bundle)]
struct Space {
    position: Position,
    rotation: Rotation,
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

pub struct Engine {
    receiver: Receiver<SpaceEvent>,
    addon_dir: PathBuf,
    pub render_backend: RenderBackend,
    object_kinds: HashMap<String, Arc<ObjectBacking>>,
    phase_states: Vec<PhaseState>,

    // ECS stuff
    world: World,
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

        let schedule = Schedule::default();

        let mut engine = Engine {
            receiver,
            addon_dir,
            render_backend,
            object_kinds,
            world,
            phase_states: Default::default(),
        };

        if let Some(backing) = engine.object_kinds.get("Cat") {
            engine.world.spawn((
                Position(Vec3::new(0.0, 130.0, 0.0)),
                Render {
                    backing: backing.clone(),
                },
            ));
        }
        Ok(engine)
    }

    pub fn new_phase(&mut self, phase_state: PhaseState) {
        self.phase_states.push(phase_state);
    }
    pub fn remove_phase(&mut self, timer: Arc<TimerFile>) {
        self.phase_states.retain(|p| !Arc::ptr_eq(&p.timer, &timer));
    }
    pub fn reset_phases(&mut self) {
        self.phase_states.clear();
    }

    pub fn process_event(&mut self) {
        match self.receiver.try_recv() {
            Ok(event) => {
                use SpaceEvent::*;
                match event {
                    MarkerFeed(phase_state) =>
                        self.new_phase(phase_state),
                    MarkerReset(timer) =>
                        self.remove_phase(timer),
                }
            },
            Err(_error) => (),
        }
    }

    pub fn render(&mut self, ui: &Ui) -> anyhow::Result<()> {
        let display_size = ui.io().display_size;
        self.process_event();
        let backend = &mut self.render_backend;
        backend.prepare(&display_size);
        let device_context =
            unsafe { backend.device.GetImmediateContext() }.expect("I lost my context!");
        let slot = 0;
        backend.perspective_handler.set(&device_context, slot);
        backend.depth_handler.setup(&device_context);
        let mut query = self.world.query::<(&mut Render, &Position)>();
        for (k, c) in &query
            .iter(&self.world)
            .chunk_by(|(r, _p)| r.backing.name.clone())
        {
            let mut itery = c.into_iter();
            let slice = itery.next().ok_or(anyhow!("empty slice!"))?;
            let (r, _p) = slice;
            let ibd: Vec<_> = vec![slice]
                .into_iter()
                .chain(itery)
                .map(|(_r, p)| {
                    let affy = r.backing.render.metadata.model_matrix * Mat4::from_translation(p.0);
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
        Ok(())
    }
}
