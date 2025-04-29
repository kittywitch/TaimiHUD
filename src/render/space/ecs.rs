use {
    super::{entitycontroller::ObjectLoader, entitydescription::EntityDescription, model::{MaterialTextures, Model, ObjModelFile}, primitivetopology::PrimitiveTopology, shader::{PixelShader, VertexShader}, state::{InstanceBufferData, RenderBackend}, texture::Texture, vertexbuffer::VertexBuffer}, anyhow::anyhow, bevy_ecs::{prelude::*, system::{StaticSystemInput, SystemId}}, bevy_utils::synccell::SyncCell, glam::{Affine3A, Mat4, Vec3}, itertools::Itertools, nexus::{imgui::Ui, paths::get_addon_dir}, std::{cell::RefCell, collections::HashMap, mem, path::{Path, PathBuf}, rc::Rc, sync::{Arc, Mutex, RwLock}}, windows::Win32::Graphics::Direct3D11::{ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT}
};

pub struct InstanceBuffer {
    buffer: ID3D11Buffer,
    count: usize,
}

impl InstanceBuffer {
    pub fn create_empty(device: &ID3D11Device) -> anyhow::Result<Self> {
        let count = 0;

        let desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of::<InstanceBufferData>() as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: size_of::<InstanceBufferData>() as u32,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA::default();

        let mut ptr: Option<ID3D11Buffer> = None;
        let buffer = unsafe {
            device.CreateBuffer(
                &desc,
                Some(&subresource_data),
                Some(&mut ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| {
            ptr.ok_or_else(|| anyhow!("no per-entity structured buffer"))
        })?;

        Ok(Self {
            buffer,
            count,

        })
    }

    pub fn create(device: &ID3D11Device, data: &[InstanceBufferData]) -> anyhow::Result<Self> {
        let count = data.len();

        let desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of_val(data) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: size_of::<InstanceBufferData>() as u32,
        };

        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: data.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let mut ptr: Option<ID3D11Buffer> = None;
        let buffer = unsafe {
            device.CreateBuffer(
                &desc,
                Some(&subresource_data),
                Some(&mut ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| {
            ptr.ok_or_else(|| anyhow!("no per-entity structured buffer"))
        })?;

        Ok(Self {
            buffer,
            count,

        })
    }
    pub fn update(&mut self, device: &ID3D11Device, device_context: &ID3D11DeviceContext, data: &[InstanceBufferData]) -> anyhow::Result<()> {
        if data.len() == self.count {
            unsafe {
                device_context.UpdateSubresource(
                    &self.buffer,
                    0,
                    None,
                    data.as_ptr().cast(),
                    0,
                    0,
                );
            }
        } else {
            *self = Self::create(device, data)?;
        }
        Ok(())
    }
}

pub struct ObjectBacking {
    pub name: String,
    pub render: ObjectRenderBacking,
}

impl ObjectBacking {
}

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
            Some(lock.buffer.clone()),
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
        let instances = lock.count;
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

impl ObjectRenderBacking {
}

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
    pub fn load_models(models_dir: &Path, object_descs: &ObjectLoader) -> anyhow::Result<HashMap<PathBuf, ObjModelFile>> {
        let mut model_files: HashMap<PathBuf, ObjModelFile> = Default::default();
        let model_filenames: Vec<PathBuf> = object_descs
            .0.iter()
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
            .0.iter()
            .flat_map(|(_f, o)| o)
            .filter_map(|o| o.to_backing(
                &model_files,
                &render_backend.device,
                &render_backend.shaders.0,
                &render_backend.shaders.1
            ).ok())
            .map(|o| {
                let name = o.name.clone();
                let oarc = Arc::new(o);
                log::info!("Entity {} loaded!", name);
                (name, oarc)
            })
            .collect();

        let mut world = World::new();

        let mut schedule = Schedule::default();


        let mut engine = Engine {
            addon_dir,
            render_backend,
            object_kinds,
            world,
        };

        log::info!("whee befwwow");
        for (name, obj) in &engine.object_kinds {
            log::info!("whee {name}");
        }
        log::info!("whee awfter");

        if let Some(backing) = engine.object_kinds.get("Cat") {
            engine.world.spawn(
                (
                    Position(Vec3::new(0.0, 130.0, 0.0)),
                    Render {
                        backing: backing.clone(),
                    }
                )
            );
        } else {
            log::info!("Couldn't find cat :(");
        }


        Ok(engine)
    }

    pub fn render(&mut self, ui: &Ui) -> anyhow::Result<()> {
        let display_size = ui.io().display_size;
            let backend = &mut self.render_backend;
            backend.prepare(&display_size);
            let device_context = unsafe { backend.device
                .GetImmediateContext() }.expect("I lost my context!");
            let slot = 0;
            log::debug!("I AM RENDERING! WAEOW");
            backend.perspective_handler.set(&device_context, slot);
            backend.depth_handler.setup(&device_context);
            let mut query = self.world.query::<(&mut Render, &Position)>();
            for (k, c) in &query.iter(&self.world)
                .chunk_by(|(r, _p)| r.backing.name.clone()) {
                    let mut itery = c.into_iter();
                    let slice = itery.next().ok_or(anyhow!("empty slice!"))?;
                    let (r, _p) = slice;
                    let ibd: Vec<_> = vec![slice].into_iter().chain(itery).map(|(_r, p)| {
                        let affy = r.backing.render.metadata.model_matrix * Mat4::from_translation(p.0);
                        InstanceBufferData {
                            world: affy,
                            //world_position: affy.translation,
                            colour: Vec3::new(1.0, 1.0, 1.0),
                        }}).collect();
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
