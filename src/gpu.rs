use crate::{Brush, Curve, Rect};

const PRIMITIVE_LINE: u32 = 0x1;
const PRIMITIVE_QUADRATIC: u32 = 0x2;
const PRIMITIVE_CIRCLE: u32 = 0x3;
const PRIMITIVE_ARC: u32 = 0x4;
const PRIMITIVE_RECT: u32 = 0x5;
const PRIMITIVE_SHADOW_RECT: u32 = 0x6;

const PRIMITIVE_FILL_COLOR: u32 = 0x10;
const PRIMITIVE_FILL_LINEAR_GRADIENT: u32 = 0x11;

fn pack_f32(a: f32) -> u32 {
    unsafe { std::mem::transmute(a) }
}

fn pack_f16x2(a: f32, b: f32) -> u32 {
    let a = half::f16::from_f32(a).to_bits();
    let b = half::f16::from_f32(b).to_bits();

    a as u32 | ((b as u32) << 16)
}

fn pack_unorm8x4(x: u8, y: u8, z: u8, w: u8) -> u32 {
    x as u32 | (y as u32) << 8 | (z as u32) << 16 | (w as u32) << 24
}

#[derive(Clone)]
pub struct GpuData {
    pub vertices: Vec<u32>,
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

    pub fn extend(&mut self, path: &[Curve], rect: Rect, brush: &Brush) {
        let primitive_start = self.primitives.len() as u32;
        let vertex_start = self.vertices.len() as u32;

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
                    self.vertices
                        .extend(&[pack_f16x2(p0.x, p0.y), pack_f16x2(p1.x, p1.y)]);
                    self.primitives.push(PRIMITIVE_LINE);
                }
                Curve::Quad { p0, p1, p2 } => {
                    self.vertices.extend(&[
                        pack_f16x2(p0.x, p0.y),
                        pack_f16x2(p1.x, p1.y),
                        pack_f16x2(p2.x, p2.y),
                    ]);
                    self.primitives.push(PRIMITIVE_QUADRATIC);
                }
                Curve::Circle { center, radius } => {
                    self.vertices
                        .extend(&[pack_f16x2(center.x, center.y), pack_f32(*radius)]);
                    self.primitives.push(PRIMITIVE_CIRCLE);
                }
                Curve::Arc { center, p0, p1 } => {
                    self.vertices.extend(&[
                        pack_f16x2(center.x, center.y),
                        pack_f16x2(p0.x - center.x, p0.y - center.y),
                        pack_f16x2(p1.x - center.x, p1.y - center.y),
                    ]);
                    self.primitives.push(PRIMITIVE_ARC);
                }
                Curve::Rect { p0, p1 } => {
                    self.vertices
                        .extend(&[pack_f16x2(p0.x, p0.y), pack_f16x2(p1.x, p1.y)]);
                    self.primitives.push(PRIMITIVE_RECT);
                }
            }
        }

        match *brush {
            Brush::Color(ref c) => {
                self.primitives.push(PRIMITIVE_FILL_COLOR);
                self.vertices.push(pack_unorm8x4(c[0], c[1], c[2], c[3]));
            }
            Brush::LinearGradient {
                ref stop0,
                ref stop1,
            } => {
                self.primitives.push(PRIMITIVE_FILL_LINEAR_GRADIENT);

                self.vertices
                    .push(pack_f16x2(stop0.position.x, stop0.position.y));
                self.vertices.push(pack_unorm8x4(
                    stop0.color[0],
                    stop0.color[1],
                    stop0.color[2],
                    stop0.color[3],
                ));

                self.vertices
                    .push(pack_f16x2(stop1.position.x, stop1.position.y));
                self.vertices.push(pack_unorm8x4(
                    stop1.color[0],
                    stop1.color[1],
                    stop1.color[2],
                    stop1.color[3],
                ));
            }
        }

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
