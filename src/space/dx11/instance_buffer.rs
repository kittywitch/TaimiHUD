use {
    super::InstanceBufferData,
    anyhow::anyhow,
    bevy_ecs::prelude::*,
    windows::Win32::Graphics::Direct3D11::{
        ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, D3D11_BIND_VERTEX_BUFFER,
        D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
    },
};

pub struct InstanceBuffer {
    buffer: ID3D11Buffer,
    count: usize,
}

impl InstanceBuffer {
    pub fn get_buffer(&self) -> ID3D11Buffer {
        self.buffer.clone()
    }

    pub fn get_count(&self) -> usize {
        self.count
    }

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
        let buffer = unsafe { device.CreateBuffer(&desc, Some(&subresource_data), Some(&mut ptr)) }
            .map_err(anyhow::Error::from)
            .and_then(|()| ptr.ok_or_else(|| anyhow!("no per-entity structured buffer")))?;

        Ok(Self { buffer, count })
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
            pSysMem: data.as_ptr().cast(),
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        let mut ptr: Option<ID3D11Buffer> = None;
        let buffer = unsafe { device.CreateBuffer(&desc, Some(&subresource_data), Some(&mut ptr)) }
            .map_err(anyhow::Error::from)
            .and_then(|()| ptr.ok_or_else(|| anyhow!("no per-entity structured buffer")))?;

        Ok(Self { buffer, count })
    }
    pub fn update(
        &mut self,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
        data: &[InstanceBufferData],
    ) -> anyhow::Result<()> {
        if data.len() == self.count {
            unsafe {
                device_context.UpdateSubresource(&self.buffer, 0, None, data.as_ptr().cast(), 0, 0);
            }
        } else {
            *self = Self::create(device, data)?;
        }
        Ok(())
    }
}
