use {
    super::{
        entity::Entity, entitydescription::EntityDescription, model::Model,
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
        rc::Rc,
    },
    windows::Win32::Graphics::Direct3D11::ID3D11Device,
};

type Eda = Rc<EntityDescription>;

#[derive(Default)]
pub struct EntityController(HashMap<PathBuf, Vec<Eda>>);

impl EntityController {
    pub fn load_desc(addon_dir: &Path) -> anyhow::Result<Self> {
        let mut entity_controller = Self::default();
        let model_folder = addon_dir.join("models");

        if model_folder.exists() {
            let entity_desc_paths: Paths = glob::glob(
                model_folder
                    .join("*.entitydesc")
                    .to_str()
                    .expect("Model load pattern is unparseable"),
            )?;
            for entity_desc_path in entity_desc_paths {
                let entity_desc_path = entity_desc_path?;
                log::info!("Loading entities from {:?}", entity_desc_path);
                let entity_descs = EntityDescription::load(&entity_desc_path)?;
                for entity_desc in entity_descs.into_iter() {
                    let full_path = model_folder.join(&entity_desc.location.file);
                    let entry = entity_controller.0.entry(full_path).or_default();
                    let entity_desc_arc = Rc::new(entity_desc);
                    entry.push(entity_desc_arc);
                }
            }
        }
        Ok(entity_controller)
    }

    pub fn load(self, device: &ID3D11Device) -> anyhow::Result<Vec<Rc<Entity>>> {
        let mut entities = Vec::new();
        for (file, descs) in &self.0 {
            let file_models = Model::load(file)?;
            for desc in descs {
                let model_idx = desc.location.index;
                let model = file_models.get(model_idx).ok_or_else(|| {
                    anyhow!(
                        "model index {} does not exist in file {:?}",
                        model_idx,
                        file
                    )
                })?;
                log::info!(
                    "Loading entity \"{}\" from \"{:?}\"@{}",
                    desc.name,
                    desc.location.file,
                    desc.location.index
                );
                let vertex_buffer = model.to_buffer(device)?;
                let vertex_buffer_rc = Rc::new(vertex_buffer);
                let mut rng = rand::rng();
                let model_matrix: Vec<_> = (0..1000 * 3)
                    .map(|_| rng.random::<f32>() * 1000.0)
                    .chunks(3)
                    .into_iter()
                    .map(|xyz| Vec3::from_slice(&xyz.into_iter().collect::<Vec<_>>()))
                    .map(Mat4::from_translation)
                    .map(|trans| InstanceBufferData {
                        model: desc.model_matrix * trans,
                    })
                    .collect();

                let instance_buffer = Entity::setup_instance_buffer(&model_matrix, device)?;
                let entity = Entity {
                    name: desc.name.clone(),
                    model: model.clone(),
                    model_matrix: RefCell::new(model_matrix),
                    location: desc.location.clone(),
                    pixel_shader: desc.pixel_shader.clone(),
                    vertex_shader: desc.vertex_shader.clone(),
                    vertex_buffer: vertex_buffer_rc.clone(),
                    instance_buffer,
                };
                let entity = Rc::new(entity);
                entities.push(entity);
            }
        }
        log::info!("Entities successfully loaded!");
        Ok(entities)
    }
}
