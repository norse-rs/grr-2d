use nalgebra_glm as glm;

mod app;
mod app_wgpu;
mod brush;
mod gpu;
mod path;
mod text;
mod viewport;

pub use crate::app::*;
pub use crate::app_wgpu::*;
pub use crate::brush::*;
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

impl Rect {
    pub fn local_to_curve(&self, local: glm::Vec2) -> glm::Vec2 {
        let tx = if self.extent_local.x.abs() > 0.0 {
            (local.x - self.offset_local.x) / self.extent_local.x
        } else {
            0.0
        };

        let ty = if self.extent_local.y.abs() > 0.0 {
            (local.y - self.offset_local.y) / self.extent_local.y
        } else {
            0.0
        };

        glm::vec2(
            self.offset_curve.x + tx * self.extent_curve.x,
            self.offset_curve.y + ty * self.extent_curve.y,
        )
    }

    pub fn extrude(&self, border: f32) -> Self {
        let offset_local = self.offset_local - glm::vec2(border, border);
        let extent_local = self.extent_local + 2.0 * glm::vec2(border, border);

        let offset_curve = self.local_to_curve(offset_local);
        let extent_curve = self.local_to_curve(offset_local + extent_local) - offset_curve;

        Rect {
            offset_local,
            extent_local,
            offset_curve,
            extent_curve,
        }
    }
}
