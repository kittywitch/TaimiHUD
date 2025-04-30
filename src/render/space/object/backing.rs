use {
    super::{super::dx11::InstanceBufferData, ObjectRenderBacking},
    windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext},
};

pub struct ObjectBacking {
    pub name: String,
    pub render: ObjectRenderBacking,
}

impl ObjectBacking {
    pub fn set_and_draw(
        &self,
        slot: u32,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
        data: &[InstanceBufferData],
    ) -> anyhow::Result<()> {
        self.render.set_and_draw(slot, device, device_context, data)
    }
}
