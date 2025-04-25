use {
    super::{
        entity::Entity,
        entitycontroller::EntityController,
        shader::{Shader, ShaderDescription},
    },
    crate::SETTINGS,
    anyhow::anyhow,
    glam::{Affine3A, Mat4, Vec3},
    glob::Paths,
    itertools::Itertools,
    nexus::{imgui::Io, paths::get_addon_dir, AddonApi},
    std::{collections::HashMap, path::Path, rc::Rc},
    tokio::sync::mpsc::Receiver,
    windows::Win32::Graphics::{
        Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        Direct3D11::{
            ID3D11Buffer, ID3D11DepthStencilState, ID3D11Device, ID3D11RasterizerState,
            ID3D11RenderTargetView, ID3D11Texture2D, D3D11_BIND_CONSTANT_BUFFER, D3D11_BUFFER_DESC,
            D3D11_COMPARISON_ALWAYS, D3D11_COMPARISON_LESS, D3D11_CULL_BACK,
            D3D11_DEFAULT_STENCIL_READ_MASK, D3D11_DEFAULT_STENCIL_WRITE_MASK,
            D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ALL,
            D3D11_FILL_SOLID, D3D11_RASTERIZER_DESC, D3D11_STENCIL_OP_DECR, D3D11_STENCIL_OP_INCR,
            D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
        },
        Dxgi::IDXGISwapChain,
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
    entities: Vec<Rc<Entity>>,
    shader_entity_map: ShaderEntityMap,
    depth_stencil_state: ID3D11DepthStencilState,
    constant_buffer: ID3D11Buffer,
    constant_buffer_data: ConstantBufferData,
    viewport: D3D11_VIEWPORT,
    device: ID3D11Device,
    swap_chain: IDXGISwapChain,
    aspect_ratio: Option<f32>,
    display_size: Option<[f32; 2]>,
}

#[repr(C, align(16))]
pub struct InstanceBufferData {
    pub model: Mat4,
}

impl InstanceBufferData {
    pub fn rotate(&mut self, dt: f32) {
        self.model = self.model * Affine3A::from_rotation_z(dt);
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

impl DrawState {
    pub fn setup_shaders(
        addon_dir: &Path,
        device: &ID3D11Device,
    ) -> anyhow::Result<HashMap<String, Rc<Shader>>> {
        log::info!("Beginning shader setup!");
        let shader_folder = addon_dir.join("shaders");
        let mut shader_descriptions: Vec<ShaderDescription> = Vec::new();
        let mut shaders: HashMap<String, Rc<Shader>> = HashMap::new();
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
    ) -> anyhow::Result<Vec<Rc<Entity>>> {
        log::info!("Beginning entity setup!");
        let entity_controller = EntityController::load_desc(addon_dir)?;
        let entities = entity_controller.load(device)?;
        log::info!("Finished entity setup. {} entities loaded!", entities.len());
        Ok(entities)
    }

    pub fn setup_shader_entity_map(
        shaders: &HashMap<String, Rc<Shader>>,
        entities: &[Rc<Entity>],
    ) -> ShaderEntityMap {
        log::info!("Beginning shader entity map setup!");
        let mut shader_entity_map = Vec::new();
        let entity_shader_combinations = entities
            .iter()
            .map(|e| (e.vertex_shader.clone(), e.pixel_shader.clone()))
            .unique();
        log::info!(
            "There are {:?} unique shader combinations across your currently loaded entities!",
            entity_shader_combinations.size_hint()
        );
        for combination in entity_shader_combinations {
            let entities_for_combination: Vec<Rc<Entity>> = entities
                .iter()
                .filter(|e| (e.vertex_shader.clone(), e.pixel_shader.clone()) == combination)
                .cloned()
                .collect();
            log::info!(
                "For the shader combination ({},{}) there are {} entities.",
                combination.0,
                combination.1,
                entities_for_combination.len()
            );
            shader_entity_map.push((
                shaders[&combination.0].clone(),
                shaders[&combination.1].clone(),
                entities_for_combination,
            ));
        }
        log::info!("Finished shader entity map setup!");
        shader_entity_map
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
    pub fn setup(receiver: Receiver<SpaceEvent>) -> anyhow::Result<Self> {
        let addon_dir = get_addon_dir("Taimi").expect("Invalid addon dir");
        let addon_api = AddonApi::get();
        log::info!("Getting d3d11 device");
        let d3d11_device = addon_api
            .get_d3d11_device()
            .ok_or_else(|| anyhow!("you will not reach heaven today, how are you here?"))?;
        log::info!("Getting d3d11 device swap chain");
        let d3d11_swap_chain = &addon_api.swap_chain;

        let shaders = Self::setup_shaders(&addon_dir, &d3d11_device)?;
        let entities = Self::setup_entities(&addon_dir, &d3d11_device)?;
        //let shader_entity_map = Self::setup_shader_entity_map(&shaders, &entities);
        let shader_entity_map = Vec::new();
        let constant_buffer = Self::setup_constant_buffer(&d3d11_device)?;

        let viewport = D3D11_VIEWPORT {
            TopLeftX: 1000.0,
            TopLeftY: 0.0,
            Width: 512.0,
            Height: 512.0,
            MinDepth: 0.0,
            MaxDepth: 10.0,
        };

        log::info!("Setting up framebuffer");
        let framebuffer: ID3D11Texture2D =
            unsafe { d3d11_swap_chain.GetBuffer(0) }.map_err(anyhow::Error::from)?;

        log::info!("Setting up render target view");
        let mut render_target_view_ptr: Option<ID3D11RenderTargetView> = None;
        /*let render_target_view_desc = D3D11_RENDER_TARGET_VIEW_DESC {
            Format: DXGI_FORMAT_UNKNOWN,
            ViewDimension: D3D11_RTV_DIMENSION_UNKNOWN,
            Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_RENDER_TARGET_VIEW_DESC_0::default(),
        };*/
        let render_target_view = unsafe {
            d3d11_device.CreateRenderTargetView(
                &framebuffer,
                None,
                Some(&mut render_target_view_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| render_target_view_ptr.ok_or_else(|| anyhow!("no render target view")))?;

        let depth_stencil_frontface_desc = D3D11_DEPTH_STENCILOP_DESC {
            StencilFunc: D3D11_COMPARISON_ALWAYS,
            StencilDepthFailOp: D3D11_STENCIL_OP_INCR,
            StencilFailOp: D3D11_STENCIL_OP_KEEP,
            StencilPassOp: D3D11_STENCIL_OP_KEEP,
        };
        let depth_stencil_backface_desc = D3D11_DEPTH_STENCILOP_DESC {
            StencilFunc: D3D11_COMPARISON_ALWAYS,
            StencilDepthFailOp: D3D11_STENCIL_OP_DECR,
            StencilFailOp: D3D11_STENCIL_OP_KEEP,
            StencilPassOp: D3D11_STENCIL_OP_KEEP,
        };
        log::info!("Setting up depth stencil");
        let depth_stencil_state_desc = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: true.into(),
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ALL,
            DepthFunc: D3D11_COMPARISON_LESS,
            StencilEnable: true.into(),
            StencilReadMask: D3D11_DEFAULT_STENCIL_READ_MASK as u8,
            StencilWriteMask: D3D11_DEFAULT_STENCIL_WRITE_MASK as u8,
            FrontFace: depth_stencil_frontface_desc,
            BackFace: depth_stencil_backface_desc,
        };
        let mut depth_stencil_state_ptr: Option<ID3D11DepthStencilState> = None;
        let depth_stencil_state = unsafe {
            d3d11_device.CreateDepthStencilState(
                &depth_stencil_state_desc,
                Some(&mut depth_stencil_state_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| depth_stencil_state_ptr.ok_or_else(|| anyhow!("no depth stencil state")))?;

        log::info!("Setting up rasterizer state");
        let rasterizer_state_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_BACK,
            FrontCounterClockwise: true.into(),
            DepthBias: 0,
            DepthBiasClamp: 0.0,
            SlopeScaledDepthBias: 0.0,
            DepthClipEnable: true.into(),
            ScissorEnable: false.into(),
            MultisampleEnable: false.into(),
            AntialiasedLineEnable: false.into(),
        };
        let mut rasterizer_state_ptr: Option<ID3D11RasterizerState> = None;
        let rasterizer_state = unsafe {
            d3d11_device
                .CreateRasterizerState(&rasterizer_state_desc, Some(&mut rasterizer_state_ptr))
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| rasterizer_state_ptr.ok_or_else(|| anyhow!("no rasterizer state")))?;

        log::info!("Setting up device context");
        Ok(DrawState {
            receiver,
            device: d3d11_device,
            swap_chain: d3d11_swap_chain.clone(),
            render_target_view: [Some(render_target_view)],
            entities,
            shader_entity_map,
            rasterizer_state,
            shaders,
            depth_stencil_state,
            viewport,
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
                                        0.000001,
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
                    //device_context.RSSetViewports(Some(&[self.viewport]));
                    //device_context.OMSetRenderTargets(Some(&self.render_target_view), None);
                    device_context.OMSetDepthStencilState(&self.depth_stencil_state, 1);
                    device_context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

                    for entity in &self.entities {
                        if let (Some(vs), Some(ps)) = (
                            &self.shaders.get(&entity.vertex_shader),
                            self.shaders.get(&entity.pixel_shader),
                        ) {
                            vs.set(&device_context);
                            ps.set(&device_context);
                            entity.set_and_draw(&device_context);
                        }
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
