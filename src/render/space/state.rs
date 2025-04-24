use {
    super::{model::{Model, Vertex, VertexBuffer}, shader::{Shader, ShaderDescription, ShaderKind}}, crate::SETTINGS, anyhow::anyhow, glam::{Mat4, Vec3}, glob::Paths, nexus::{imgui::Io, paths::get_addon_dir, AddonApi}, std::{
        collections::HashMap, ffi::{c_char, CStr}, mem::offset_of, path::{Path, PathBuf}, slice::from_raw_parts
    }, tokio::sync::mpsc::Receiver, windows::Win32::Graphics::{
        Direct3D::{
            Fxc::{D3DCompileFromFile, D3DCOMPILE_DEBUG},
            ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        },
        Direct3D11::{
            ID3D11Buffer, ID3D11DepthStencilState, ID3D11Device, ID3D11InputLayout,
            ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11Texture2D,
            ID3D11VertexShader, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_BIND_CONSTANT_BUFFER,
            D3D11_BUFFER_DESC, D3D11_COMPARISON_ALWAYS, D3D11_COMPARISON_LESS, D3D11_CULL_BACK,
            D3D11_DEFAULT_STENCIL_READ_MASK, D3D11_DEFAULT_STENCIL_WRITE_MASK,
            D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ALL,
            D3D11_FILL_SOLID, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA,
            D3D11_RASTERIZER_DESC, D3D11_STENCIL_OP_DECR, D3D11_STENCIL_OP_INCR,
            D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT,
        },
        Dxgi::{
            Common::{DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32_FLOAT},
            IDXGISwapChain,
        },
    }, windows_strings::*
};

#[derive(Debug)]
pub struct DrawData {
    pub player_position: Option<Vec3>,
    pub camera_front: Vec3,
    pub camera_up: Vec3,
    pub camera_position: Vec3,
}

pub struct DrawState {
    receiver: Receiver<SpaceEvent>,
    draw_data: Option<DrawData>,
    render_target_view: [Option<ID3D11RenderTargetView>; 1],
    shaders: HashMap<String, Shader>,
    rasterizer_state: ID3D11RasterizerState,
    vertex_buffers: Vec<VertexBuffer>,
    depth_stencil_state: ID3D11DepthStencilState,
    constant_buffer: ID3D11Buffer,
    constant_buffer_data: ConstantBufferData,
    viewport: D3D11_VIEWPORT,
    device: ID3D11Device,
    swap_chain: IDXGISwapChain,
    aspect_ratio: Option<f32>,
    display_size: Option<[f32; 2]>,
}

#[repr(C)]
struct ConstantBufferData {
    model: Mat4,
    view: Mat4,
    projection: Mat4,
}

impl ConstantBufferData {
    fn rotate_model(&mut self, dt: f32) {
        self.model *= Mat4::from_rotation_y(dt);
    }
}

pub enum SpaceEvent {
    Update(DrawData),
}

impl DrawState {

    pub fn setup_shaders(addon_dir: &Path, device: &ID3D11Device) -> anyhow::Result<HashMap<String, Shader>> {
        let shader_folder = addon_dir.join("shaders");
        let mut shader_descriptions: Vec<ShaderDescription> = Vec::new();
        let mut shaders: HashMap<String, Shader> = HashMap::new();
        if shader_folder.exists() {
            let shader_description_paths: Paths = glob::glob(shader_folder
                .join("*.shaderdesc")
                .to_str()
                .expect("Shader load pattern is unparseable"))?;
            for shader_description_path in shader_description_paths {
                let shader_description = ShaderDescription::load(&shader_folder.join(shader_description_path?))?;
                shader_descriptions.extend(shader_description);
            }
            for shader_description in shader_descriptions {
                let shader = Shader::create(&shader_folder, device, &shader_description)?;
                shaders.insert(shader_description.identifier, shader);
            };
        }
        Ok(shaders)

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

        let obj_file = addon_dir.join("horse.obj");
        let mut vertex_buffers = Vec::new();
        if obj_file.exists() {
            vertex_buffers.extend(Model::load_to_buffers(d3d11_device.clone(), &obj_file)?);
        }

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
            d3d11_device.CreateBuffer(
                &constant_buffer_desc,
                Some(&constant_subresource_data),
                Some(&mut constant_buffer_ptr),
            )
        }
        .map_err(anyhow::Error::from)
        .and_then(|()| constant_buffer_ptr.ok_or_else(|| anyhow!("no constant buffer")))?;

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
            vertex_buffers,
            render_target_view: [Some(render_target_view)],
            rasterizer_state,
            shaders,
            depth_stencil_state,
            viewport,
            constant_buffer,
            draw_data: None,
            aspect_ratio: None,
            display_size: None,
            constant_buffer_data: ConstantBufferData {
                //model:Mat4::from_translation(Vec3::new(0.25,0.5,1.0)),
                model: Mat4::from_translation(Vec3::new(-52.0, 130.0, 1.0))
                    * Mat4::from_scale(Vec3::new(0.25, 0.25, 0.25)),
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
                                    self.constant_buffer_data.view =
                                        Mat4::look_to_lh(data.camera_position, data.camera_front, up);
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

                self.constant_buffer_data.rotate_model(io.delta_time);
                unsafe {
                    let device_context = self
                        .device
                        .GetImmediateContext()
                        .expect("I lost my context!");
                    device_context.RSSetState(&self.rasterizer_state);
                    device_context.VSSetConstantBuffers(0, Some(&[Some(self.constant_buffer.clone())]));
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
                    if let (Some(vs), Some(ps)) = (self.shaders.get("generic_vs"), self.shaders.get("generic_ps")) {
                        vs.set(&device_context);
                        ps.set(&device_context);
                        VertexBuffer::set_and_draw(&self.vertex_buffers, &device_context);
                    }
                }
            }
        }
    }
}
