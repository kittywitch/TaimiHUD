use {
    super::{
        entity::Entity,
        entitycontroller::EntityController,
        shader::{Shader, ShaderDescription},
    },
    crate::SETTINGS,
    anyhow::anyhow,
    glam::{Affine3A, Mat4, Vec3, Vec4},
    glob::Paths,
    itertools::Itertools,
    nexus::{imgui::Io, paths::get_addon_dir, AddonApi},
    std::{collections::HashMap, path::Path, rc::Rc},
    tokio::sync::mpsc::Receiver,
    windows::Win32::Graphics::{
        Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        Direct3D11::{
            ID3D11Buffer, ID3D11DepthStencilState, ID3D11DepthStencilView, ID3D11Device, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11Texture2D, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_DEPTH_STENCIL, D3D11_BUFFER_DESC, D3D11_CLEAR_DEPTH, D3D11_CLEAR_STENCIL, D3D11_COMPARISON_ALWAYS, D3D11_COMPARISON_EQUAL, D3D11_COMPARISON_GREATER, D3D11_COMPARISON_GREATER_EQUAL, D3D11_COMPARISON_LESS, D3D11_COMPARISON_LESS_EQUAL, D3D11_CULL_BACK, D3D11_CULL_FRONT, D3D11_CULL_NONE, D3D11_DEFAULT_STENCIL_READ_MASK, D3D11_DEFAULT_STENCIL_WRITE_MASK, D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_STENCIL_VIEW_DESC, D3D11_DEPTH_STENCIL_VIEW_DESC_0, D3D11_DEPTH_WRITE_MASK_ALL, D3D11_DSV_DIMENSION_TEXTURE2D, D3D11_DSV_READ_ONLY_DEPTH, D3D11_FILL_SOLID, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RTV_DIMENSION_TEXTURE2D, D3D11_STENCIL_OP_DECR, D3D11_STENCIL_OP_INCR, D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA, D3D11_TEX2DMS_ARRAY_DSV, D3D11_TEX2D_DSV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT
        },
        Dxgi::{Common::{DXGI_FORMAT_D24_UNORM_S8_UINT, DXGI_FORMAT_D32_FLOAT_S8X24_UINT, DXGI_SAMPLE_DESC}, IDXGISwapChain},
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

impl DrawState {
    pub fn setup_shaders(
        addon_dir: &Path,
        device: &ID3D11Device,
    ) -> anyhow::Result<Shaders> {
        log::info!("Beginning shader setup!");
        let shader_folder = addon_dir.join("shaders");
        let mut shader_descriptions: Vec<ShaderDescription> = Vec::new();
        let mut shaders: Shaders = HashMap::new();
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
                let shader = Rc::new(Shader::create(&shader_folder, device, &shader_description)?);
                shaders.insert(shader_description.identifier, shader);
            }
        }
        log::info!("Finished shader setup. {} shaders loaded!", shaders.len());
        Ok(shaders)
    }

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

    pub fn setup_constant_buffer(device: &ID3D11Device) -> anyhow::Result<ID3D11Buffer> {
        let constant_buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of::<ConstantBufferData>() as u32,
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

    pub fn setup_viewport(display_size: &[f32; 2]) -> D3D11_VIEWPORT {
        log::debug!("Setting up viewport with dimensions ({},{})", display_size[0], display_size[1]);
        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: display_size[0],
            Height: display_size[1],
            MinDepth: 0.0,
            MaxDepth: 1000.0,
        };
        log::debug!("Set up viewport with dimensions ({},{})", display_size[0], display_size[1]);
        viewport
    }

    pub fn setup_framebuffer(swap_chain: &IDXGISwapChain) -> anyhow::Result<ID3D11Texture2D> {
        log::info!("Setting up framebuffer");
        let framebuffer: ID3D11Texture2D =
            unsafe { swap_chain.GetBuffer(0) }.map_err(anyhow::Error::from)?;
        log::info!("Set up framebuffer");
        Ok(framebuffer)
    }

    pub fn setup_render_target_view(device: &ID3D11Device, framebuffer: &ID3D11Texture2D) -> anyhow::Result<ID3D11RenderTargetView> {
        log::debug!("Setting up render target view");
        let mut render_target_view_ptr: Option<ID3D11RenderTargetView> = None;
        let render_target_view = unsafe {
            device.CreateRenderTargetView(
                framebuffer,
                None,
                Some(&mut render_target_view_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| render_target_view_ptr.ok_or_else(|| anyhow!("no render target view")))?;
        log::debug!("Set up render target view");
        Ok(render_target_view)
    }

    pub fn setup_depth_stencil_buffer(device: &ID3D11Device, display_size: &[f32; 2]) -> anyhow::Result<ID3D11Texture2D> {
        log::info!("Setting up depth stencil buffer");
        let depth_stencil_buffer_sample_desc = DXGI_SAMPLE_DESC {
            Count:  1,
            Quality: 0,
        };
        let depth_stencil_buffer_desc = D3D11_TEXTURE2D_DESC {
            Width: display_size[0] as u32,
            Height: display_size[1] as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_D24_UNORM_S8_UINT,
            SampleDesc: depth_stencil_buffer_sample_desc,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_DEPTH_STENCIL.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        log::info!("Set up depth stencil buffer");
        let mut depth_stencil_buffer_ptr: Option<ID3D11Texture2D> = None;
        let depth_stencil_buffer = unsafe {
            device.CreateTexture2D(&depth_stencil_buffer_desc, None, Some(&mut depth_stencil_buffer_ptr))
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| depth_stencil_buffer_ptr.ok_or_else(|| anyhow!("no depth stencil buffer")))?;
        Ok(depth_stencil_buffer)

    }

    pub fn setup_depth_stencil_view(device: &ID3D11Device, buffer: &ID3D11Texture2D) -> anyhow::Result<ID3D11DepthStencilView> {
        let dsv_tex2d = D3D11_TEX2D_DSV{
            MipSlice: 0,
        };
        let depth_stencil_view_anonymous = D3D11_DEPTH_STENCIL_VIEW_DESC_0 {
            Texture2D: dsv_tex2d,

        };

        let depth_stencil_view_desc = D3D11_DEPTH_STENCIL_VIEW_DESC {
            Format: DXGI_FORMAT_D24_UNORM_S8_UINT,
            ViewDimension: D3D11_DSV_DIMENSION_TEXTURE2D,
            Flags: 0,
            Anonymous: depth_stencil_view_anonymous,
        
        };
        log::info!("Setting up depth stencil view");
        let mut depth_stencil_view_ptr: Option<ID3D11DepthStencilView> = None;
        let depth_stencil_view = unsafe {
            device.CreateDepthStencilView(buffer, Some(&depth_stencil_view_desc), Some(&mut depth_stencil_view_ptr))
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| depth_stencil_view_ptr.ok_or_else(|| anyhow!("no depth stencil view")))?;
        log::info!("Set up depth stencil view");
        Ok(depth_stencil_view)
    }

    pub fn setup_depth_stencil_state(device: &ID3D11Device) -> anyhow::Result<ID3D11DepthStencilState> {
        log::info!("Setting up depth stencil state");
        let depth_stencil_frontface_desc = D3D11_DEPTH_STENCILOP_DESC {
            StencilFunc: D3D11_COMPARISON_ALWAYS,
            StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
            StencilFailOp: D3D11_STENCIL_OP_KEEP,
            StencilPassOp: D3D11_STENCIL_OP_KEEP,
        };
        let depth_stencil_backface_desc = D3D11_DEPTH_STENCILOP_DESC {
            StencilFunc: D3D11_COMPARISON_ALWAYS,
            StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
            StencilFailOp: D3D11_STENCIL_OP_KEEP,
            StencilPassOp: D3D11_STENCIL_OP_KEEP,
        };
        let depth_stencil_state_desc = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: true.into(),
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
            DepthFunc: D3D11_COMPARISON_LESS,
            StencilEnable: false.into(),
            StencilReadMask: D3D11_DEFAULT_STENCIL_READ_MASK as u8,
            StencilWriteMask: D3D11_DEFAULT_STENCIL_WRITE_MASK as u8,
            FrontFace: depth_stencil_frontface_desc,
            BackFace: depth_stencil_backface_desc,
        };
        let mut depth_stencil_state_ptr: Option<ID3D11DepthStencilState> = None;
        let depth_stencil_state = unsafe {
            device.CreateDepthStencilState(
                &depth_stencil_state_desc,
                Some(&mut depth_stencil_state_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| depth_stencil_state_ptr.ok_or_else(|| anyhow!("no depth stencil state")))?;
        log::info!("Set up depth stencil state");
        Ok(depth_stencil_state)
    }

    pub fn setup_rasterizer_state(device: &ID3D11Device) -> anyhow::Result<ID3D11RasterizerState> {
        log::info!("Setting up rasterizer state");
        let rasterizer_state_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_BACK,
            FrontCounterClockwise: false.into(),
            DepthBias: 0,
            DepthBiasClamp: 0.0,
            SlopeScaledDepthBias: 0.0,
            DepthClipEnable: false.into(),
            ScissorEnable: false.into(),
            MultisampleEnable: false.into(),
            AntialiasedLineEnable: false.into(),
        };
        let mut rasterizer_state_ptr: Option<ID3D11RasterizerState> = None;
        let rasterizer_state = unsafe {
            device
                .CreateRasterizerState(&rasterizer_state_desc, Some(&mut rasterizer_state_ptr))
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| rasterizer_state_ptr.ok_or_else(|| anyhow!("no rasterizer state")))?;
        log::info!("Set up rasterizer state");
        Ok(rasterizer_state)
    }

    pub fn setup(receiver: Receiver<SpaceEvent>, display_size: [f32; 2]) -> anyhow::Result<Self> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let addon_api = AddonApi::get();
        log::info!("Getting d3d11 device");
        let device = addon_api
            .get_d3d11_device()
            .ok_or_else(|| anyhow!("you will not reach heaven today, how are you here?"))?;
        log::info!("Getting d3d11 device swap chain");
        let d3d11_swap_chain = &addon_api.swap_chain;

        let shaders = Self::setup_shaders(&addon_dir, &device)?;
        let entities = Self::setup_entities(&addon_dir, &device, &shaders)?;
        //let shader_entity_map = Self::setup_shader_entity_map(&shaders, &entities);
        let shader_entity_map = Vec::new();
        let constant_buffer = Self::setup_constant_buffer(&device)?;
        let framebuffer = Self::setup_framebuffer(d3d11_swap_chain)?;
        let render_target_view = Self::setup_render_target_view(&device, &framebuffer)?;
        let depth_stencil_state = Self::setup_depth_stencil_state(&device)?;
        let depth_stencil_buffer = Self::setup_depth_stencil_buffer(&device, &display_size)?;
        let depth_stencil_view = Self::setup_depth_stencil_view(&device, &depth_stencil_buffer)?;
        let rasterizer_state = Self::setup_rasterizer_state(&device)?;
        let viewport = Self::setup_viewport(&display_size);

        log::info!("Setting up device context");
        Ok(DrawState {
            receiver,
            device,
            swap_chain: d3d11_swap_chain.clone(),
            render_target_view: [Some(render_target_view)],
            entities,
            shader_entity_map,
            rasterizer_state,
            viewport,
            shaders,
            depth_stencil_state,
            depth_stencil_buffer,
            depth_stencil_view,
            constant_buffer,
            draw_data: None,
            aspect_ratio: None,
            display_size: None,
            constant_buffer_data: ConstantBufferData {
                view: Mat4::IDENTITY,
                projection: Mat4::IDENTITY,
            },
        })
    }
    pub fn draw(&mut self, io: &Io) {
        if let Some(settings) = SETTINGS.get().and_then(|settings| settings.try_read().ok()) {
            if settings.enable_katrender {
                let display_size = io.display_size;
                if self.aspect_ratio.is_none() || self.display_size != Some(display_size) {
                    self.aspect_ratio = Some(display_size[0] / display_size[1]);
                    self.display_size = Some(display_size);
                }
                use SpaceEvent::*;
                match self.receiver.try_recv() {
                    Ok(event) => {
                        match event {
                            Update(data) => {
                                //log::info!("{:?}", data);
                                if let Some(aspect_ratio) = self.aspect_ratio {
                                    let up = Vec3::new(0.0, 1.0, 0.0);
                                    self.constant_buffer_data.view = Mat4::look_to_lh(
                                        data.camera_position,
                                        data.camera_front,
                                        up,
                                    );
                                    self.constant_buffer_data.projection = Mat4::perspective_lh(
                                        70.0f32.to_radians(),
                                        aspect_ratio,
                                        0.1,
                                        1000.0,
                                    );
                                }
                                self.draw_data = Some(data);
                            }
                        }
                    }

                    Err(_error) => (),
                }

                //self.constant_buffer_data.rotate_model(io.delta_time);
                unsafe {
                    let device_context = self
                        .device
                        .GetImmediateContext()
                        .expect("I lost my context!");
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

                    device_context.RSSetState(&self.rasterizer_state);
                    device_context
                        .VSSetConstantBuffers(0, Some(&[Some(self.constant_buffer.clone())]));
                    device_context.UpdateSubresource(
                        &self.constant_buffer,
                        0,
                        None,
                        &self.constant_buffer_data as *const _ as *const _,
                        0,
                        0,
                    );
                    device_context.RSSetViewports(Some(&[self.viewport]));
                    device_context.OMSetRenderTargets(Some(&self.render_target_view), Some(&self.depth_stencil_view));
                    device_context.ClearDepthStencilView(&self.depth_stencil_view, D3D11_CLEAR_DEPTH.0|D3D11_CLEAR_STENCIL.0, 1.0, 0);
                    //device_context.OMSetRenderTargets(None, Some(&self.depth_stencil_view));
                    device_context.OMSetDepthStencilState(&self.depth_stencil_state, 1);
                    device_context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

                    for entity in &self.entities {
                        entity.vertex_shader.set(&device_context);
                        entity.pixel_shader.set(&device_context);
                        entity.set_and_draw(&device_context);
                    }

                    /*for (vs, ps, entities) in self.shader_entity_map.iter() {
                        vs.set(&device_context);
                        ps.set(&device_context);
                        let vertex_buffers: Vec<VertexBuffer> = entities
                            .iter()
                            .map(
                                |e|
                                e.vertex_buffer.to_owned()
                            ).collect();
                        VertexBuffer::set_and_draw(vertex_buffers.as_slice(), &device_context);
                    }*/
                }
            }
        }
    }
}
