use {
    super::{
        state::Texture, vertexbuffer::VertexBuffer
    },
    anyhow::anyhow,
    glam::{Vec2, Vec3, Vec3Swizzles},
    itertools::Itertools,
    serde::{Deserialize, Serialize},
    std::{
        path::{Path, PathBuf},
        rc::Rc,
    },
    windows::Win32::Graphics::Direct3D11::{
        ID3D11Buffer, ID3D11Device, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
        D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
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
    pub file: PathBuf,
    pub index: usize,
}

#[derive(Default)]
pub struct Model {
    pub vertices: Vec<Vertex>,
    pub texture: Option<Texture>,
}

impl Model {
    pub fn swizzle(&mut self) {
        for v in &mut self.vertices {
            v.position = v.position.xzy();
        }
    }

    pub fn load(device: &ID3D11Device, obj_file: &Path) -> anyhow::Result<Vec<Self>> {
        let folder = obj_file.parent();
        let (models, materials) = tobj::load_obj(
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

        match &materials {
            Ok(mats) => {
                log::info!("Model file \"{:?}\" contains {} materials!", obj_file, mats.len());
                for mat in mats {
                    log::info!("Material {}, diffuse: {:?}", mat.name, mat.diffuse_texture);
                };
            },
            Err(err) => log::info!("{err}: Model file \"{:?}\" contains no materials!", obj_file),
        }

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

            log::info!("model[{}].texcoords        = {}", i, mesh.texcoords.len() / 2);

            let texture = match (&materials, mesh.material_id) {
                (Ok(mats), Some(mat_id)) => {
                    if let (Some(folder), Some(texture_path)) = (folder, &mats[mat_id].diffuse_texture) {

                        let texture = Texture::load(device, &folder.join(texture_path))?;
                        Some(texture)
                    } else {
                        None
                    }
                },
                _ => None,
            };

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

            kat_models.push(Self { vertices, texture });
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
}
