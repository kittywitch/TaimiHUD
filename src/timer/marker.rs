use {
    crate::{render::space::engine::RotationType, timer::BlishVec3}, glam::{Mat4, Vec3}, serde::{Deserialize, Serialize}, std::path::PathBuf
};

fn default_size() -> f32 {
    return 1.0
}

fn default_opacity() -> f32 {
    return 0.8
}

fn default_duration() -> f32 {
    return 10.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlishMarker {
    #[serde(default)]
    pub position: BlishVec3,
    #[serde(default)]
    pub rotation: BlishVec3,
    #[serde(default="default_size")]
    pub size: f32,
    #[serde(default)]
    pub fade_center: bool,
    #[serde(default="default_opacity")]
    pub opacity: f32,
    pub texture: PathBuf,
    #[serde(default="default_duration")]
    pub duration: f32,
    #[serde(default)]
    pub timestamps: Vec<f32>,
}

impl BlishMarker {
    fn marker(&self, timestamp: f32) -> TimerMarker {
        let position = self.position.to_vec3();
        let rotation = self.rotation.to_vec3();
        let kind = if rotation == Vec3::ZERO {
            RotationType::Billboard
        } else {
            let rotation_rads = rotation.map(|x| x.to_radians());
            RotationType::Rotation(rotation_rads)
        };
        TimerMarker {
            position,
            size: self.size,
            duration: self.duration,
            opacity: self.opacity,
            texture: self.texture.clone(),
            timestamp,
            kind,
        }
    }

    pub fn get_markers(&self) -> Vec<TimerMarker> {
        self.timestamps
            .iter()
            .map(|&ts| self.marker(ts))
            .collect()
    }
}

#[derive(Clone)]
pub struct TimerMarker {
    pub kind: RotationType,
    pub position: Vec3,
    pub size: f32,
    pub opacity: f32,
    pub texture: PathBuf,
    pub timestamp: f32,
    pub duration: f32,
}

impl TimerMarker {
    pub fn model_matrix(&self) -> Mat4 {
        // scale first
        let mtx_scale = Mat4::from_scale(Vec3::new(self.size, self.size, self.size));
        // then rotate the points
        let mtx_rotation = match self.kind {
            // billboards should have their rotation component handled elsewhere ideally
            // perhaps *prior* to the application of this, thus NOOP :p
            RotationType::Billboard => Mat4::IDENTITY,
            RotationType::Rotation(rot) => 
                    Mat4::from_rotation_x(rot.x) *
                    Mat4::from_rotation_y(rot.y) *
                    Mat4::from_rotation_z(rot.z),
        };
        // then move them
        let mtx_position = Mat4::from_translation(self.position);
        mtx_scale * mtx_rotation * mtx_position
    }
}
