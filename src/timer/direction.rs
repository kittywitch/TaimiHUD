use relative_path::RelativePathBuf;
use super::BlishVec3;

struct BlishDirection {
    name: String,
    destination: BlishVec3,
    texture: RelativePathBuf,
    animSpeed: f32,
    duration: f32,
    timestamps: Vec<f32>,

}

struct TimerDirection {
    name: String,
    destination: BlishVec3,
    texture: RelativePathBuf,
    animSpeed: f32,
    duration: f32,
    timestamp: f32,
}
