use crate::glm;

pub type Color = [u8; 4];

pub struct GradientStop {
    pub position: glm::Vec2,
    pub color: Color,
}

pub enum Brush {
    Color(Color),
    LinearGradient {
        stop0: GradientStop,
        stop1: GradientStop,
    },
}
