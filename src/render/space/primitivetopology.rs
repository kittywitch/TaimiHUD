use {
    super::{texture::Texture, vertexbuffer::VertexBuffer},
    anyhow::anyhow,
    glam::{Vec2, Vec3, Vec3Swizzles},
    itertools::Itertools,
    serde::{Deserialize, Serialize},
    std::path::{Path, PathBuf},
    windows::Win32::Graphics::{Direct3D::{D3D11_PRIMITIVE_TOPOLOGY_LINELIST, D3D11_PRIMITIVE_TOPOLOGY_LINELIST_ADJ, D3D11_PRIMITIVE_TOPOLOGY_LINESTRIP, D3D11_PRIMITIVE_TOPOLOGY_LINESTRIP_ADJ, D3D11_PRIMITIVE_TOPOLOGY_POINTLIST, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST_ADJ, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP_ADJ, D3D11_PRIMITIVE_TOPOLOGY_UNDEFINED, D3D_PRIMITIVE_TOPOLOGY}, Direct3D11::{
        ID3D11Buffer, ID3D11Device, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC,
        D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
    }},
};

#[derive(Debug,Default,Clone,Copy,Serialize,Deserialize,PartialEq)]
pub enum PrimitiveTopology {
    Undefined,
    PointList,
    LineList,
    LineStrip,
    #[default]
    TriangleList,
    TriangleStrip,
    LineListAdj,
    LineStripAdj,
    TriangleListAdj,
    TriangleStripAdj,
}

impl PrimitiveTopology {
    pub fn dx11(&self) -> D3D_PRIMITIVE_TOPOLOGY {
        use PrimitiveTopology::*;
        match self {
            Undefined => D3D11_PRIMITIVE_TOPOLOGY_UNDEFINED,
            PointList => D3D11_PRIMITIVE_TOPOLOGY_POINTLIST,
            LineList => D3D11_PRIMITIVE_TOPOLOGY_LINELIST,
            LineStrip => D3D11_PRIMITIVE_TOPOLOGY_LINESTRIP,
            TriangleList => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            TriangleStrip => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP,
            LineListAdj => D3D11_PRIMITIVE_TOPOLOGY_LINELIST_ADJ,
            LineStripAdj => D3D11_PRIMITIVE_TOPOLOGY_LINESTRIP_ADJ,
            TriangleListAdj => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST_ADJ,
            TriangleStripAdj => D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP_ADJ,
        }
    }
}
