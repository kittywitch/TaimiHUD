use {
    super::{
        model::Model,
        vertexbuffer::VertexBuffer,
        state::InstanceBufferData,
    },
    anyhow::anyhow,
    glam::{Affine3A, Mat4, Vec2, Vec3},
    glob::Paths,
    itertools::Itertools,
    rand::Rng,
    relative_path::RelativePathBuf,
    serde::{Deserialize, Serialize},
    std::{
        cell::RefCell,
        collections::HashMap,
        fs::read_to_string,
        iter,
        path::{Path, PathBuf},
        rc::Rc,
        slice::from_ref,
    },
    tobj::{Material, Mesh},
    windows::Win32::Graphics::Direct3D11::{
        ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_SHADER_RESOURCE, D3D11_BIND_UNORDERED_ACCESS, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_CPU_ACCESS_WRITE, D3D11_RESOURCE_MISC_BUFFER_STRUCTURED, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC
    },
};
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: Vec3,
    pub normal: Vec3,
    pub texture: Vec2,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ModelLocation {
    file: PathBuf,
    index: usize,
}

type Eda = Rc<EntityDescription>;

#[derive(Default)]
pub struct EntityController(HashMap<PathBuf, Vec<Eda>>);

impl EntityController {
    pub fn load_desc(addon_dir: &Path) -> anyhow::Result<Self> {
        let mut entity_controller = Self::default();
        let model_folder = addon_dir.join("models");

        if model_folder.exists() {
            let entity_desc_paths: Paths = glob::glob(
                model_folder
                    .join("*.entitydesc")
                    .to_str()
                    .expect("Model load pattern is unparseable"),
            )?;
            for entity_desc_path in entity_desc_paths {
                let entity_desc_path = entity_desc_path?;
                log::info!("Loading entities from {:?}", entity_desc_path);
                let entity_descs = EntityDescription::load(&entity_desc_path)?;
                for entity_desc in entity_descs.into_iter() {
                    let full_path = model_folder.join(&entity_desc.location.file);
                    let entry = entity_controller.0.entry(full_path).or_default();
                    let entity_desc_arc = Rc::new(entity_desc);
                    entry.push(entity_desc_arc);
                }
            }
        }
        Ok(entity_controller)
    }

    pub fn load(self, device: &ID3D11Device) -> anyhow::Result<Vec<Rc<Entity>>> {
        let mut entities = Vec::new();
        for (file, descs) in &self.0 {
            let file_models = Model::load(file)?;
            for desc in descs {
                let model_idx = desc.location.index;
                let model = file_models.get(model_idx).ok_or_else(|| {
                    anyhow!(
                        "model index {} does not exist in file {:?}",
                        model_idx,
                        file
                    )
                })?;
                log::info!(
                    "Loading entity \"{}\" from \"{:?}\"@{}",
                    desc.name,
                    desc.location.file,
                    desc.location.index
                );
                let vertex_buffer = model.to_buffer(device)?;
                let vertex_buffer_rc = Rc::new(vertex_buffer);
                let mut rng = rand::rng();
                let model_matrix: Vec<_> = (0..1000 * 3)
                    .map(|_| rng.random::<f32>() * 1000.0)
                    .chunks(3)
                    .into_iter()
                    .map(|xyz| Vec3::from_slice(&xyz.into_iter().collect::<Vec<_>>()))
                    .map(Mat4::from_translation)
                    .map(|trans| InstanceBufferData {
                        model: desc.model_matrix * trans,
                    })
                    .collect();

                let instance_buffer = Entity::setup_instance_buffer(&model_matrix, device)?;
                let entity = Entity {
                    name: desc.name.clone(),
                    model: model.clone(),
                    model_matrix: RefCell::new(model_matrix),
                    location: desc.location.clone(),
                    pixel_shader: desc.pixel_shader.clone(),
                    vertex_shader: desc.vertex_shader.clone(),
                    vertex_buffer: vertex_buffer_rc.clone(),
                    instance_buffer,
                };
                let entity = Rc::new(entity);
                entities.push(entity);
            }
        }
        log::info!("Entities successfully loaded!");
        Ok(entities)
    }
}

