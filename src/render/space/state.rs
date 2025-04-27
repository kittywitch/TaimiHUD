use {
    super::{
        depthhandler::DepthHandler, entity::Entity, entitycontroller::EntityController,
        perspectivehandler::PerspectiveHandler, shader::Shaders,
    },
    crate::{render::space::perspectiveinputdata::PerspectiveInputData, SETTINGS},
    anyhow::anyhow,
    glam::{Affine3A, Mat4, Vec3, Vec4},
    itertools::Itertools,
    nexus::{imgui::Io, paths::get_addon_dir, AddonApi},
    std::path::Path,
    windows::Win32::Graphics::{
        Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        Direct3D11::{
            ID3D11Device, ID3D11SamplerState, D3D11_COMPARISON_ALWAYS,
            D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_WRAP,
        },
        Dxgi::IDXGISwapChain,
    },
};

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
        let mut entities = Self::setup_entities(&addon_dir, &device, &shaders)?;
        let perspective_handler = PerspectiveHandler::setup(&device, &display_size)?;

        let depth_handler = DepthHandler::create(&display_size, &device, swap_chain)?;
        let sampler_state = vec![Self::setup_sampler(&device).ok()];

        log::info!("Setting up device context");
        let device_context = unsafe { device.GetImmediateContext().expect("I lost my context!") };

        let path = addon_dir.join("QuitarHero_Hero-Timers/timers/Assets/Raids/Deimos.png");
        if let Ok(quad) = Entity::quad(&device, &shaders, Some(&path)) {
            entities.push(quad);
        }
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
