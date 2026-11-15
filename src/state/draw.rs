//! What to draw, described by the game state as plain data. The renderer turns
//! a [`DrawList`] into quads; nothing here depends on it. Coordinates are in
//! tiles, and items are drawn in the order they were added.

use crate::state::common::{BoundingBox, TILE_SIZE};

/// A sprite frame from the atlas, stretched over a rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    /// The sprite name in the atlas (`"<name> <frame>.aseprite"`).
    pub name: &'static str,
    pub frame: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// Mirrored horizontally.
    pub flip: bool,
    /// 0 = invisible, 1 = opaque.
    pub alpha: f32,
}

impl Sprite {
    pub fn new(name: &'static str, frame: u32, x: f32, y: f32, w: f32, h: f32) -> Sprite {
        Sprite {
            name,
            frame,
            x,
            y,
            w,
            h,
            flip: false,
            alpha: 1.0,
        }
    }

    /// A sprite one art pixel larger than `bb` on every side; the usual size
    /// for characters and enemies, whose sprites have a 1 px border.
    pub fn around(name: &'static str, frame: u32, bb: &BoundingBox) -> Sprite {
        let px = 1.0 / TILE_SIZE;
        Sprite::new(
            name,
            frame,
            bb.x - px,
            bb.y - px,
            bb.w + 2.0 * px,
            bb.h + 2.0 * px,
        )
    }

    pub fn flipped(self, flip: bool) -> Sprite {
        Sprite { flip, ..self }
    }

    pub fn with_alpha(self, alpha: f32) -> Sprite {
        Sprite { alpha, ..self }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DrawItem {
    Sprite(Sprite),
    /// A colored rectangle.
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
    },
    /// A colored rectangle rotated by `angle_rad` around `(pivot_x, pivot_y)`.
    RotatedRect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        pivot_x: f32,
        pivot_y: f32,
        angle_rad: f32,
        color: [f32; 4],
    },
    /// A health bar above a box whose top-left is `(x, y)` and width `w`.
    HealthBar {
        x: f32,
        y: f32,
        w: f32,
        ratio: f32,
    },
}

/// Draw items in drawing order (later ones on top).
#[derive(Default)]
pub struct DrawList {
    items: Vec<DrawItem>,
}

impl DrawList {
    pub fn new() -> DrawList {
        DrawList::default()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn items(&self) -> &[DrawItem] {
        &self.items
    }

    pub fn sprite(&mut self, sprite: Sprite) {
        if sprite.alpha > 0.0 {
            self.items.push(DrawItem::Sprite(sprite));
        }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.items.push(DrawItem::Rect { x, y, w, h, color });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rotated_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        pivot_x: f32,
        pivot_y: f32,
        angle_rad: f32,
        color: [f32; 4],
    ) {
        self.items.push(DrawItem::RotatedRect {
            x,
            y,
            w,
            h,
            pivot_x,
            pivot_y,
            angle_rad,
            color,
        });
    }

    pub fn health_bar_above(&mut self, bb: &BoundingBox, ratio: f32) {
        self.items.push(DrawItem::HealthBar {
            x: bb.x,
            y: bb.y,
            w: bb.w,
            ratio,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_around_adds_one_pixel_on_each_side() {
        let bb = BoundingBox {
            x: 2.0,
            y: 3.0,
            w: 1.0,
            h: 0.5,
            vx: 0.0,
            vy: 0.0,
        };
        let s = Sprite::around("slime", 1, &bb);
        assert_eq!((s.x, s.y), (2.0 - 1.0 / 16.0, 3.0 - 1.0 / 16.0));
        assert_eq!((s.w, s.h), (1.0 + 2.0 / 16.0, 0.5 + 2.0 / 16.0));
        assert!(!s.flip);
        assert_eq!(s.alpha, 1.0);
    }

    #[test]
    fn invisible_sprites_are_skipped_and_order_is_kept() {
        let mut list = DrawList::new();
        list.sprite(Sprite::new("a", 0, 0.0, 0.0, 1.0, 1.0));
        list.sprite(Sprite::new("hidden", 0, 0.0, 0.0, 1.0, 1.0).with_alpha(0.0));
        list.rect(0.0, 0.0, 1.0, 1.0, [1.0; 4]);
        let kinds: Vec<_> = list
            .items()
            .iter()
            .map(|item| match item {
                DrawItem::Sprite(s) => s.name,
                DrawItem::Rect { .. } => "rect",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, vec!["a", "rect"]);
    }
}
