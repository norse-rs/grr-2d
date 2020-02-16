use crate::{Curve, Rect};

const PRIMITIVE_LINE: u32 = 0x1;
const PRIMITIVE_QUADRATIC: u32 = 0x2;
const PRIMITIVE_CIRCLE: u32 = 0x3;
const PRIMITIVE_ARC: u32 = 0x4;

const PRIMITIVE_FILL: u32 = 0x5;

pub struct GpuData {
    pub vertices: Vec<f32>,
    pub primitives: Vec<u32>,
    pub bbox: Vec<f32>,
    pub curve_ranges: Vec<u32>,
}

impl GpuData {
    pub fn new() -> Self {
        GpuData {
            vertices: Vec::new(),
            primitives: Vec::new(),
            bbox: Vec::new(),
            curve_ranges: Vec::new(),
        }
    }

    pub fn extend(&mut self, path: &[Curve], rect: Rect) {
        let primitive_start = self.primitives.len() as u32;
        let vertex_start = self.vertices.len() as u32 / 2;

        let min_local = rect.offset_local;
        let max_local = rect.offset_local + rect.extent_local;

        let min_curve = rect.offset_curve;
        let max_curve = rect.offset_curve + rect.extent_curve;

        self.bbox.extend(&[
            min_local.x,
            min_local.y,
            min_curve.x,
            min_curve.y,
            min_local.x,
            max_local.y,
            min_curve.x,
            max_curve.y,
            max_local.x,
            max_local.y,
            max_curve.x,
            max_curve.y,
            max_local.x,
            min_local.y,
            max_curve.x,
            min_curve.y,
            min_local.x,
            min_local.y,
            min_curve.x,
            min_curve.y,
            max_local.x,
            max_local.y,
            max_curve.x,
            max_curve.y,
        ]);

        for curve in path {
            match curve {
                Curve::Line { p0, p1 } => {
                    self.vertices.extend(&[p0.x, p0.y, p1.x, p1.y]);
                    self.primitives.push(PRIMITIVE_LINE);
                }
                Curve::Quad { p0, p1, p2 } => {
                    self.vertices.extend(&[p0.x, p0.y, p1.x, p1.y, p2.x, p2.y]);
                    self.primitives.push(PRIMITIVE_QUADRATIC);
                }
                Curve::Circle { center, radius } => {
                    self.vertices.extend(&[center.x, center.y, *radius, *radius]);
                    self.primitives.push(PRIMITIVE_CIRCLE);
                }
                Curve::Arc { center, p0, p1 } => {
                    self.vertices.extend(&[center.x, center.y, p0.x - center.x, p0.y - center.y, p1.x - center.x, p1.y - center.y]);
                    self.primitives.push(PRIMITIVE_ARC);
                }
            }
        }
        self.primitives.push(PRIMITIVE_FILL);
        let primitive_end = self.primitives.len() as u32;

        self.curve_ranges.extend(&[
            vertex_start,
            primitive_start,
            primitive_end,
            vertex_start,
            primitive_start,
            primitive_end,
            vertex_start,
            primitive_start,
            primitive_end,
            vertex_start,
            primitive_start,
            primitive_end,
            vertex_start,
            primitive_start,
            primitive_end,
            vertex_start,
            primitive_start,
            primitive_end,
        ]);
    }
}
