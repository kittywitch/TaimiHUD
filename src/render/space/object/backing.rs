use {
    super::ObjectRenderBacking, crate::render::space::state::InstanceBufferData, windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext}
};

pub struct ObjectBacking {
    pub name: String,
    pub render: ObjectRenderBacking,
}

impl ObjectBacking {
    pub fn update_instance_buffer(&self, device: &ID3D11Device,
        device_context: &ID3D11DeviceContext, data: &[InstanceBufferData]) -> anyhow::Result<()> {
        self.render.update_instance_buffer(device, device_context, data)?;
        Ok(())
    }
}
