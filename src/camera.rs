pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Camera {
    pub fn new(x: f32, y: f32, zoom: f32) -> Self {
        Camera { x, y, zoom, min_zoom: 1.0, max_zoom: 16.0 }
    }

    pub fn follow(&mut self, target_x: f32, target_y: f32) {
        self.x = target_x;
        self.y = target_y;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
    }

    pub fn zoom_scroll(&mut self, scroll_dy: f32) {
        // Positive dy typically means scroll up on miniquad (zoom in)
        let new_zoom = self.zoom + scroll_dy * 0.1;
        self.set_zoom(new_zoom);
    }
}


