//! CPU-side quad batching. Pure data, no GPU calls, so it can be unit tested.

/// One vertex of every mesh the renderer draws (sprites, rectangles, tiles).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

/// A texture region in UV coordinates (0..1). `u0 > u1` flips horizontally.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl UvRect {
    /// The whole texture.
    pub const FULL: UvRect = UvRect {
        u0: 0.0,
        v0: 0.0,
        u1: 1.0,
        v1: 1.0,
    };

    /// Mirrors the region horizontally when `flip` is true.
    pub fn flipped_x(self, flip: bool) -> UvRect {
        if flip {
            UvRect {
                u0: self.u1,
                u1: self.u0,
                ..self
            }
        } else {
            self
        }
    }
}

pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// Indices of the two triangles of a quad whose corners are pushed in the
/// order top-left, top-right, bottom-right, bottom-left.
const QUAD_INDICES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// A run of consecutive indices drawn with the same texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment<T> {
    pub texture: T,
    pub first_index: usize,
    pub index_count: usize,
}

/// Quads collected in submission order. Consecutive quads with the same
/// texture share a [`Segment`], so a frame is one draw call per texture
/// change, and later quads are always drawn on top of earlier ones.
pub struct QuadBatch<T> {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    segments: Vec<Segment<T>>,
}

impl<T: Copy + PartialEq> Default for QuadBatch<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + PartialEq> QuadBatch<T> {
    pub fn new() -> Self {
        QuadBatch {
            vertices: Vec::new(),
            indices: Vec::new(),
            segments: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.segments.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn segments(&self) -> &[Segment<T>] {
        &self.segments
    }

    /// Adds a quad with corners in the order top-left, top-right,
    /// bottom-right, bottom-left.
    pub fn push_quad(&mut self, texture: T, corners: [[f32; 2]; 4], uv: UvRect, color: [f32; 4]) {
        let uvs = [
            [uv.u0, uv.v0],
            [uv.u1, uv.v0],
            [uv.u1, uv.v1],
            [uv.u0, uv.v1],
        ];
        let base = self.vertices.len() as u32;
        for (pos, uv) in corners.into_iter().zip(uvs) {
            self.vertices.push(Vertex { pos, uv, color });
        }
        self.indices.extend(QUAD_INDICES.iter().map(|i| base + i));

        match self.segments.last_mut() {
            Some(segment) if segment.texture == texture => segment.index_count += 6,
            _ => self.segments.push(Segment {
                texture,
                first_index: self.indices.len() - 6,
                index_count: 6,
            }),
        }
    }

    /// Adds an axis-aligned rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn push_rect(
        &mut self,
        texture: T,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv: UvRect,
        color: [f32; 4],
    ) {
        let corners = [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        self.push_quad(texture, corners, uv, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(batch: &mut QuadBatch<u8>, texture: u8) {
        batch.push_rect(texture, 0.0, 0.0, 1.0, 1.0, UvRect::FULL, WHITE);
    }

    #[test]
    fn same_texture_quads_share_a_segment() {
        let mut batch = QuadBatch::new();
        rect(&mut batch, 1);
        rect(&mut batch, 1);
        assert_eq!(
            batch.segments(),
            &[Segment {
                texture: 1,
                first_index: 0,
                index_count: 12
            }]
        );
        assert_eq!(batch.vertices().len(), 8);
        assert_eq!(&batch.indices()[6..], &[4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn texture_changes_keep_submission_order() {
        let mut batch = QuadBatch::new();
        rect(&mut batch, 1);
        rect(&mut batch, 2);
        rect(&mut batch, 1);
        let segments: Vec<_> = batch
            .segments()
            .iter()
            .map(|s| (s.texture, s.first_index, s.index_count))
            .collect();
        assert_eq!(segments, vec![(1, 0, 6), (2, 6, 6), (1, 12, 6)]);
    }

    #[test]
    fn indices_go_past_u16() {
        let mut batch = QuadBatch::new();
        for _ in 0..20_000 {
            rect(&mut batch, 0);
        }
        assert_eq!(*batch.indices().last().unwrap(), 20_000 * 4 - 1);
    }

    #[test]
    fn rect_corners_and_flipped_uvs() {
        let mut batch = QuadBatch::new();
        let uv = UvRect {
            u0: 0.25,
            v0: 0.0,
            u1: 0.5,
            v1: 1.0,
        };
        batch.push_rect(0u8, 2.0, 3.0, 4.0, 5.0, uv.flipped_x(true), WHITE);
        let v = batch.vertices();
        assert_eq!(v[0].pos, [2.0, 3.0]);
        assert_eq!(v[2].pos, [6.0, 8.0]);
        assert_eq!(v[0].uv, [0.5, 0.0]);
        assert_eq!(v[1].uv, [0.25, 0.0]);
    }

    #[test]
    fn clear_empties_everything() {
        let mut batch = QuadBatch::new();
        rect(&mut batch, 0);
        batch.clear();
        assert!(batch.is_empty());
        assert!(batch.segments().is_empty());
        assert!(batch.vertices().is_empty());
    }
}
