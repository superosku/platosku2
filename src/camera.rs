pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub screen_w: f32,
    pub screen_h: f32,
}

impl Camera {
    pub fn new(x: f32, y: f32, zoom: f32, width: f32, height: f32) -> Self {
        Camera {
            x,
            y,
            zoom,
            min_zoom: 1.0 / 16.0,
            max_zoom: 16.0,
            screen_w: width,
            screen_h: height,
        }
    }

    pub fn on_resize(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }

    pub fn slowly_follow(&mut self, x: f32, y: f32, zoom: f32) {
        let target_zoom = zoom.clamp(self.min_zoom, self.max_zoom);

        self.x += (x - self.x) * 0.10;
        self.y += (y - self.y) * 0.10;
        self.zoom += (target_zoom - self.zoom) * 0.10;
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
    }

    pub fn zoom_scroll(&mut self, scroll_dy: f32) {
        // Positive dy typically means scroll up on miniquad (zoom in)
        let new_zoom = self.zoom + scroll_dy * 0.1;
        self.set_zoom(new_zoom);
    }

    pub fn screen_to_tile(&self, mouse_x: f32, mouse_y: f32) -> (i32, i32) {
        // Keep this in sync with TILE_SIZE used in rendering.
        const TILE_SIZE: f32 = 16.0;

        // Camera center in world pixels (rendering uses pixel-snapped camera center)
        let cx_px = self.x * TILE_SIZE;
        let cy_px = self.y * TILE_SIZE;
        let snapped_cx = (cx_px * self.zoom).round() / self.zoom;
        let snapped_cy = (cy_px * self.zoom).round() / self.zoom;

        // Invert the View transform used in rendering:
        // screen = (world - snapped_center) * zoom + screen_center
        // => world = (screen - screen_center) / zoom + snapped_center
        let world_x_px = (mouse_x - self.screen_w * 0.5) / self.zoom + snapped_cx;
        let world_y_px = (mouse_y - self.screen_h * 0.5) / self.zoom + snapped_cy;

        // Convert world pixels to tile indices on the base grid
        let tx = (world_x_px / TILE_SIZE).floor() as i32;
        let ty = (world_y_px / TILE_SIZE).floor() as i32;
        (tx, ty)
    }

    pub fn zoom_to_fit_horizontal_tiles(&self, tiles: u32) -> f32 {
        // Keep this in sync with TILE_SIZE used in rendering.
        const TILE_SIZE: f32 = 16.0;
        if tiles == 0 {
            return self.max_zoom;
        }
        let desired = self.screen_w / (tiles as f32 * TILE_SIZE);
        desired.clamp(self.min_zoom, self.max_zoom)
    }

    pub fn zoom_to_fit_vertical_tiles(&self, tiles: u32) -> f32 {
        // Keep this in sync with TILE_SIZE used in rendering.
        const TILE_SIZE: f32 = 16.0;
        if tiles == 0 {
            return self.max_zoom;
        }
        let desired = self.screen_h / (tiles as f32 * TILE_SIZE);
        desired.clamp(self.min_zoom, self.max_zoom)
    }
}
