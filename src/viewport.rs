pub struct Viewport {
    pub position: (f32, f32),
    pub scaling_y: f32,
    pub aspect_ratio: f32, // width / height
}

impl Viewport {
    pub fn get_rect(&self) -> [f32; 4] {
        let height = self.scaling_y;
        let width = self.aspect_ratio * height;

        let (cx, cy) = self.position;
        let x = cx - width / 2.0;
        let y = cy - height / 2.0;

        [x, y, width, height]
    }

    pub fn get_scale(&self) -> (f32, f32) {
        (self.scaling_y * self.aspect_ratio, self.scaling_y)
    }
}
