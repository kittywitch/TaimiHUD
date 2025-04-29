use {
    super::{
        primitivetopology::PrimitiveTopology, texture::Texture, vertexbuffer::VertexBuffer
    }, anyhow::anyhow, glam::{Vec2, Vec3, Vec3Swizzles}, itertools::Itertools, serde::{Deserialize, Serialize}, std::{collections::HashMap, path::{Path, PathBuf}, sync::Arc}, tobj::{Material as tobjMaterial, Model as tobjModel}, windows::Win32::Graphics::{Direct3D::D3D11_PRIMITIVE_TOPOLOGY_UNDEFINED, Direct3D11::{
        ID3D11Buffer, ID3D11Device, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
        D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
    }}
};
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: Vec3,
    pub normal: Vec3,
    pub texture: Vec2,
}

#[derive(Debug,Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ModelLocationDescription {
    pub file: PathBuf,
    pub index: usize,
}

pub struct ColouredMaterialTexture {
    pub texture: Texture,
    pub colour: Vec3,
}
pub struct AttributedMaterialTexture {
    pub texture: Texture,
    pub attribute: f32,
}



#[derive(Default)]
pub struct MaterialTextures {
    pub ambient: Option<ColouredMaterialTexture>,
    pub diffuse: Option<ColouredMaterialTexture>,
    pub specular: Option<ColouredMaterialTexture>,
    pub normal: Option<Texture>,
    pub shininess: Option<AttributedMaterialTexture>,
    pub dissolve: Option<AttributedMaterialTexture>,
}

pub struct ObjMaterials {
    pub materials: Vec<tobjMaterial>,
    pub folder: PathBuf,
}

impl ObjMaterials {
    fn load_idx(&self, device: &ID3D11Device, idx: usize) -> anyhow::Result<MaterialTextures> {
        let material = &self.materials[idx];
        let device_context = unsafe { device
            .GetImmediateContext() }.expect("I lost my context!");

        let ambient = if let (Some(texture), Some(value)) = (&material.ambient_texture, &material.ambient) {
            let texture_path = self.folder.join(PathBuf::from(&texture));
            let texture = Texture::load(device, &texture_path)?;
            let colour = Vec3::from_slice(value);
            Some(ColouredMaterialTexture {
                texture,
                colour,
            })
        } else { None };
        let diffuse = if let (Some(texture), Some(value)) = (&material.diffuse_texture, &material.diffuse) {
            let texture_path = self.folder.join(PathBuf::from(&texture));
            let texture = Texture::load(device, &texture_path)?;
            texture.generate_mips(&device_context);
            let colour = Vec3::from_slice(value);
            Some(ColouredMaterialTexture {
                texture,
                colour,
            })
        } else { None };
        let specular = if let (Some(texture), Some(value)) = (&material.specular_texture, &material.specular) {
            let texture_path = self.folder.join(PathBuf::from(&texture));
            let texture = Texture::load(device, &texture_path)?;
            let colour = Vec3::from_slice(value);
            Some(ColouredMaterialTexture {
                texture,
                colour,
            })
        } else { None };
        let normal = if let Some(texture) = &material.normal_texture {
            let texture_path = self.folder.join(PathBuf::from(&texture));
            Some(Texture::load(device, &texture_path)?)
        } else { None };
        let shininess = if let (Some(texture), Some(attribute)) = (&material.shininess_texture, material.shininess) {
            let texture_path = self.folder.join(PathBuf::from(&texture));
            let texture = Texture::load(device, &texture_path)?;
            Some(AttributedMaterialTexture {
                texture,
                attribute,
            })
        } else { None };
        let dissolve = if let (Some(texture), Some(attribute)) = (&material.dissolve_texture, material.dissolve) {
            let texture_path = self.folder.join(PathBuf::from(&texture));
            let texture = Texture::load(device, &texture_path)?;
            Some(AttributedMaterialTexture {
                texture,
                attribute,
            })
        } else { None };
        let material_set = MaterialTextures {
            ambient,
            diffuse,
            specular,
            normal,
            shininess,
            dissolve
        };

        Ok(material_set)
    }
    fn load_all(&self, device: &ID3D11Device) -> anyhow::Result<Vec<MaterialTextures>> {
        let mut material_textures = Vec::new();
        for i in 0..self.materials.len() {
            material_textures.push(self.load_idx(device, i)?);
        }
        Ok(material_textures)
    }
}

