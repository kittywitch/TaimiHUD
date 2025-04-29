use {
    super::{
        entitydescription::EntityDescription, model::Model, shader::Shaders,
        state::InstanceBufferData,
    },
    anyhow::anyhow,
    glam::{Mat4, Vec3},
    glob::Paths,
    itertools::Itertools,
    rand::Rng,
    std::{
        cell::RefCell,
        collections::HashMap,
        path::{Path, PathBuf},
    },
    windows::Win32::Graphics::Direct3D11::ID3D11Device,
};

#[derive(Default,Debug)]
pub struct ObjectLoader(pub HashMap<PathBuf, Vec<EntityDescription>>);

impl ObjectLoader {
    pub fn load_desc(model_dir: &Path) -> anyhow::Result<Self> {
        let mut entity_controller = Self::default();

        if model_dir.exists() {
            let entity_desc_paths: Paths = glob::glob(
                model_dir
                    .join("*.entitydesc")
                    .to_str()
                    .expect("Model load pattern is unparseable"),
            )?;
            for entity_desc_path in entity_desc_paths {
                let entity_desc_path = entity_desc_path?;
                log::info!("Loading entities from {:?}", entity_desc_path);
                let entity_descs = EntityDescription::load(&entity_desc_path)?;
                for entity_desc in entity_descs.into_iter() {
                    log::debug!("eouh {}", entity_desc.name);
                    let full_path = model_dir.join(&entity_desc.location.file);
                    let entry = entity_controller.0.entry(full_path).or_default();
                    entry.push(entity_desc);
                }
            }
        }
        Ok(entity_controller)
    }

}
