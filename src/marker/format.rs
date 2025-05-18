use {
    crate::timer::{BlishVec3, Polytope, Position},
    anyhow::anyhow,
    chrono::{DateTime, Utc},
    glam::Vec3,
    glob::Paths,
    nexus::gamebind::GameBind,
    serde::{Deserialize, Serialize},
    serde_repr::{Deserialize_repr, Serialize_repr},
    std::{
        collections::HashMap,
        fs::exists,
        path::{Path, PathBuf},
        sync::Arc,
    },
    strum_macros::{Display, FromRepr},
    tokio::{
        fs::read_to_string,
        sync::Semaphore,
        task::JoinSet,
    },
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum MarkerFormats {
    File(MarkerFile),
    Custom(CustomMarkers),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuntimeMarkers {
    pub path: Option<PathBuf>,
    pub file: MarkerFormats,
}
impl RuntimeMarkers {
    pub fn glob() -> String {
        "**/*.markers".to_string()
    }

    pub fn path_glob(path: &Path) -> PathBuf {
        path.join(&Self::glob())
    }

    pub fn get_paths(path: &Path) -> anyhow::Result<Paths> {
        let pathbuf_glob = Self::path_glob(path);

        let path_glob_str = pathbuf_glob
            .to_str()
            .ok_or_else(|| anyhow!("Timer file loading path glob unparseable for {path:?}"))?;
        Ok(glob::glob(path_glob_str)?)
    }
    pub async fn load_many(
        load_dir: &Path,
        simultaneous_limit: usize,
    ) -> anyhow::Result<Vec<Arc<Self>>> {
        log::debug!("Beginning load_many for {load_dir:?} with a simultaneous open limit of {simultaneous_limit}.");
        let mut marker_files = Vec::new();
        if exists(load_dir)? {
            let mut set = JoinSet::new();
            let semaphore = Arc::new(Semaphore::new(simultaneous_limit));
            let mut paths = Self::get_paths(load_dir)?;
            while let Some(path) = paths.next() {
                let permit = semaphore.clone().acquire_owned().await?;
                let path = path?.clone();
                set.spawn(async move {
                    let marker_file = Self::load(&path).await?;
                    drop(permit);
                    Ok::<Arc<Self>, anyhow::Error>(marker_file)
                });
            }
            let (mut join_errors, mut load_errors): (usize, usize) = (0, 0);
            while let Some(marker_file) = set.join_next().await {
                match marker_file {
                    Ok(res) => match res {
                        Ok(marker_file) => {
                            marker_files.push(marker_file);
                        }
                        Err(err) => {
                            load_errors += 1;
                            log::error!("marker load_many error for {load_dir:?}: {err}");
                        }
                    },
                    Err(err) => {
                        join_errors += 1;
                        log::error!("marker load_many join error for {load_dir:?}: {err}");
                    }
                }
            }
            log::debug!(
                "Finished load_many for {load_dir:?}: {} succeeded, {join_errors} join errors, {load_errors} other errors.",
                marker_files.len()
            );
        }
        Ok(marker_files)
    }
    pub async fn load(path: &PathBuf) -> anyhow::Result<Arc<Self>> {
        log::debug!("Attempting to load the markers file at \"{path:?}\".");
        let mut file_data = read_to_string(path).await?;
        json_strip_comments::strip(&mut file_data)?;
        let format: MarkerFormats = serde_json::from_str(&file_data)?;
        let data = Self {
            file: format,
            path: Some(path.to_path_buf()),
        };
        log::debug!("Successfully loaded the markers file at \"{path:?}\".");
        Ok(Arc::new(data))
    }
    pub async fn markers(marker_packs: Vec<Arc<Self>>) -> HashMap<String, Vec<Arc<MarkerSet>>> {
        let mut finalized: HashMap<String, Vec<Arc<MarkerSet>>> = HashMap::new();
        for pack in marker_packs {
            log::info!("{:?}", pack);
            match &pack.file {
                MarkerFormats::File(f) => {
                    for category in &f.categories {
                        let category_name = category.name.clone();
                        let entry = finalized.entry(category_name).or_default();
                        for marker_set in &category.marker_sets {
                            let mut marker_set_data = marker_set.clone();
                            marker_set_data.path = pack.path.clone();
                            let marker_set_arc = Arc::new(marker_set_data);
                            entry.push(marker_set_arc);
                        }
                    }
                }
                MarkerFormats::Custom(c) => {
                    let category_name = "Custom".to_string();
                    let entry = finalized.entry(category_name).or_default();
                    for marker_set in &c.squad_marker_preset {
                        let mut marker_set_data = marker_set.clone();
                        marker_set_data.path = pack.path.clone();
                        let marker_set_arc = Arc::new(marker_set_data);
                        entry.push(marker_set_arc);
                    }
                }
            }
        }
        finalized
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkerFile {
    pub last_edit: DateTime<Utc>,
    pub path: Option<PathBuf>,
    pub categories: Vec<MarkerCategory>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CustomMarkers {
    pub version: String,
    pub path: Option<PathBuf>,
    pub squad_marker_preset: Vec<MarkerSet>,
}

impl CustomMarkers {
    pub async fn load(path: &PathBuf) -> anyhow::Result<Arc<Self>> {
        log::debug!("Attempting to load the markers file at \"{path:?}\".");
        let mut file_data = read_to_string(path).await?;
        json_strip_comments::strip(&mut file_data)?;
        let mut data: Self = serde_json::from_str(&file_data)?;
        data.path = Some(path.to_path_buf());
        log::debug!("Successfully loaded the markers file at \"{path:?}\".");
        Ok(Arc::new(data))
    }
}

impl MarkerFile {
    pub async fn load(path: &PathBuf) -> anyhow::Result<Arc<Self>> {
        log::debug!("Attempting to load the markers file at \"{path:?}\".");
        let mut file_data = read_to_string(path).await?;
        json_strip_comments::strip(&mut file_data)?;
        let mut data: Self = serde_json::from_str(&file_data)?;
        data.path = Some(path.to_path_buf());
        log::debug!("Successfully loaded the markers file at \"{path:?}\".");
        Ok(Arc::new(data))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkerCategory {
    #[serde(alias = "categoryName")]
    pub name: String,
    pub marker_sets: Vec<MarkerSet>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MarkerSet {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub author: Option<String>,
    pub name: String,
    pub description: String,
    pub map_id: u32,
    pub trigger: MarkerPosition,
    pub markers: Vec<MarkerEntry>,
    #[serde(default)]
    pub path: Option<PathBuf>,
}

impl MarkerSet {
    pub fn combined(&self) -> String {
        if let Some(author) = &self.author {
            format!("{}\nAuthor: {}", self.name.clone(), author.clone())
        } else {
            format!("{}\nUnknown Author", self.name.clone())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarkerEntry {
    #[serde(alias = "i")]
    pub marker: MarkerType,
    #[serde(alias = "d")]
    pub id: Option<String>,
    #[serde(flatten)]
    pub position: MarkerPosition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarkerPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<MarkerPosition> for Vec3 {
    // it's pre-swizzled
    fn from(local: MarkerPosition) -> Self {
        Self::new(local.x, local.z, local.y)
    }
}

impl From<Vec3> for MarkerPosition {
    // it's pre-swizzled
    fn from(local: Vec3) -> Self {
        Self {
            x: local.x,
            y: local.z,
            z: local.y,
        }
    }
}

impl From<MarkerPosition> for BlishVec3 {
    fn from(local: MarkerPosition) -> Self {
        Self::from_vec3(local.into())
    }
}

impl From<MarkerPosition> for Position {
    fn from(local: MarkerPosition) -> Self {
        let local_vec3: Vec3 = local.into();
        Self::from(local_vec3)
    }
}

impl From<MarkerPosition> for Polytope {
    fn from(local: MarkerPosition) -> Self {
        Polytope::NSphere {
            center: local.into(),
            radius: 15.0,
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, FromRepr, Display, Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum MarkerType {
    // Schema reference: https://github.com/manlaan/BlishHud-CommanderMarkers/blob/bhud-static/Manlaan.CommanderMarkers/README.md?plain=1#L69-L78
    // According to the schema, 0 and 9 are both Clear Markers.
    // Code reference: https://github.com/manlaan/BlishHud-CommanderMarkers/blob/7c3b2081596f7b8746e5e57d65213711aafa938c/Library/Enums/SquadMarker.cs#L6-L28
    // According to their code, 0 is None and 9 is Clear Markers.
    // This is why I have trust issues, man.
    Blank = 0,
    Arrow = 1,
    Circle = 2,
    Heart = 3,
    Square = 4,
    Star = 5,
    Spiral = 6,
    Triangle = 7,
    Cross = 8,
    ClearMarkers = 9,
}

impl MarkerType {
    pub fn iter_real_values() -> impl Iterator<Item = Self> {
        (1..9).flat_map(|i| Self::from_repr(i))
    }

    pub fn to_place_world_gamebind(&self) -> GameBind {
        match self {
            Self::Blank => panic!("i can't believe you've done this"),
            Self::Arrow => GameBind::SquadMarkerPlaceWorldArrow,
            Self::Circle => GameBind::SquadMarkerPlaceWorldCircle,
            Self::Heart => GameBind::SquadMarkerPlaceWorldHeart,
            Self::Square => GameBind::SquadMarkerPlaceWorldSquare,
            Self::Star => GameBind::SquadMarkerPlaceWorldStar,
            // what in the fuck my dudes
            Self::Spiral => GameBind::SquadMarkerPlaceWorldSwirl,
            Self::Triangle => GameBind::SquadMarkerPlaceWorldTriangle,
            Self::Cross => GameBind::SquadMarkerPlaceWorldCross,
            Self::ClearMarkers => GameBind::SquadMarkerClearAllWorld,
        }
    }
    pub fn to_set_agent_gamebind(&self) -> GameBind {
        match self {
            Self::Blank => panic!("i can't believe you've done this"),
            Self::Arrow => GameBind::SquadMarkerSetAgentArrow,
            Self::Circle => GameBind::SquadMarkerSetAgentCircle,
            Self::Heart => GameBind::SquadMarkerSetAgentHeart,
            Self::Square => GameBind::SquadMarkerSetAgentSquare,
            Self::Star => GameBind::SquadMarkerSetAgentStar,
            // what in the fuck my dudes
            Self::Spiral => GameBind::SquadMarkerSetAgentSwirl,
            Self::Triangle => GameBind::SquadMarkerSetAgentTriangle,
            Self::Cross => GameBind::SquadMarkerSetAgentCross,
            Self::ClearMarkers => GameBind::SquadMarkerClearAllWorld,
        }
    }
}
