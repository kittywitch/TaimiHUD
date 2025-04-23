
use {
    crate::{
        controller::{Controller, ControllerEvent}, render::{RenderEvent, RenderState}, settings::SettingsLock
    }, anyhow::anyhow, arcdps::AgentOwned, glam::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles}, nexus::{
        event::{
            arc::{CombatData, COMBAT_LOCAL},
            event_consume, MumbleIdentityUpdate, MUMBLE_IDENTITY_UPDATED,
        }, gui::{register_render, render, RenderType}, imgui::Io, keybind::{keybind_handler, register_keybind_with_string}, paths::get_addon_dir, quick_access::add_quick_access, AddonApi, AddonFlags, UpdateProvider
    }, std::{
        ffi::{c_char, CStr, CString}, mem::offset_of, path::{Path, PathBuf}, ptr, slice::from_raw_parts, sync::{Mutex, OnceLock}, thread::{self, JoinHandle}
    }, tokio::sync::mpsc::{channel, Receiver, Sender}, windows::Win32::{Graphics::{Direct3D::{Fxc::{D3DCompileFromFile, D3DCOMPILE_DEBUG}, ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST}, Direct3D11::{ID3D11Buffer, ID3D11DepthStencilState, ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout, ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11Texture2D, ID3D11VertexShader, D3D11_APPEND_ALIGNED_ELEMENT, D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_VERTEX_BUFFER, D3D11_BUFFER_DESC, D3D11_COMPARISON_ALWAYS, D3D11_COMPARISON_GREATER, D3D11_COMPARISON_GREATER_EQUAL, D3D11_COMPARISON_LESS, D3D11_CULL_BACK, D3D11_CULL_NONE, D3D11_DEFAULT_STENCIL_READ_MASK, D3D11_DEFAULT_STENCIL_WRITE_MASK, D3D11_DEPTH_STENCILOP_DESC, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_WRITE_MASK_ALL, D3D11_FILL_SOLID, D3D11_FILL_WIREFRAME, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_INSTANCE_DATA, D3D11_INPUT_PER_VERTEX_DATA, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_VIEW_DESC, D3D11_RTV_DIMENSION_UNKNOWN, D3D11_STENCIL_OP_DECR, D3D11_STENCIL_OP_INCR, D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT, D3D11_VIEWPORT}, Dxgi::{Common::{DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_UNKNOWN}, IDXGISwapChain}, Hlsl::D3D_COMPILE_STANDARD_FILE_INCLUDE}, System::Diagnostics::Debug::OutputDebugStringA}, windows_strings::*
};
#[derive(Copy,Clone)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub colour: Vec3,
    pub normal: Vec3,
    pub texture: Vec2,
}

#[derive(Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
}

#[derive(Clone)]
pub struct VertexBuffer {
    pub buffer: ID3D11Buffer,
    pub stride: u32,
    pub offset: u32,
    pub count: u32,
}

impl VertexBuffer {

    pub fn set(bufs: &[Self], slot: u32, device_context: &ID3D11DeviceContext) {
        let buf_len = bufs.len() as u32;
        let strides: Vec<_> = bufs.iter().map(|b| b.stride).collect();
        let strides = strides.as_slice();
        let offsets: Vec<_> = bufs.iter().map(|b| b.offset).collect();
        let offsets = offsets.as_slice();
        let buffers: Vec<_> = bufs.iter().map(|b| Some(b.buffer.to_owned())).collect();
        let buffers = buffers.as_slice();

        unsafe {
            device_context.IASetVertexBuffers(
                slot,
                buf_len,
                Some(buffers.as_ptr()),
                Some(strides.as_ptr()),
                Some(offsets.as_ptr()),
            );
        }
    }

    pub fn draw(bufs: &[Self], start: u32, device_context: &ID3D11DeviceContext) {
        let total = bufs.iter().map(|b| b.count).sum();
        unsafe { device_context.Draw(total, start) }
    }

