use crate::render::TILE_SIZE;
use glam::{Mat4, Vec3};

pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub screen_w: f32,
    pub screen_h: f32,
}

#[derive(Debug)]
pub struct MouseCoords {
    pub x: f32,
    pub y: f32,
}

impl MouseCoords {
    pub fn as_i(&self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }
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

    /// The camera center in world pixels, snapped so that it lands on a whole
    /// screen pixel. This avoids subpixel seams between tiles at any zoom.
    fn snapped_center_px(&self) -> (f32, f32) {
        let cx = self.x * TILE_SIZE;
        let cy = self.y * TILE_SIZE;
        (
            (cx * self.zoom).round() / self.zoom,
            (cy * self.zoom).round() / self.zoom,
        )
    }

    /// Screen pixels (origin top-left, y down) to clip space.
    pub fn screen_proj(&self) -> Mat4 {
        Mat4::orthographic_rh_gl(0.0, self.screen_w, self.screen_h, 0.0, -1.0, 1.0)
    }

    /// World pixels to screen pixels:
    /// `screen = (world - snapped_center) * zoom + screen_center`.
    pub fn view(&self) -> Mat4 {
        let (cx, cy) = self.snapped_center_px();
        Mat4::from_translation(Vec3::new(self.screen_w * 0.5, self.screen_h * 0.5, 0.0))
            * Mat4::from_scale(Vec3::new(self.zoom, self.zoom, 1.0))
            * Mat4::from_translation(Vec3::new(-cx, -cy, 0.0))
    }

    /// World pixels to clip space.
    pub fn view_proj(&self) -> Mat4 {
        self.screen_proj() * self.view()
    }

    /// The visible area in world pixels: (min_x, min_y, max_x, max_y).
    pub fn visible_world_px(&self) -> (f32, f32, f32, f32) {
        let (cx, cy) = self.snapped_center_px();
        let half_w = self.screen_w * 0.5 / self.zoom;
        let half_h = self.screen_h * 0.5 / self.zoom;
        (cx - half_w, cy - half_h, cx + half_w, cy + half_h)
    }

    /// Screen pixels to world tiles, the inverse of [`Camera::view`].
    pub fn screen_to_tile(&self, mouse_x: f32, mouse_y: f32) -> MouseCoords {
        let (cx, cy) = self.snapped_center_px();
        let world_x_px = (mouse_x - self.screen_w * 0.5) / self.zoom + cx;
        let world_y_px = (mouse_y - self.screen_h * 0.5) / self.zoom + cy;
        MouseCoords {
            x: world_x_px / TILE_SIZE,
            y: world_y_px / TILE_SIZE,
        }
    }

    pub fn zoom_to_fit_horizontal_tiles(&self, tiles: u32) -> f32 {
        if tiles == 0 {
            return self.max_zoom;
        }
        let desired = self.screen_w / (tiles as f32 * TILE_SIZE);
        desired.clamp(self.min_zoom, self.max_zoom)
    }

    pub fn zoom_to_fit_vertical_tiles(&self, tiles: u32) -> f32 {
        if tiles == 0 {
            return self.max_zoom;
        }
        let desired = self.screen_h / (tiles as f32 * TILE_SIZE);
        desired.clamp(self.min_zoom, self.max_zoom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_tile_inverts_view() {
        let camera = Camera::new(12.3, 4.56, 2.7, 800.0, 600.0);
        let screen = camera.view() * glam::Vec4::new(150.0, 90.0, 0.0, 1.0);
        let tile = camera.screen_to_tile(screen.x, screen.y);
        assert!((tile.x * TILE_SIZE - 150.0).abs() < 1e-3, "{tile:?}");
        assert!((tile.y * TILE_SIZE - 90.0).abs() < 1e-3, "{tile:?}");
    }

    #[test]
    fn camera_center_is_screen_center() {
        let camera = Camera::new(10.0, 5.0, 3.0, 800.0, 600.0);
        let center = camera.view() * glam::Vec4::new(160.0, 80.0, 0.0, 1.0);
        assert_eq!((center.x, center.y), (400.0, 300.0));
    }
}
