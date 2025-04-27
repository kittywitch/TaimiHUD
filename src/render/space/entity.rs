use {
    super::{
        model::{Model, ModelLocation}, primitivetopology::PrimitiveTopology, shader::{Shader, Shaders}, state::InstanceBufferData, vertexbuffer::VertexBuffer
    },
    anyhow::anyhow,
    glam::{Mat4, Vec2, Vec3},
    std::{cell::RefCell, path::Path, rc::Rc},
    windows::Win32::Graphics::Direct3D11::{
        ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
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

pub struct Entity {
    pub name: String,
    pub model_matrix: RefCell<Vec<InstanceBufferData>>,
    pub location: Option<ModelLocation>,
    pub model: Model,
    pub vertex_buffer: VertexBuffer,
    pub vertex_shader: Rc<Shader>,
    pub pixel_shader: Rc<Shader>,
    pub instance_buffer: ID3D11Buffer,
    pub topology: PrimitiveTopology,
}

impl Entity {

    pub fn quad(device: &ID3D11Device, shaders: &Shaders, path: Option<&Path>) -> anyhow::Result<Self> {
        let model = Model::quad(device, path)?;
        let model_matrix = vec![InstanceBufferData {
            model: Mat4::from_translation(Vec3::new(0.0, 150.0, 0.0)) * Mat4::from_scale(Vec3::new(10.0, 10.0, 10.0)),
            colour: Vec3::new(1.0,0.0,1.0),
        }];
        Ok(Self {
            topology: PrimitiveTopology::TriangleList,
            instance_buffer: Self::setup_instance_buffer(&model_matrix, device)?,
            vertex_buffer: model.to_buffer(device)?,
            name: "Quad".to_string(),
            vertex_shader: shaders.0["textured_vs"].clone(),
            pixel_shader: shaders.0["textured_ps"].clone(),
            location: None,
            model_matrix: RefCell::new(model_matrix),
            model,
        })
    }
    pub fn set(&self, slot: u32, device_context: &ID3D11DeviceContext) {
        let instance_buffer_stride = size_of::<InstanceBufferData>() as u32;
        let instance_buffer_offset = 0_u32;
        let buffers = [
            Some(self.vertex_buffer.buffer.clone()),
            Some(self.instance_buffer.clone()),
        ];
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
        let total = self.vertex_buffer.count + self.model_matrix.borrow().len() as u32;
        let instances = self.model_matrix.borrow().len();
        unsafe {
            device_context.IASetPrimitiveTopology(self.topology.dx11());
            device_context.DrawInstanced(total, instances as u32, start, 0)
        }
    }

    pub fn set_and_draw(&self, device_context: &ID3D11DeviceContext) {
        self.set(0_u32, device_context);
        self.draw(0, device_context);
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
