use {
    crate::timer::{
        TimerPhase,
        TimerTrigger
    }, nexus::texture::{texture_receive, RawTextureReceiveCallback, load_texture_from_file}, relative_path::RelativePathBuf, serde::{Deserialize, Serialize}, std::{collections::HashMap, path::PathBuf}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TimerFile {
    #[serde(default,skip)]
    pub path: Option<PathBuf>,
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub icon: RelativePathBuf,
    // I probably don't need to do this, but it's convenient :o
    #[serde(rename = "map")]
    pub map_id: u32,
    pub reset: TimerTrigger,
    pub phases: Vec<TimerPhase>,
}

impl TimerFile {
    pub fn name(&self) -> String {
        self.name.replace("\n", " ")
    }
    pub fn hypheny_name(&self) -> String {
        self.name.replace("\n", " - ")
    }
    pub fn author(&self) -> String {
        self.author.replace("\n", "")
    }
    pub fn list_icon_paths(&self) -> HashMap<RelativePathBuf, PathBuf> {
        let mut textures = HashMap::new();
        if let Some(path) = &self.path {
            if let Some(base) = path.parent() {
                textures = self.phases.iter().flat_map(|phase| phase.list_icon_paths(&base.to_path_buf())).collect();
                textures.insert(self.icon.clone(), self.icon.to_path(base));
            }
        }
        textures
    }
    pub fn load_textures(&self) {
        let textures = self.list_icon_paths();
        let cally: RawTextureReceiveCallback = texture_receive!(|id, _texture| {
            log::info!("Texture {id} loaded.");
        });
        for (relative, absolute) in textures {
            load_texture_from_file(relative.as_str(), absolute,Some(cally));
        }
    }
}
