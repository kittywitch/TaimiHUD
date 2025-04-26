use {
    super::{
        depthhandler::DepthHandler,
        entity::Entity,
        entitycontroller::EntityController,
        shader::{Shader, ShaderDescription, Shaders},
    }, crate::SETTINGS, anyhow::anyhow, arc_atomic::AtomicArc, glam::{Affine3A, Mat4, Vec3, Vec4}, glob::Paths, image::ImageReader, itertools::Itertools, nexus::{imgui::Io, paths::get_addon_dir, AddonApi}, std::{collections::HashMap, path::Path, rc::Rc, sync::{Arc, OnceLock}}, tokio::sync::mpsc::Receiver, windows::Win32::Graphics::{
        Direct3D::{D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D11_SRV_DIMENSION_TEXTURE2D},
        Direct3D11::{
            ID3D11Buffer, ID3D11DepthStencilState, ID3D11DepthStencilView, ID3D11Device,
            ID3D11DeviceContext, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState,
            ID3D11ShaderResourceView, ID3D11Texture2D, D3D11_BIND_CONSTANT_BUFFER,
            D3D11_BIND_DEPTH_STENCIL, D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE,
            D3D11_BUFFER_DESC, D3D11_CLEAR_DEPTH, D3D11_CLEAR_STENCIL, D3D11_COMPARISON_ALWAYS,
            D3D11_COMPARISON_LESS,
            D3D11_CULL_BACK, D3D11_DEFAULT_STENCIL_READ_MASK,
            D3D11_DEFAULT_STENCIL_WRITE_MASK, D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC,
            D3D11_DEPTH_STENCIL_VIEW_DESC, D3D11_DEPTH_STENCIL_VIEW_DESC_0,
            D3D11_DEPTH_WRITE_MASK_ALL, D3D11_DSV_DIMENSION_TEXTURE2D,
            D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_RASTERIZER_DESC, D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_SAMPLER_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC,
            D3D11_SHADER_RESOURCE_VIEW_DESC_0,
            D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA,
            D3D11_TEX2D_DSV, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC,
            D3D11_TEXTURE_ADDRESS_WRAP, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
        },
        Dxgi::{
            Common::{
                DXGI_FORMAT_D24_UNORM_S8_UINT,
                DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_SAMPLE_DESC,
            },
            IDXGISwapChain,
        },
    }
};

static PERSPECTIVEINPUTDATA: OnceLock<Arc<AtomicArc<PerspectiveInputData>>> = OnceLock::new();

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PerspectiveInputData {
    pub front: Vec3,
    pub pos: Vec3,
    pub fov: f32,
}

impl PerspectiveInputData {
    pub fn create() {
        let aarc = Arc::new(AtomicArc::new(Arc::new(Self::default())));
        let _ = PERSPECTIVEINPUTDATA.set(aarc);
    }

    pub fn read() -> Option<Arc<Self>> {
        Some(PERSPECTIVEINPUTDATA.get()?.load())
    }

    pub fn swap_camera(front: Vec3, pos: Vec3) {
        if let Some(data) = PERSPECTIVEINPUTDATA.get() {
            let pdata = data.load();
            data.store(Arc::new(PerspectiveInputData {
                fov: pdata.fov,
                front,
                pos
            }))

        }
    }

    pub fn swap_fov(fov: f32) {
        if let Some(data) = PERSPECTIVEINPUTDATA.get() {
            let pdata = data.load();
            data.store(Arc::new(PerspectiveInputData {
                fov,
                front: pdata.front,
                pos: pdata.pos,
            }))

        }
    }
}

pub struct DrawState {
    depth_handler: DepthHandler,
    perspective_handler: PerspectiveHandler,

    shaders: Shaders,
    entities: Vec<Entity>,
    sampler_state: Vec<Option<ID3D11SamplerState>>,
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
#[derive(Debug)]
struct PerspectiveData {
    view: Mat4,
    projection: Mat4,
}

struct PerspectiveHandler {
    constant_buffer: ID3D11Buffer,
    constant_buffer_data: PerspectiveData,
    aspect_ratio: f32,
    up: Vec3,
    near: f32,
    far: f32,
    last_display_size: [f32; 2],
}

impl PerspectiveHandler {
    pub fn setup(device: &ID3D11Device, display_size: &[f32; 2]) -> anyhow::Result<Self> {
        let aspect_ratio = display_size[0] / display_size[1];
        let constant_buffer = Self::create_constant_buffer(device)?;
        let constant_buffer_data = PerspectiveData {
            view: Mat4::IDENTITY,
            projection: Mat4::IDENTITY,
        };
        Ok(Self {
            up: Vec3::new(0.0, 1.0, 0.0),
            aspect_ratio,
            last_display_size: *display_size,
            constant_buffer,
            constant_buffer_data,
            near: 0.1,
            far: 1000.0,
        })
    }

    pub fn update_perspective(&mut self, display_size: &[f32; 2]) {
        if let Some(data ) = PerspectiveInputData::read() {
            if *display_size != self.last_display_size {
                self.aspect_ratio = display_size[0] / display_size[1];
                self.last_display_size = *display_size;
            }

            self.constant_buffer_data.view = Mat4::look_to_lh(
                data.pos,
                data.front,
                self.up,
            );
            self.constant_buffer_data.projection = Mat4::perspective_lh(
                data.fov,
                self.aspect_ratio,
                self.near,
                self.far,
            );

        }
    }

