use nalgebra_glm as glm;

mod gpu;
mod path;
mod text;
mod viewport;

pub use crate::gpu::*;
pub use crate::path::*;
pub use crate::text::*;
pub use crate::viewport::*;

pub type Offset = glm::Vec2;
pub type Extent = glm::Vec2;

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub offset_local: Offset,
    pub extent_local: Extent,
    pub offset_curve: Offset,
    pub extent_curve: Extent,
}
