pub mod backing;
pub mod description;
pub mod loader;
pub mod primitivetopology;
pub mod render;

pub use {
    backing::ObjectBacking,
    description::ObjectDescription,
    loader::ObjectLoader,
    primitivetopology::PrimitiveTopology,
    render::{ObjectRenderBacking, ObjectRenderMetadata},
};
