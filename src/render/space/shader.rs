use {
    super::model::Vertex,
    anyhow::anyhow,
    core::ffi::c_char,
    std::{
        ffi::CStr,
        path::PathBuf,
        mem::offset_of,
        slice::from_raw_parts,
    },
    windows_strings::{
        s,
        PCSTR,
        HSTRING,
    },
    strum_macros::Display,
    windows::Win32::Graphics::{
        Direct3D::{
            Fxc::{
                D3DCompileFromFile,
                D3DCOMPILE_DEBUG,
            },
            ID3DBlob,
        },
        Direct3D11::{
            ID3D11Device,
            ID3D11DeviceContext,
            ID3D11VertexShader,
            ID3D11PixelShader,
            ID3D11InputLayout,
            D3D11_INPUT_ELEMENT_DESC,
            D3D11_APPEND_ALIGNED_ELEMENT,
            D3D11_INPUT_PER_VERTEX_DATA,
        },
        Dxgi::Common::{DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32_FLOAT},
    },
};

#[derive(Display)]
pub enum ShaderKind {
    Vertex,
    Pixel,
}
pub enum Shader {
    Vertex(VertexShader),
    Pixel(PixelShader),
}

pub struct VertexShader {
    path: PathBuf,
    entrypoint: PCSTR,
    shader: ID3D11VertexShader,
    layout: ID3D11InputLayout,
}

pub struct PixelShader {
    path: PathBuf,
    entrypoint: PCSTR,
    shader: ID3D11PixelShader,
}

impl Shader {
    pub fn set(&self, context: &ID3D11DeviceContext) {
        match self {
            Self::Vertex(s) => {
                unsafe {
                    context.IASetInputLayout(&s.layout);
                    context.VSSetShader(&s.shader, None);
                }
            },
            Self::Pixel(s) => {
                unsafe {
                    context.PSSetShader(&s.shader, None);
                }
            },
        }
    }
    pub fn compile(path: &PathBuf, kind: &ShaderKind, entrypoint: PCSTR) -> anyhow::Result<ID3DBlob> {
        let filename = HSTRING::from(path.as_os_str());
        let target = match kind {
            ShaderKind::Vertex => s!("vs_5_0"),
            ShaderKind::Pixel => s!("ps_5_0"),
        };
        log::info!("Beginning compile from {:?} of {} shader, entrypoint {:?}", path, kind, entrypoint);
        let mut blob_ptr: Option<ID3DBlob> = None;
        let mut error_blob: Option<ID3DBlob> = None;
        let blob = unsafe {
            D3DCompileFromFile(
                &filename,
                None,
                None,
                entrypoint,
                target,
                D3DCOMPILE_DEBUG,
                0,
                &mut blob_ptr,
                Some(&mut error_blob)
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| blob_ptr.ok_or_else(|| anyhow!("no {} shader", kind)))
        .map_err(|e| match error_blob {
            Some(ref error_blob) => {
                let msg = unsafe { CStr::from_ptr(error_blob.GetBufferPointer() as *const c_char) };
                let res = anyhow!("{}: {}", e, msg.to_string_lossy());
                let _ = error_blob;
                res
            }
            None => e,
        })?;

        log::info!("Compile successful from {:?} of {} shader, entrypoint {:?}", path, kind, entrypoint);
        Ok(blob)
    }

    pub fn create(device: &ID3D11Device, path: &PathBuf, kind: ShaderKind, entrypoint: PCSTR) -> anyhow::Result<Self> {
        let blob = Self::compile(path, &kind, entrypoint)?;

        let blob_bytes = unsafe {
            from_raw_parts(
                blob.GetBufferPointer() as *const u8,
                blob.GetBufferSize(),
            )
        };
        log::info!("Creating {:?} of {} shader, entrypoint {:?}", path, kind, entrypoint);
        match kind {
            ShaderKind::Vertex => {
                let mut shader_ptr: Option<ID3D11VertexShader> = None;
                let shader = unsafe {
                    device.CreateVertexShader(
                        blob_bytes,
                        None,
                        Some(&mut shader_ptr),
                    )
                }
                .map_err(anyhow::Error::from)
                .and_then(|()| shader_ptr.ok_or_else(|| anyhow!("no vertex shader")))?;
                let input_layout_description: &[D3D11_INPUT_ELEMENT_DESC] = &[
                    D3D11_INPUT_ELEMENT_DESC {
                        SemanticName: s!("POSITION"),
                        SemanticIndex: 0,
                        Format: DXGI_FORMAT_R32G32B32_FLOAT,
                        InputSlot: 0,
                        AlignedByteOffset: offset_of!(Vertex, position) as u32,
                        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                        InstanceDataStepRate: 0,
                    },
                    D3D11_INPUT_ELEMENT_DESC {
                        SemanticName: s!("COLOR"),
                        SemanticIndex: 0,
                        Format: DXGI_FORMAT_R32G32B32_FLOAT,
                        InputSlot: 0,
                        AlignedByteOffset: offset_of!(Vertex, colour) as u32,
                        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                        InstanceDataStepRate: 0,
                    },
                    D3D11_INPUT_ELEMENT_DESC {
                        SemanticName: s!("NORMAL"),
                        SemanticIndex: 0,
                        Format: DXGI_FORMAT_R32G32B32_FLOAT,
                        InputSlot: 0,
                        AlignedByteOffset: offset_of!(Vertex, normal) as u32,
                        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                        InstanceDataStepRate: 0,
                    },
                    D3D11_INPUT_ELEMENT_DESC {
                        SemanticName: s!("TEX"),
                        SemanticIndex: 0,
                        Format: DXGI_FORMAT_R32G32_FLOAT,
                        InputSlot: 0,
                        AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                        InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                        InstanceDataStepRate: 0,
                    },
                ];
                log::info!("Creating input layout for {:?} of {} shader, entrypoint {:?}", path, kind, entrypoint);
                let mut layout_ptr: Option<ID3D11InputLayout> = None;
                let layout = unsafe {
                    device.CreateInputLayout(
                        input_layout_description,
                        blob_bytes,
                        Some(&mut layout_ptr),
                    )
                }
                .map_err(anyhow::Error::from)
                .and_then(|()| layout_ptr.ok_or_else(|| anyhow!("no input layout")))?;

                let wrapped_shader = VertexShader {

                    path: path.to_path_buf(),
                    layout,
                    entrypoint,
                    shader,
                };
                Ok(Shader::Vertex(wrapped_shader))
            },
            ShaderKind::Pixel => {
                let mut shader_ptr: Option<ID3D11PixelShader> = None;
                let shader = unsafe {
                    device.CreatePixelShader(
                        blob_bytes,
                        None,
                        Some(&mut shader_ptr),
                    )
                }
                .map_err(anyhow::Error::from)
                .and_then(|()| shader_ptr.ok_or_else(|| anyhow!("no pixel shader")))?;
                let wrapped_shader = PixelShader {
                    path: path.to_path_buf(),
                    entrypoint,
                    shader,
                };
                Ok(Shader::Pixel(wrapped_shader))
            },
        }
    
    }
}
