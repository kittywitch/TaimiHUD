use {
    super::{super::dx11::InstanceBufferData, ObjectRenderBacking, ObjectRenderMetadata}, crate::{render::{space::{dx11::InstanceBuffer, resources::{obj_format::material::ColouredMaterialTexture, Model, ObjMaterial, ShaderPair, Texture}}, Engine}, timer::TimerMarker}, glam::{Mat4, Vec3}, std::sync::RwLock, windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11DeviceContext}
};

pub struct ObjectBacking {
    pub name: String,
    pub render: ObjectRenderBacking,
}

impl ObjectBacking {
    pub fn create_marker(
        &self,
        engine: &Engine,
        marker: &TimerMarker,
    ) -> anyhow::Result<Self> {
        let texture = Texture::load(&engine.render_backend.device, &marker.texture)?;
        let shaders = ShaderPair(
            engine.render_backend.shaders.0["textured"].clone(),
            engine.render_backend.shaders.1["textured"].clone(),
        );
        let model = Model::quad()?;
        let model_matrix = marker.model_matrix();
        let ibd = [InstanceBufferData {
            world: model_matrix,

            colour: Vec3::ONE,
        }];
        let render = ObjectRenderBacking {
            instance_buffer: RwLock::new(InstanceBuffer::create(
                &engine.render_backend.device, &ibd)?),
            vertex_buffer: model.to_buffer(&engine.render_backend.device)?,
            shaders,
            metadata: ObjectRenderMetadata {
                model,
                material: ObjMaterial {
                    ambient: None,
                    specular: None,
                    shininess: None,
                    dissolve: None,
                    normal: None,
                    diffuse: Some(ColouredMaterialTexture {
                        texture,
                        colour: Vec3::ONE,
                    })
                },
                model_matrix,
                topology: Default::default(),
            }
        };
        Ok(Self {
            name: "Marker".to_string(),
            render,
        })

    }

    pub fn set_and_draw(
        &self,
        slot: u32,
        device: &ID3D11Device,
        device_context: &ID3D11DeviceContext,
        data: &[InstanceBufferData],
    ) -> anyhow::Result<()> {
        self.render.set_and_draw(slot, device, device_context, data)
    }
}
