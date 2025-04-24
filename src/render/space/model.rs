use {
    super::state::InstanceBufferData,
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
        ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_CONSTANT_BUFFER,
        D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
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

                let constant_buffer = Entity::setup_constant_buffer(&model_matrix, device)?;
                let entity = Entity {
                    name: desc.name.clone(),
                    model: model.clone(),
                    model_matrix: RefCell::new(model_matrix),
                    location: desc.location.clone(),
                    pixel_shader: desc.pixel_shader.clone(),
                    vertex_shader: desc.vertex_shader.clone(),
                    vertex_buffer: vertex_buffer_rc.clone(),
                    constant_buffer,
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
    pub constant_buffer: ID3D11Buffer,
}

impl Entity {
    pub fn set(&self, slot: u32, device_context: &ID3D11DeviceContext) {
        unsafe {
            device_context.IASetVertexBuffers(
                slot,
                1,
                Some(&self.vertex_buffer.buffer as *const _ as *const _),
                Some(&self.vertex_buffer.stride),
                Some(&self.vertex_buffer.offset),
            );
        }
    }

    pub fn draw(&self, start: u32, device_context: &ID3D11DeviceContext) {
        let total = self.vertex_buffer.count;
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

    pub fn setup_constant_buffer(
        model_matrix: &[InstanceBufferData],
        device: &ID3D11Device,
    ) -> anyhow::Result<ID3D11Buffer> {

        /*let stride: u32 = size_of::<InstanceBufferData>() as u32;
        let offset: u32 = 0;
        let count: u32 = instance_data_array.len() as u32;*/

        log::info!("Setting up vertex buffer");
        let constant_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of_val(model_matrix) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let constant_subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: model_matrix.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let mut constant_buffer_ptr: Option<ID3D11Buffer> = None;
        let constant_buffer = unsafe {
            device.CreateBuffer(
                &constant_buffer_desc,
                Some(&constant_subresource_data),
                Some(&mut constant_buffer_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| {
            constant_buffer_ptr.ok_or_else(|| anyhow!("no per-entity constant buffer"))
        })?;

        Ok(constant_buffer)
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

#[derive(Clone)]
pub struct Model {
    // todo! figure out how to store a relative path here o:
    pub name: String,
    pub file: PathBuf,
    pub index: usize,
    pub vertices: Vec<Vertex>,
}

#[derive(Clone)]
pub struct VertexBuffer {
    pub buffer: ID3D11Buffer,
    pub stride: u32,
    pub offset: u32,
    pub count: u32,
}

impl VertexBuffer {
    pub fn set(&self, slot: u32, device_context: &ID3D11DeviceContext) {
        unsafe {
            device_context.IASetVertexBuffers(
                slot,
                1,
                Some(&self.buffer as *const _ as *const _),
                Some(&self.stride),
                Some(&self.offset),
            );
        }
    }
    pub fn set_many(bufs: &[&Self], slot: u32, device_context: &ID3D11DeviceContext) {
        let buf_len = bufs.len() as u32;
        if buf_len != 0 {
            let strides: Vec<_> = bufs.iter().map(|b| b.stride).collect();
            let strides = strides.as_slice();
            let offsets: Vec<_> = bufs.iter().map(|b| b.offset).collect();
            let offsets = offsets.as_slice();
            let buffers: Vec<_> = bufs.iter().map(|b| Some(b.buffer.clone())).collect();
            let buffers = buffers.as_slice();

            unsafe {
                device_context.IASetVertexBuffers(
                    slot,
                    buf_len,
                    Some(buffers.as_ptr()),
                    Some(strides.as_ptr()),
                    Some(offsets.as_ptr()),
                );
            }
        }
    }

    pub fn draw(bufs: &[&Self], start: u32, device_context: &ID3D11DeviceContext) {
        let total = bufs.iter().map(|b| b.count).sum();
        unsafe { device_context.Draw(total, start) }
    }

    pub fn set_and_draw(bufs: &[&Self], device_context: &ID3D11DeviceContext) {
        for (i, buf) in bufs.iter().enumerate() {
            buf.set(i as u32, device_context);
        }
        //Self::set_many(bufs, 0, device_context);
        Self::draw(bufs, 0, device_context);
    }
}

impl Model {
    pub fn load(obj_file: &Path) -> anyhow::Result<Vec<Rc<Self>>> {
        let (models, _materials) = tobj::load_obj(
            obj_file,
            &tobj::LoadOptions {
                merge_identical_points: false,
                reorder_data: false,
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )?;

        log::info!("File {:?} contains {} models", obj_file, models.len());
        let mut kat_models = Vec::new();
        for (i, m) in models.iter().enumerate() {
            let mesh = &m.mesh;
            log::info!("model[{}].name             = \'{}\'", i, m.name);
            log::info!("model[{}].mesh.material_id = {:?}", i, mesh.material_id);

            log::info!(
                "model[{}].face_count       = {}",
                i,
                mesh.face_arities.len()
            );

            log::info!(
                "model[{}].positions        = {}",
                i,
                mesh.positions.len() / 3
            );
            assert!(mesh.positions.len() % 3 == 0);

            log::info!("model[{}].normals        = {}", i, mesh.normals.len() / 3);

            let mut vertices = Vec::new();
            for index in mesh.indices.iter() {
                let start = *index as usize * 3;
                let end = *index as usize * 3 + 3;
                let start_2d = *index as usize * 2;
                let end_2d = *index as usize * 2 + 2;
                let vertex = &mesh
                    .positions
                    .get(start..end)
                    .map(Vec3::from_slice)
                    .unwrap_or_default();
                let colour = &mesh
                    .vertex_color
                    .get(start..end)
                    .map(Vec3::from_slice)
                    .unwrap_or(Vec3::new(1.0, 1.0, 1.0));
                let normal = &mesh
                    .normals
                    .get(start..end)
                    .map(Vec3::from_slice)
                    .unwrap_or_default();
                let texture = &mesh
                    .texcoords
                    .get(start_2d..end_2d)
                    .map(Vec2::from_slice)
                    .unwrap_or_default();
                vertices.push(Vertex {
                    position: *vertex,
                    colour: *colour,
                    normal: *normal,
                    texture: *texture,
                })
            }

            kat_models.push(Rc::new(Self {
                name: m.name.clone(),
                file: obj_file.to_path_buf(),
                index: i,
                vertices,
            }));
        }
        Ok(kat_models)
    }

    pub fn to_buffer(&self, device: &ID3D11Device) -> anyhow::Result<VertexBuffer> {
        let vertex_data_array: &[Vertex] = self.vertices.as_slice();

        let stride: u32 = size_of::<Vertex>() as u32;
        let offset: u32 = 0;
        let count: u32 = vertex_data_array.len() as u32;

        log::info!("Setting up vertex buffer");
        let mut vertex_buffer_ptr: Option<ID3D11Buffer> = None;
        let subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: vertex_data_array.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let vertex_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of_val(vertex_data_array) as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let buffer = unsafe {
            device.CreateBuffer(
                &vertex_buffer_desc,
                Some(&subresource_data),
                Some(&mut vertex_buffer_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| vertex_buffer_ptr.ok_or_else(|| anyhow!("no vertex buffer")))?;

        let vertex_buffer = VertexBuffer {
            buffer,
            stride,
            offset,
            count,
        };
        Ok(vertex_buffer)
    }

    pub fn to_buffers(
        device: &ID3D11Device,
        models: Vec<Rc<Self>>,
    ) -> anyhow::Result<Vec<VertexBuffer>> {
        let mut vertex_buffers = Vec::new();
        for model in models {
            let vertex_buffer = model.to_buffer(device)?;
            vertex_buffers.push(vertex_buffer);
        }
        Ok(vertex_buffers)
    }

    pub fn load_to_buffers(
        device: &ID3D11Device,
        obj_file: &Path,
    ) -> anyhow::Result<Vec<VertexBuffer>> {
        let models = Self::load(obj_file)?;
        Ok(Self::to_buffers(device, models)?)
    }
}
