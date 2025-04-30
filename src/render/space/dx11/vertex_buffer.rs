use windows::Win32::Graphics::Direct3D11::ID3D11Buffer;

#[derive(Clone, PartialEq)]
pub struct VertexBuffer {
    pub buffer: ID3D11Buffer,
    pub stride: u32,
    pub offset: u32,
    pub count: u32,
}
