use {
    arc_atomic::AtomicArc,
    glam::Vec3,
    itertools::Itertools,
    std::sync::{Arc, OnceLock},
};

static PERSPECTIVEINPUTDATA: OnceLock<Arc<AtomicArc<PerspectiveInputData>>> = OnceLock::new();

#[derive(Debug, Default, PartialEq, Clone)]
pub struct PerspectiveInputData {
    pub front: Vec3,
    pub pos: Vec3,
    pub fov: f32,
}

impl PerspectiveInputData {
    pub fn create() {
        let aarc = Arc::new(AtomicArc::new(Arc::new(Self::default())));
        let _ = PERSPECTIVEINPUTDATA.set(aarc);
    }

    pub fn read() -> Option<Arc<Self>> {
        Some(PERSPECTIVEINPUTDATA.get()?.load())
    }

    pub fn swap_camera(front: Vec3, pos: Vec3) {
        if let Some(data) = PERSPECTIVEINPUTDATA.get() {
            let pdata = data.load();
            data.store(Arc::new(PerspectiveInputData {
                fov: pdata.fov,
                front,
                pos,
            }))
        }
    }

    pub fn swap_fov(fov: f32) {
        if let Some(data) = PERSPECTIVEINPUTDATA.get() {
            let pdata = data.load();
            data.store(Arc::new(PerspectiveInputData {
                fov,
                front: pdata.front,
                pos: pdata.pos,
            }))
        }
    }
}
