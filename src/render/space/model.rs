use {
    anyhow::anyhow, glam::{Vec2, Vec3}, relative_path::RelativePathBuf, std::path::{Path, PathBuf}, windows::Win32::Graphics::Direct3D11::{
        ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
    }
};
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: Vec3,
    pub normal: Vec3,
    pub texture: Vec2,
}

#[derive(Clone)]
pub struct Model {
    // todo! figure out how to store a relative path here o:
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
    pub fn set(bufs: &[Self], slot: u32, device_context: &ID3D11DeviceContext) {
        let buf_len = bufs.len() as u32;
        if buf_len != 0 {
            let strides: Vec<_> = bufs.iter().map(|b| b.stride).collect();
            let strides = strides.as_slice();
            let offsets: Vec<_> = bufs.iter().map(|b| b.offset).collect();
            let offsets = offsets.as_slice();
            let buffers: Vec<_> = bufs.iter().map(|b| Some(b.buffer.to_owned())).collect();
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

    pub fn draw(bufs: &[Self], start: u32, device_context: &ID3D11DeviceContext) {
        let total = bufs.iter().map(|b| b.count).sum();
        unsafe { device_context.Draw(total, start) }
    }

    pub fn set_and_draw(bufs: &[Self], device_context: &ID3D11DeviceContext) {
        Self::set(bufs, 0, device_context);
        Self::draw(bufs, 0, device_context);
    }
}

impl Model {
    pub fn load(obj_file: &Path) -> Vec<Self> {
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
        )
        .expect("Failed to load OBJ file");

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

            kat_models.push(Self {
                file: obj_file.to_path_buf(),
                index: i,
                vertices
            });
        }
        kat_models
    }

    pub fn load_to_buffers(
        d3d11_device: ID3D11Device,
        obj_file: &Path,
    ) -> anyhow::Result<Vec<VertexBuffer>> {
        let models = Self::load(obj_file);

        let mut vertex_buffers = Vec::new();
        for model in models {
            let vertex_data_array: &[Vertex] = model.vertices.as_slice();

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
                d3d11_device.CreateBuffer(
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
            vertex_buffers.push(vertex_buffer);
        }

        Ok(vertex_buffers)
    }
}
