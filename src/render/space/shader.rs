use {
    super::model::Vertex, crate::render::space::state::InstanceBufferData, anyhow::anyhow, core::ffi::c_char, glob::Paths, serde::{Deserialize, Serialize}, std::{
        collections::HashMap,
        ffi::{CStr, CString},
        fs::read_to_string,
        mem::offset_of,
        path::{Path, PathBuf},
        rc::Rc,
        slice::from_raw_parts, sync::Arc,
    }, strum_macros::Display, windows::Win32::Graphics::{
        Direct3D::{
            Fxc::{D3DCompileFromFile, D3DCOMPILE_DEBUG},
            ID3DBlob,
        },
        Direct3D11::{
            ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout, ID3D11PixelShader,
            ID3D11VertexShader, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_INPUT_ELEMENT_DESC,
            D3D11_INPUT_PER_INSTANCE_DATA, D3D11_INPUT_PER_VERTEX_DATA,
        },
        Dxgi::Common::{
            DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32_FLOAT,
        },
    }, windows_strings::{s, HSTRING, PCSTR}
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ShaderDescription {
    pub identifier: String,
    pub kind: ShaderKind,
    pub path: PathBuf,
    pub entrypoint: String,
}

impl ShaderDescription {
    pub fn load(path: &PathBuf) -> anyhow::Result<Vec<Self>> {
        log::debug!("Attempting to load the shader description file at \"{path:?}\".");
        let mut file_data = read_to_string(path)?;
        json_strip_comments::strip(&mut file_data)?;
        let shader_description_data: Vec<Self> = serde_json::from_str(&file_data)?;
        Ok(shader_description_data)
    }
    pub fn get(&self, shader_folder: &Path) -> anyhow::Result<(HSTRING, PCSTR, CString)> {
        let filename = HSTRING::from(shader_folder.join(&self.path).as_os_str());
        let target = match self.kind {
            ShaderKind::Vertex => s!("vs_5_0"),
            ShaderKind::Pixel => s!("ps_5_0"),
        };
        let entrypoint_cstring = CString::new(self.entrypoint.clone())?;

        Ok((filename, target, entrypoint_cstring))
    }
}

#[derive(Display, Debug, Serialize, Deserialize)]
pub enum ShaderKind {
    Vertex,
    Pixel,
}

#[derive(PartialEq)]
pub struct VertexShader {
    shader: ID3D11VertexShader,
    layout: ID3D11InputLayout,
}
impl VertexShader {
    pub fn create(
        shader_folder: &Path,
        device: &ID3D11Device,
        desc: &ShaderDescription,
    ) -> anyhow::Result<Self> {
        let blob = Self::compile(shader_folder, desc)?;

        let blob_bytes =
            unsafe { from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize()) };
        log::info!(
            "Creating {:?} of {} shader, entrypoint {:?}",
            &desc.path,
            &desc.kind,
            &desc.entrypoint
        );
            let mut shader_ptr: Option<ID3D11VertexShader> = None;
            let shader =
                unsafe { device.CreateVertexShader(blob_bytes, None, Some(&mut shader_ptr)) }
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
                    SemanticName: s!("TEXCOORD"),
                    SemanticIndex: 0,
                    Format: DXGI_FORMAT_R32G32_FLOAT,
                    InputSlot: 0,
                    AlignedByteOffset: offset_of!(Vertex, texture) as u32,
                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                    InstanceDataStepRate: 0,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("MODEL"),
                    SemanticIndex: 0,
                    InputSlot: 1,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
                    InstanceDataStepRate: 1,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("MODEL"),
                    SemanticIndex: 1,
                    InputSlot: 1,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
                    InstanceDataStepRate: 1,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("MODEL"),
                    SemanticIndex: 2,
                    InputSlot: 1,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
                    InstanceDataStepRate: 1,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("MODEL"),
                    SemanticIndex: 3,
                    InputSlot: 1,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
                    InstanceDataStepRate: 1,
                },
                D3D11_INPUT_ELEMENT_DESC {
                    SemanticName: s!("COLOUR"),
                    SemanticIndex: 0,
                    InputSlot: 1,
                    Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                    AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
                    InputSlotClass: D3D11_INPUT_PER_INSTANCE_DATA,
                    InstanceDataStepRate: 1,
                },
            ];
            log::info!(
                "Creating input layout for {:?} of {} shader, entrypoint {:?}",
                &desc.path,
                &desc.kind,
                &desc.entrypoint
            );
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

            Ok(VertexShader { layout, shader })
    }
    pub fn set(&self, context: &ID3D11DeviceContext) {
        unsafe {
            context.IASetInputLayout(&self.layout);
            context.VSSetShader(&self.shader, None);
        }
    }

    pub fn compile(shader_folder: &Path, desc: &ShaderDescription) -> anyhow::Result<ID3DBlob> {
        let (filename, target, entrypoint_cstring) = desc.get(shader_folder)?;
        log::info!(
            "Beginning compile from {:?} of {} shader, entrypoint {:?}",
            &desc.path,
            &desc.kind,
            entrypoint_cstring
        );
        let mut blob_ptr: Option<ID3DBlob> = None;
        let mut error_blob: Option<ID3DBlob> = None;
        let entrypoint = PCSTR::from_raw(entrypoint_cstring.as_ptr() as *const u8);
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
                Some(&mut error_blob),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| blob_ptr.ok_or_else(|| anyhow!("no {} shader", &desc.kind)))
        .map_err(|e| match error_blob {
            Some(ref error_blob) => {
                let msg = unsafe { CStr::from_ptr(error_blob.GetBufferPointer() as *const c_char) };
                let res = anyhow!("{}: {}", e, msg.to_string_lossy());
                let _ = error_blob;
                res
            }
            None => e,
        })?;

        log::info!(
            "Compile successful from {:?} of {} shader, entrypoint {:?}",
            &desc.path,
            &desc.kind,
            entrypoint
        );
        Ok(blob)
    }
}

