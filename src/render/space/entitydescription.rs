use {
    super::model::ModelLocation,
    glam::Mat4,
    serde::{Deserialize, Serialize},
    std::{fs::read_to_string, path::PathBuf},
};

fn default_pixel_shader() -> String {
    "generic_ps".to_string()
}
fn default_vertex_shader() -> String {
    "generic_vs".to_string()
}
#[derive(Clone, Serialize, Deserialize)]
pub struct EntityDescription {
    pub name: String,
    pub location: ModelLocation,
    #[serde(default = "default_vertex_shader")]
    pub vertex_shader: String,
    #[serde(default = "default_pixel_shader")]
    pub pixel_shader: String,
    #[serde(default)]
    pub model_matrix: Mat4,
}

impl EntityDescription {
    pub fn load(path: &PathBuf) -> anyhow::Result<Vec<Self>> {
        log::debug!("Attempting to load the entity description file at \"{path:?}\".");
        let mut file_data = read_to_string(path)?;
        json_strip_comments::strip(&mut file_data)?;
        let entity_description_data: Vec<Self> = serde_json::from_str(&file_data)?;
        Ok(entity_description_data)
    }
}
