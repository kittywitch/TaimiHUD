use {
    super::{
        entitycontroller::ObjectLoader,
        model::{MaterialTextures, Model, ObjModelFile},
        primitivetopology::PrimitiveTopology,
        shader::{PixelShader, VertexShader},
        state::{InstanceBufferData, RenderBackend},
        vertexbuffer::VertexBuffer,
        instancebuffer::InstanceBuffer,
    },
    anyhow::anyhow,
    bevy_ecs::prelude::*,
    glam::{Mat4, Vec3},
    itertools::Itertools,
    nexus::{imgui::Ui, paths::get_addon_dir},
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, RwLock},
    },
    windows::Win32::Graphics::Direct3D11::ID3D11DeviceContext,
};


pub struct ObjectBacking {
    pub name: String,
    pub render: ObjectRenderBacking,
}

impl ObjectBacking {}

pub struct ObjectRenderMetadata {
    pub model: Model,
    pub material: MaterialTextures,
    pub model_matrix: Mat4,
    pub topology: PrimitiveTopology,
}

impl ObjectRenderBacking {
    pub fn set(&self, slot: u32, device_context: &ID3D11DeviceContext) {
        let instance_buffer_stride = size_of::<InstanceBufferData>() as u32;
        let instance_buffer_offset = 0_u32;
        let lock = self.instance_buffer.read().unwrap();
        let buffers = [
            Some(self.vertex_buffer.buffer.clone()),
            Some(lock.get_buffer()),
        ];
        drop(lock);
        let strides = [self.vertex_buffer.stride, instance_buffer_stride];
        let offsets = [self.vertex_buffer.offset, instance_buffer_offset];
        unsafe {
            device_context.IASetVertexBuffers(
                slot,
                2,
                Some(buffers.as_ptr().cast()),
                Some(strides.as_ptr()),
                Some(offsets.as_ptr()),
            );
        }
    }

    pub fn draw(&self, start: u32, device_context: &ID3D11DeviceContext) {
        let lock = self.instance_buffer.read().unwrap();
        let instances = lock.get_count();
        drop(lock);
        let total = self.vertex_buffer.count + instances as u32;
        unsafe {
            device_context.IASetPrimitiveTopology(self.metadata.topology.dx11());
            device_context.DrawInstanced(total, instances as u32, start, 0)
        }
    }
    pub fn set_and_draw(&self, device_context: &ID3D11DeviceContext) {
        self.set(0_u32, device_context);
        self.draw(0, device_context);
    }
}

pub struct ShaderPair(pub Arc<VertexShader>, pub Arc<PixelShader>);

impl ShaderPair {
    pub fn set(&self, device_context: &ID3D11DeviceContext) {
        self.0.set(device_context);
        self.1.set(device_context);
    }
}

pub struct ObjectRenderBacking {
    pub metadata: ObjectRenderMetadata,
    pub instance_buffer: RwLock<InstanceBuffer>,
    pub vertex_buffer: VertexBuffer,
    pub shaders: ShaderPair,
}

impl ObjectRenderBacking {}

#[derive(Component)]
struct Render {
    backing: Arc<ObjectBacking>,
}
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Rotation(Vec3);
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

pub struct Engine {
    addon_dir: PathBuf,
    render_backend: RenderBackend,
    object_kinds: HashMap<String, Arc<ObjectBacking>>,

    // ECS stuff
    world: World,
}

impl Engine {
    pub fn load_models(
        models_dir: &Path,
        object_descs: &ObjectLoader,
    ) -> anyhow::Result<HashMap<PathBuf, ObjModelFile>> {
        let mut model_files: HashMap<PathBuf, ObjModelFile> = Default::default();
        let model_filenames: Vec<PathBuf> = object_descs
            .0
            .iter()
            .flat_map(|(_f, o)| o)
            .map(|o| o.location.file.clone())
            .dedup()
            .collect();
        for model_filename in &model_filenames {
            let model_file = ObjModelFile::load_file(&models_dir.join(model_filename))?;
            model_files.insert(model_filename.to_path_buf(), model_file);
        }
        Ok(model_files)
    }

    pub fn initialise(ui: &Ui) -> anyhow::Result<Engine> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");

        let render_backend = RenderBackend::setup(&addon_dir, ui.io().display_size)?;

        let models_dir = addon_dir.join("models");
        let object_descs = ObjectLoader::load_desc(&models_dir)?;
        log::debug!("{:?}", object_descs);
        let model_files = Self::load_models(&models_dir, &object_descs)?;

        let object_kinds: HashMap<String, Arc<ObjectBacking>> = object_descs
            .0
            .iter()
            .flat_map(|(_f, o)| o)
            .filter_map(|o| {
                o.to_backing(
                    &model_files,
                    &render_backend.device,
                    &render_backend.shaders.0,
                    &render_backend.shaders.1,
                )
                .ok()
            })
            .map(|o| {
                let name = o.name.clone();
                let oarc = Arc::new(o);
                log::info!("Entity {} loaded!", name);
                (name, oarc)
            })
            .collect();

        let world = World::new();

        let schedule = Schedule::default();

        let mut engine = Engine {
            addon_dir,
            render_backend,
            object_kinds,
            world,
        };

        if let Some(backing) = engine.object_kinds.get("Cat") {
            engine.world.spawn((
                Position(Vec3::new(0.0, 130.0, 0.0)),
                Render {
                    backing: backing.clone(),
                },
            ));
        } else {
        }

        Ok(engine)
    }

    pub fn render(&mut self, ui: &Ui) -> anyhow::Result<()> {
        let display_size = ui.io().display_size;
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
            let mut lock = r.backing.render.instance_buffer.write().unwrap();
            lock.update(&backend.device, &device_context, &ibd)?;
            drop(lock);
            r.backing.render.shaders.set(&device_context);
            if let Some(diffuse) = &r.backing.render.metadata.material.diffuse {
                diffuse.texture.set(&device_context, slot);
            }
            r.backing.render.set_and_draw(&device_context);
        }
        Ok(())
    }
}
