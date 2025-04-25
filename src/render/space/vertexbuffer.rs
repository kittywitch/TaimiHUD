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