pub struct ObjModelFile {
    pub models: Vec<tobjModel>,
    pub materials: Option<ObjMaterials>,
}

pub struct ObjModelData {
    pub model: Model,
    pub material: MaterialTextures,
}

impl ObjModelFile {
    pub fn load_data(&self, device: &ID3D11Device, idxs: Vec<usize>) -> Vec<ObjModelData> {
        idxs.iter().map(|idx| self.load_datum(device, *idx, false)).collect()
    }

    pub fn load_datum(&self, device: &ID3D11Device, idx: usize, xzy: bool) -> ObjModelData {
        ObjModelData {
            model: self.load_model(idx, xzy),
            material: self.load_material_for_model(device, idx).unwrap_or_default(),
        }
    }

    pub fn load_material_for_model(&self, device: &ID3D11Device, idx: usize) -> Option<MaterialTextures> {
        let mat_idx = &self.models[idx].mesh.material_id?;
        if let Some(materials) = &self.materials {
            let material = materials.load_idx(device, *mat_idx).ok();
            material
        } else {
            None
        }
    }
    pub fn load_model(&self, idx: usize, xzy: bool) -> Model {
        let mesh = &self.models[idx].mesh;
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
                .map(|v| if xzy { v.xzy() } else { v })
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
        Model(vertices)
    }
    pub fn load_file(file: &Path) -> anyhow::Result<Self> {
        log::info!("Attempting to load {file:?}.");
        let (models, materials) = tobj::load_obj(
            file,
            &tobj::LoadOptions {
                merge_identical_points: false,
                reorder_data: false,
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            },
        )?;
        let folder = file.parent();
        let materials = match (materials, folder) {
            (Ok(mats), Some(folder)) => {
                log::info!("Material load succeeded for obj model file {file:?}!");
                Some(ObjMaterials {
                    materials: mats,
                    folder: folder.to_path_buf(),

                })
            }
            (_, None) => {
                log::warn!("Material load failure for obj model file {file:?}, has no parent");
                None
            },
            (Err(err), _) => {
                log::warn!("Material load error for obj model file {file:?}: {err}");
                None
            },
        };
        if let Some(ref materials) = materials {
            log::info!("File {file:?} loaded, contents: {} models, {} materials.", models.len(), materials.materials.len());
        } else {
            log::info!("File {file:?} loaded, contents: {} models, no materials.", models.len());
        }
        Ok(Self {
            models,
            materials,
        })
    }
}

#[derive(Clone)]
pub struct TobjRef {
    file: Arc<ObjModelFile>,
    model_idx: usize,
}

#[derive(Default, PartialEq)]
pub struct Model(Vec<Vertex>);



impl Model {
    pub fn swizzle(&mut self) {
        for v in &mut self.0 {
            v.position = v.position.xzy();
        }
    }
    pub fn quad() -> anyhow::Result<Self> {
        let mut vertices = Vec::new();
        let height = 1.0;
        let width = 1.0;
        let vertex_coordinates= [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, height, 0.0),
            Vec3::new(width, height, 0.0),

            Vec3::new(width, height, 0.0),
            Vec3::new(width, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let colour = Vec3::new(1.0, 1.0, 1.0);
        let mut normal = Vec3::new(0.0, 0.0, 0.0);
        for i in 0..vertex_coordinates.len() {
            let current = vertex_coordinates[i];
            let next_idx = (i + 1) % vertex_coordinates.len();
            let next = vertex_coordinates[next_idx];
            normal += Vec3::new(
            (current.y - next.y) * (current.z + next.z),
                (current.z - next.z) * (current.x + next.x),
                (current.x - next.x) * (current.y + next.y)
            );
            vertices.push(Vertex {
                position: current - Vec3::new(width/2.0, height/2.0, 0.0),
                normal,
                texture: current.xy(),
                colour,
            });
        }

        Ok(Self(
            vertices
        ))
    }

    pub fn to_buffer(&self, device: &ID3D11Device) -> anyhow::Result<VertexBuffer> {
        let vertex_data_array: &[Vertex] = self.0.as_slice();

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
}
