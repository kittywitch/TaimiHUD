use {
    super::{
        super::{
            dx11::{InstanceBuffer, InstanceBufferData, VertexBuffer},
            resources::{Model, ObjMaterial, ShaderPair},
        },
        PrimitiveTopology,
    },
    glam::Mat4,
    itertools::Itertools,
    std::sync::RwLock,
    windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext},
};

pub struct ObjectRenderBacking {
    pub metadata: ObjectRenderMetadata,
    pub instance_buffer: RwLock<InstanceBuffer>,
    pub vertex_buffer: VertexBuffer,
    pub shaders: ShaderPair,
}

pub struct ObjectRenderMetadata {
    pub model: Model,
    pub material: ObjMaterial,
    pub model_matrix: Mat4,
    pub topology: PrimitiveTopology,
}

impl ObjectRenderBacking {
    pub fn update_instance_buffer(
        &self,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
        data: &[InstanceBufferData],
    ) -> anyhow::Result<()> {
        // TODO: extract inner error somehow, arc's suggestion didn't work o:
        let mut lock = self.instance_buffer.write().unwrap();
        lock.update(device, device_context, data)?;
        drop(lock);
        Ok(())
    }

    pub fn set_shaders(&self, device_context: &ID3D11DeviceContext) {
        self.shaders.set(device_context);
    }

    pub fn set_texture(&self, slot: u32, device_context: &ID3D11DeviceContext) {
        if let Some(diffuse) = &self.metadata.material.diffuse {
            diffuse.texture.set(device_context, slot);
        }
    }

    pub fn set_buffers(&self, slot: u32, device_context: &ID3D11DeviceContext) {
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
    pub fn set_and_draw(
        &self,
        slot: u32,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
        data: &[InstanceBufferData],
    ) -> anyhow::Result<()> {
        self.update_instance_buffer(device, device_context, data)?;
        self.set_shaders(device_context);
        self.set_texture(slot, device_context);
        self.set_buffers(slot, device_context);
        self.draw(slot, device_context);
        Ok(())
    }
}