    pub fn create_constant_buffer(device: &ID3D11Device) -> anyhow::Result<ID3D11Buffer> {
        let constant_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of::<PerspectiveData>() as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let constant_subresource_data = D3D11_SUBRESOURCE_DATA::default();

        let mut constant_buffer_ptr: Option<ID3D11Buffer> = None;
        let constant_buffer = unsafe {
            device.CreateBuffer(
                &constant_buffer_desc,
                Some(&constant_subresource_data),
                Some(&mut constant_buffer_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| constant_buffer_ptr.ok_or_else(|| anyhow!("no constant buffer")))?;

        Ok(constant_buffer)
    }

    fn update_cb(&mut self, device_context: &ID3D11DeviceContext) {
        unsafe {
            device_context.UpdateSubresource(
                &self.constant_buffer,
                0,
                None,
                &self.constant_buffer_data as *const _ as *const _,
                0,
                0,
            );
        }
    }
    fn set_cb(&self, device_context: &ID3D11DeviceContext, slot: u32) {
        unsafe {
            device_context
                .VSSetConstantBuffers(slot, Some(&[Some(self.constant_buffer.clone())]));
        }
    }
    pub fn set(&mut self, device_context: &ID3D11DeviceContext, slot: u32) {
        self.set_cb(device_context, slot);
        self.update_cb(device_context);
    }
}

impl DrawState {
    pub fn setup_entities(
        addon_dir: &Path,
        device: &ID3D11Device,
        shaders: &Shaders,
    ) -> anyhow::Result<Vec<Entity>> {
        log::info!("Beginning entity setup!");
        let entity_controller = EntityController::load_desc(addon_dir)?;
        let entities = entity_controller.load(device, shaders)?;
        log::info!("Finished entity setup. {} entities loaded!", entities.len());
        Ok(entities)
    }


    pub fn setup_sampler(device: &ID3D11Device) -> anyhow::Result<ID3D11SamplerState> {
        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_WRAP,
            AddressV: D3D11_TEXTURE_ADDRESS_WRAP,
            AddressW: D3D11_TEXTURE_ADDRESS_WRAP,
            MipLODBias: 0.0,
            MinLOD: 0.0,
            MaxLOD: f32::MAX,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            BorderColor: Vec4::new(0.0, 0.0, 0.0, 0.0).into(),
        };
        let mut sampler_state_ptr: Option<ID3D11SamplerState> = None;
        let sampler_state =
            unsafe { device.CreateSamplerState(&sampler_desc, Some(&mut sampler_state_ptr)) }
                .map_err(anyhow::Error::from)
                .and_then(|()| sampler_state_ptr.ok_or_else(|| anyhow!("no sampler state")))?;
        Ok(sampler_state)
    }

    pub fn setup(display_size: [f32; 2]) -> anyhow::Result<Self> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let addon_api = AddonApi::get();

        log::info!("Getting d3d11 device");
        let device = addon_api
            .get_d3d11_device()
            .ok_or_else(|| anyhow!("you will not reach heaven today, how are you here?"))?;
        log::info!("Getting d3d11 device swap chain");
        let swap_chain = &addon_api.swap_chain;

        PerspectiveInputData::create();

        let shaders = Shaders::setup(&addon_dir, &device)?;
        let entities = Self::setup_entities(&addon_dir, &device, &shaders)?;
        let perspective_handler = PerspectiveHandler::setup(&device, &display_size)?;

        let depth_handler = DepthHandler::create(&display_size, &device, swap_chain)?;
        let sampler_state = vec![Self::setup_sampler(&device).ok()];

        log::info!("Setting up device context");
        let device_context = unsafe { device.GetImmediateContext().expect("I lost my context!") };

        for entity in entities.iter() {
            if let Some(texture) = &entity.model.texture {
                texture.generate_mips(&device_context);
            }
        }
        Ok(DrawState {
            depth_handler,
            perspective_handler,

            device,
            swap_chain: swap_chain.clone(),
            entities,
            shaders,
            sampler_state,
            aspect_ratio: None,
            display_size: None,
        })
    }
    pub fn draw(&mut self, io: &Io) {
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            if settings.enable_katrender {
                let display_size = io.display_size;

                self.perspective_handler.update_perspective(&display_size);
                unsafe {
                    let slot = 0;

                    let device_context = self
                        .device
                        .GetImmediateContext()
                        .expect("I lost my context!");

                    self.perspective_handler.set(&device_context, slot);
                    for entity in &self.entities {
                        entity.rotate(io.delta_time);
                        device_context.UpdateSubresource(
                            &entity.instance_buffer,
                            0,
                            None,
                            entity.model_matrix.borrow().as_ptr() as *const _ as *const _,
                            0,
                            0,
                        );
                    }

                    self.depth_handler.setup(&device_context);
                    device_context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
                    device_context.PSSetSamplers(slot, Some(&self.sampler_state));
                    for entity in &self.entities {
                        if let Some(texture) = &entity.model.texture {
                            texture.set(&device_context, slot);
                        }
                        entity.vertex_shader.set(&device_context);
                        entity.pixel_shader.set(&device_context);
                        entity.set_and_draw(&device_context);
                    }
                }
            }
        }
    }
}