#[derive(PartialEq)]
pub struct PixelShader {
    shader: ID3D11PixelShader,
}
impl PixelShader {
    pub fn create(
        shader_folder: &Path,
        device: &ID3D11Device,
        desc: &ShaderDescription,
    ) -> anyhow::Result<Self> {
        let blob = Self::compile(shader_folder, desc)?;

        let blob_bytes =
            unsafe { from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize()) };
        log::info!(
            "Creating {:?} of {} shader, entrypoint {:?}",
            &desc.path,
            &desc.kind,
            &desc.entrypoint
        );
        let mut shader_ptr: Option<ID3D11PixelShader> = None;
        let shader =
            unsafe { device.CreatePixelShader(blob_bytes, None, Some(&mut shader_ptr)) }
                .map_err(anyhow::Error::from)
                .and_then(|()| shader_ptr.ok_or_else(|| anyhow!("no pixel shader")))?;
        Ok(PixelShader { shader })
    }
    pub fn set(&self, context: &ID3D11DeviceContext) {
        unsafe {
            context.PSSetShader(&self.shader, None);
        }
    }
    pub fn compile(shader_folder: &Path, desc: &ShaderDescription) -> anyhow::Result<ID3DBlob> {
        let (filename, target, entrypoint_cstring) = desc.get(shader_folder)?;
        log::info!(
            "Beginning compile from {:?} of {} shader, entrypoint {:?}",
            &desc.path,
            &desc.kind,
            entrypoint_cstring
        );
        let mut blob_ptr: Option<ID3DBlob> = None;
        let mut error_blob: Option<ID3DBlob> = None;
        let entrypoint = PCSTR::from_raw(entrypoint_cstring.as_ptr() as *const u8);
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
                Some(&mut error_blob),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| blob_ptr.ok_or_else(|| anyhow!("no {} shader", &desc.kind)))
        .map_err(|e| match error_blob {
            Some(ref error_blob) => {
                let msg = unsafe { CStr::from_ptr(error_blob.GetBufferPointer() as *const c_char) };
                let res = anyhow!("{}: {}", e, msg.to_string_lossy());
                let _ = error_blob;
                res
            }
            None => e,
        })?;

        log::info!(
            "Compile successful from {:?} of {} shader, entrypoint {:?}",
            &desc.path,
            &desc.kind,
            entrypoint
        );
        Ok(blob)
    }
}


pub type VertexShaders = HashMap<String, Arc<VertexShader>>;
pub type PixelShaders = HashMap<String, Arc<PixelShader>>;

pub struct Shaders(pub VertexShaders, pub PixelShaders);

impl Shaders {
    pub fn setup(addon_dir: &Path, device: &ID3D11Device) -> anyhow::Result<Self> {
        log::info!("Beginning shader setup!");
        let shader_folder = addon_dir.join("shaders");
        let mut shader_descriptions: Vec<ShaderDescription> = Vec::new();
        let mut shaders: Shaders = Self(HashMap::new(), HashMap::new());
        if shader_folder.exists() {
            let shader_description_paths: Paths = glob::glob(
                shader_folder
                    .join("*.shaderdesc")
                    .to_str()
                    .expect("Shader load pattern is unparseable"),
            )?;
            for shader_description_path in shader_description_paths {
                let shader_description =
                    ShaderDescription::load(&shader_folder.join(shader_description_path?))?;
                shader_descriptions.extend(shader_description);
            }
            for shader_description in shader_descriptions {
                match shader_description.kind {
                    ShaderKind::Vertex => {
                        let shader = Arc::new(VertexShader::create(&shader_folder, device, &shader_description)?);
                        shaders.0.insert(shader_description.identifier, shader);
                    },
                    ShaderKind::Pixel => {
                        let shader = Arc::new(PixelShader::create(&shader_folder, device, &shader_description)?);
                        shaders.1.insert(shader_description.identifier, shader);
                },
                }
            }
        }
        log::info!("Finished shader setup. {} vertex shaders, {} pixel shaders loaded!", shaders.0.len(), shaders.1.len());
        Ok(shaders)
    }
}