    pub fn set_and_draw(bufs: &[Self], device_context: &ID3D11DeviceContext) {
        Self::set(bufs, 0, device_context);
        Self::draw(bufs, 0, device_context);
    }
}

impl Model {
    pub fn load(obj_file: &Path) -> Vec<Self> {
        let (models, _materials) = tobj
            ::load_obj(obj_file, &tobj::LoadOptions {
                merge_identical_points: false,
                reorder_data: false,
                single_index: true,
                triangulate: true,
                ignore_points: true,
                ignore_lines: true,
            })
            .expect("Failed to load OBJ file");

        log::info!("File {:?} contains {} models", obj_file, models.len());
        let mut kat_models = Vec::new();
        for (i, m) in models.iter().enumerate() {
            let mesh = &m.mesh;
            log::info!("model[{}].name             = \'{}\'", i, m.name);
            log::info!("model[{}].mesh.material_id = {:?}", i, mesh.material_id);

            log::info!(
                "model[{}].face_count       = {}",
                i,
                mesh.face_arities.len()
            );

            log::info!(
                "model[{}].positions        = {}",
                i,
                mesh.positions.len() / 3
            );
            assert!(mesh.positions.len() % 3 == 0);

            log::info!(
                "model[{}].normals        = {}",
                i,
                mesh.normals.len() / 3
            );

            let mut vertices = Vec::new();
            for index in mesh.indices.iter() {
                let start = *index as usize*3;
                let end = *index as usize*3+3;
                let start_2d = *index as usize*2;
                let end_2d = *index as usize*2+2;
                let vertex = &mesh.positions.get(start..end).map(Vec3::from_slice).unwrap_or_default();
                let colour = &mesh.vertex_color.get(start..end).map(Vec3::from_slice).unwrap_or(Vec3::new(1.0, 1.0, 1.0));
                let normal = &mesh.normals.get(start..end).map(Vec3::from_slice).unwrap_or_default();
                let texture = &mesh.texcoords.get(start_2d..end_2d).map(Vec2::from_slice).unwrap_or_default();
                vertices.push(Vertex {
                    position: *vertex,
                    colour: *colour,
                    normal: *normal,
                    texture: *texture,
                })

            }

            kat_models.push(Self {
                vertices,
            });
        }
        kat_models
    }

    pub fn load_to_buffers(d3d11_device: ID3D11Device, obj_file: &Path) -> anyhow::Result<Vec<VertexBuffer>>  {
        let models = Self::load(obj_file);

        let mut vertex_buffers = Vec::new();
        for model in models {
            let vertex_data_array: &[Vertex] = model.vertices.as_slice();

            let stride: u32 = size_of::<Vertex>() as u32;
            let offset: u32 = 0;
            let count: u32 = vertex_data_array.len() as u32;

            log::info!("Setting up vertex buffer");
            let mut vertex_buffer_ptr: Option<ID3D11Buffer> = None;
            let subresource_data = D3D11_SUBRESOURCE_DATA {
                pSysMem: vertex_data_array.as_ptr() as *const _,
                SysMemPitch: 0,
                SysMemSlicePitch: 0,
            };
            let vertex_buffer_desc = D3D11_BUFFER_DESC {
                ByteWidth: size_of_val(vertex_data_array) as u32,
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
                StructureByteStride: 0,
            };
            let buffer = unsafe { d3d11_device.CreateBuffer(&vertex_buffer_desc, Some(&subresource_data), Some(&mut vertex_buffer_ptr)) }
                .map_err(anyhow::Error::from)
                .and_then(|()| vertex_buffer_ptr.ok_or_else(|| anyhow!("no vertex buffer")))?;

            let vertex_buffer = VertexBuffer {
                buffer,
                stride,
                offset,
                count,
            };
            vertex_buffers.push(vertex_buffer);
        }


        Ok(vertex_buffers)
    }
}


