use {
    super::{
        ecs::{
            InstanceBuffer, ObjectBacking, ObjectRenderBacking, ObjectRenderMetadata, ShaderPair,
        },
        model::{ModelLocationDescription, ObjModelData, ObjModelFile},
        primitivetopology::PrimitiveTopology,
        shader::{PixelShader, PixelShaders, VertexShader, VertexShaders},
    },
    anyhow::anyhow,
    glam::Mat4,
    itertools::Itertools,
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        fs::read_to_string,
        path::PathBuf,
        sync::{Arc, RwLock},
    },
    windows::Win32::Graphics::Direct3D11::ID3D11Device,
};

fn default_pixel_shader() -> String {
    "generic".to_string()
}
fn default_vertex_shader() -> String {
    "generic".to_string()
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EntityDescription {
    pub name: String,
    pub location: ModelLocationDescription,
    #[serde(default = "default_vertex_shader")]
    pub vertex_shader: String,
    #[serde(default = "default_pixel_shader")]
    pub pixel_shader: String,
    #[serde(default)]
    pub model_matrix: Mat4,
    #[serde(default)]
    pub xzy: bool,
    #[serde(default)]
    pub topology: PrimitiveTopology,
}

impl EntityDescription {
    pub fn load(path: &PathBuf) -> anyhow::Result<Vec<Self>> {
        log::debug!("Attempting to load the entity description file at \"{path:?}\".");
        let mut file_data = read_to_string(path)?;
        json_strip_comments::strip(&mut file_data)?;
        let entity_description_data: Vec<Self> = serde_json::from_str(&file_data)?;
        Ok(entity_description_data)
    }

    pub fn get_shaders(
        &self,
        vertex_shaders: &VertexShaders,
        pixel_shaders: &PixelShaders,
    ) -> ShaderPair {
        let vertex_shader: Arc<VertexShader> =
            vertex_shaders.get(&self.vertex_shader).unwrap().clone();
        let pixel_shader: Arc<PixelShader> = pixel_shaders.get(&self.pixel_shader).unwrap().clone();
        ShaderPair(vertex_shader, pixel_shader)
    }

    pub fn get_model_and_material(
        &self,
        device: &ID3D11Device,
        model_files: &HashMap<PathBuf, ObjModelFile>,
    ) -> anyhow::Result<ObjModelData> {
        let model_file = model_files
            .get(&self.location.file)
            .ok_or_else(|| anyhow!("Could not load model file!"))?;
        let obj_model_data = model_file.load_datum(device, self.location.index, self.xzy);
        Ok(obj_model_data)
    }

    pub fn to_backing(
        &self,
        model_files: &HashMap<PathBuf, ObjModelFile>,
        device: &ID3D11Device,
        vertex_shaders: &VertexShaders,
        pixel_shaders: &PixelShaders, // Shaders
    ) -> anyhow::Result<ObjectBacking> {
        log::info!("A conversion to ObjectBacking!");
        let shaders = self.get_shaders(vertex_shaders, pixel_shaders);
        let obj_model_data = self.get_model_and_material(device, model_files)?;
        let model = obj_model_data.model;
        let material = obj_model_data.material;
        let vertex_buffer = model.to_buffer(device)?;
        let render_metadata = ObjectRenderMetadata {
            model,
            material,
            model_matrix: self.model_matrix,
            topology: self.topology,
        };

        let render_backing = ObjectRenderBacking {
            metadata: render_metadata,
            instance_buffer: RwLock::new(InstanceBuffer::create_empty(device)?),
            vertex_buffer,
            shaders,
        };

        let backing = ObjectBacking {
            name: self.name.clone(),
            render: render_backing,
        };

        Ok(backing)
    }
}
