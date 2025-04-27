use {
    super::{
        entity::Entity, entitydescription::EntityDescription, model::Model, shader::Shaders,
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

#[derive(Default)]
pub struct EntityController(HashMap<PathBuf, Vec<EntityDescription>>);

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
                    entry.push(entity_desc);
                }
            }
        }
        Ok(entity_controller)
    }

    pub fn load(self, device: &ID3D11Device, shaders: &Shaders) -> anyhow::Result<Vec<Entity>> {
        let mut entities = Vec::new();
        for (file, descs) in &self.0 {
            let mut file_models = Model::load(device, file)?;
            for desc in descs {
                let model_idx = desc.location.index;
                log::info!(
                    "Loading entity \"{}\" from \"{:?}\"@{}",
                    desc.name,
                    desc.location.file,
                    model_idx
                );
                let mut model = std::mem::take(&mut file_models[model_idx]);
                if desc.xzy {
                    model.swizzle();
                }
                let vertex_buffer = model.to_buffer(device)?;
                let mut rng = rand::rng();
                let mut rng2 = rand::rng();
                let model_matrix: Vec<_> = (0..3)
                    .map(|_| rng.random::<f32>() * 10.0)
                    .chunks(3)
                    .into_iter()
                    .map(|xyz| Vec3::from_slice(&xyz.into_iter().collect::<Vec<_>>()))
                    .map(Mat4::from_translation)
                    .map(|trans| InstanceBufferData {
                        colour: Vec3::new(
                            rng2.random::<f32>(),
                            rng2.random::<f32>(),
                            rng2.random::<f32>(),
                        ),
                        model: desc.model_matrix
                            * Mat4::from_scale(Vec3::new(2.0, 2.0, 2.0))
                            * trans,
                    })
                    .collect();

                let instance_buffer = Entity::setup_instance_buffer(&model_matrix, device)?;

                let vertex_shader = shaders
                    .0
                    .get(&desc.vertex_shader)
                    .ok_or_else(|| {
                        anyhow!(
                            "Vertex shader {} is missing, required for entity {}!",
                            &desc.pixel_shader,
                            &desc.name
                        )
                    })?
                    .clone();
                let pixel_shader = shaders
                    .0
                    .get(&desc.pixel_shader)
                    .ok_or_else(|| {
                        anyhow!(
                            "Pixel shader {} is missing, required for entity {}!",
                            &desc.pixel_shader,
                            &desc.name
                        )
                    })?
                    .clone();
                let entity = Entity {
                    topology: desc.topology,
                    name: desc.name.clone(),
                    model_matrix: RefCell::new(model_matrix),
                    location: Some(desc.location.clone()),
                    pixel_shader,
                    vertex_shader,
                    model,
                    vertex_buffer,
                    instance_buffer,
                };
                entities.push(entity);
            }
        }
        log::info!("Entities successfully loaded!");
        Ok(entities)
    }
}
