use {
    super::{
        entity::Entity,
        shader::Shader,
    },
    anyhow::anyhow,
    glam::{Affine3A, Mat4, Vec3},
    image::ImageReader,
    itertools::Itertools,
    std::{collections::HashMap, path::Path, rc::Rc},
    tokio::sync::mpsc::Receiver,
    windows::Win32::Graphics::{
        Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
        Direct3D11::{
            ID3D11Buffer, ID3D11DepthStencilState, ID3D11DepthStencilView, ID3D11Device,
            ID3D11DeviceContext, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState,
            ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_SHADER_RESOURCE_VIEW_DESC,
            D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
        },
        Dxgi::{
            Common::{
                DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_SAMPLE_DESC,
            },
            IDXGISwapChain,
        },
    },
};

#[derive(Debug)]
pub struct DrawData {
    pub player_position: Option<Vec3>,
    pub camera_front: Vec3,
    pub camera_up: Vec3,
    pub camera_position: Vec3,
}

type ShaderEntityMap = Vec<(Rc<Shader>, Rc<Shader>, Vec<Rc<Entity>>)>;

pub struct DrawState {
    receiver: Receiver<SpaceEvent>,
    draw_data: Option<DrawData>,
    render_target_view: [Option<ID3D11RenderTargetView>; 1],
    shaders: HashMap<String, Rc<Shader>>,
    rasterizer_state: ID3D11RasterizerState,
    viewport: D3D11_VIEWPORT,
    entities: Vec<Entity>,
    shader_entity_map: ShaderEntityMap,
    depth_stencil_state: ID3D11DepthStencilState,
    depth_stencil_buffer: ID3D11Texture2D,
    depth_stencil_view: ID3D11DepthStencilView,
    sampler_state: Vec<Option<ID3D11SamplerState>>,
    constant_buffer: ID3D11Buffer,
    constant_buffer_data: ConstantBufferData,
    device: ID3D11Device,
    swap_chain: IDXGISwapChain,
    aspect_ratio: Option<f32>,
    display_size: Option<[f32; 2]>,
}

#[repr(C, align(16))]
pub struct InstanceBufferData {
    pub model: Mat4,
    pub colour: Vec3,
}

impl InstanceBufferData {
    pub fn rotate(&mut self, dt: f32) {
        self.model = self.model * Affine3A::from_rotation_y(dt);
    }
}

#[repr(C)]
struct ConstantBufferData {
    view: Mat4,
    projection: Mat4,
}

impl ConstantBufferData {}

pub enum SpaceEvent {
    Update(DrawData),
}

pub type Shaders = HashMap<String, Rc<Shader>>;

pub struct Texture {
    pub texture: ID3D11Texture2D,
    pub dimensions: [u32; 2],
    pub view: Vec<Option<ID3D11ShaderResourceView>>,
}

impl Texture {
    pub fn load(device: &ID3D11Device, path: &Path) -> anyhow::Result<Self> {
        let image_reader = ImageReader::open(path)?;
        let format = image_reader.format();
        log::info!("Loading {:?} texture from {path:?}!", format);
        let image = image_reader.with_guessed_format()?.decode()?;
        let rgba_image = image.to_rgba32f();
        let dimensions = rgba_image.dimensions();
        let raw_rgba_image = rgba_image.into_raw();
        let texture_sample_desc = DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        };
        let texture_desc = D3D11_TEXTURE2D_DESC {
            Width: dimensions.0,
            Height: dimensions.1,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            SampleDesc: texture_sample_desc,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
        };
        let texture_subresource_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: raw_rgba_image.as_ptr().cast(),
            SysMemPitch: (size_of::<f32>() as u32 * dimensions.0 * 4),
            SysMemSlicePitch: 0,
        };
        let mut texture_ptr: Option<ID3D11Texture2D> = None;
        let texture = unsafe {
            device.CreateTexture2D(
                &texture_desc,
                Some(&texture_subresource_data),
                Some(&mut texture_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| texture_ptr.ok_or_else(|| anyhow!("no texture for {path:?}")))?;
        log::info!("Creating a shader resource view for {:?}!", path);
        let tex2d_srv = D3D11_TEX2D_SRV {
            MostDetailedMip: 0,
            MipLevels: u32::MAX,
        };
        let view_anonymous = D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
            Texture2D: tex2d_srv,
        };
        let view_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
            Anonymous: view_anonymous,
        };
        let mut view_ptr: Option<ID3D11ShaderResourceView> = None;
        let view = unsafe {
            device.CreateShaderResourceView(&texture, Some(&view_desc), Some(&mut view_ptr))
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| view_ptr.ok_or_else(|| anyhow!("no shader resource view")))?;
        let view = vec![Some(view)];
        log::info!("Loaded {:?} texture from {path:?}!", format);
        Ok(Self {
            texture,
            view,
            dimensions: dimensions.into(),
        })
    }

    pub fn generate_mips(&self, device_context: &ID3D11DeviceContext) {
        unsafe {
            let mut itty = self.view.iter();
            while let Some(Some(view)) = itty.next() {
                device_context.GenerateMips(view);
            }
        }
    }
}