fn default_pixel_shader() -> String {
    "generic_ps".to_string()
}
fn default_vertex_shader() -> String {
    "generic_vs".to_string()
}
#[derive(Clone, Serialize, Deserialize)]
pub struct EntityDescription {
    pub name: String,
    pub location: ModelLocation,
    #[serde(default = "default_vertex_shader")]
    pub vertex_shader: String,
    #[serde(default = "default_pixel_shader")]
    pub pixel_shader: String,
    #[serde(default)]
    pub model_matrix: Mat4,
}

pub struct Entity {
    pub name: String,
    pub model_matrix: RefCell<Vec<InstanceBufferData>>,
    pub location: ModelLocation,
    pub model: Rc<Model>,
    pub vertex_buffer: Rc<VertexBuffer>,
    pub vertex_shader: String,
    pub pixel_shader: String,
    pub instance_buffer: ID3D11Buffer,
}

impl Entity {
    pub fn set(&self, slot: u32, device_context: &ID3D11DeviceContext) {
        let instance_buffer_stride = size_of::<InstanceBufferData>() as u32;
        let instance_buffer_offset = 0_u32;
        let buffers = [Some(self.vertex_buffer.buffer.clone()), Some(self.instance_buffer.clone())];
        let strides = [self.vertex_buffer.stride, instance_buffer_stride];
        let offsets = [self.vertex_buffer.offset, instance_buffer_offset];
        unsafe {
            device_context.IASetVertexBuffers(
                slot,
                2,
                Some(buffers.as_ptr() as *const _),
                Some(strides.as_ptr()),
                Some(offsets.as_ptr()),
            );
        }
    }

    pub fn draw(&self, start: u32, device_context: &ID3D11DeviceContext) {
        let total = self.vertex_buffer.count + self.model_matrix.borrow().len() as u32;
        let instances = self.model_matrix.borrow().len();
        unsafe { device_context.DrawInstanced(total, instances as u32, start, 0) }
    }

    pub fn set_and_draw(&self, device_context: &ID3D11DeviceContext) {
        self.set(0_u32, device_context);
        //Self::set_many(bufs, 0, device_context);
        self.draw( 0, device_context);
    }

    pub fn rotate(&self, dt: f32) {
        let mut model = self.model_matrix.borrow_mut();
        for mdl in model.iter_mut() {
            mdl.rotate(dt);
        }
    }

    pub fn setup_instance_buffer(
        model_matrix: &[InstanceBufferData],
        device: &ID3D11Device,
    ) -> anyhow::Result<ID3D11Buffer> {

        /*let stride: u32 = size_of::<InstanceBufferData>() as u32;
        let offset: u32 = 0;
        let count: u32 = instance_data_array.len() as u32;*/

        let instance_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of_val(model_matrix) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: size_of::<InstanceBufferData>() as u32,
        };

        let instanced_subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: model_matrix.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let mut instance_buffer_ptr: Option<ID3D11Buffer> = None;
        let instance_buffer = unsafe {
            device.CreateBuffer(
                &instance_buffer_desc,
                Some(&instanced_subresource_data),
                Some(&mut instance_buffer_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| {
            instance_buffer_ptr.ok_or_else(|| anyhow!("no per-entity structured buffer"))
        })?;

        Ok(instance_buffer)
    }
}

impl EntityDescription {
    fn load(path: &PathBuf) -> anyhow::Result<Vec<Self>> {
        log::debug!("Attempting to load the entity description file at \"{path:?}\".");
        let mut file_data = read_to_string(path)?;
        json_strip_comments::strip(&mut file_data)?;
        let entity_description_data: Vec<Self> = serde_json::from_str(&file_data)?;
        Ok(entity_description_data)
    }
    /*fn load_model(model_folder: &Path, device: ID3D11Device) -> anyhow::Result<Vec<Entity>> {
        Model::load_to_buffers(device, obj_file)

    }*/
}
