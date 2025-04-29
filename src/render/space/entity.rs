use {
    super::{
        model::{Model, ModelLocationDescription}, primitivetopology::PrimitiveTopology, shader::{Shaders}, state::InstanceBufferData, vertexbuffer::VertexBuffer
    },
    anyhow::anyhow,
    glam::{Mat4, Vec2, Vec3},
    std::{cell::RefCell, path::Path, rc::Rc, sync::Arc},
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
