//! Dual-grid autotiling: builds tile meshes on the CPU. No GPU calls.
//!
//! The dual grid is offset by half a tile from the map grid. Each dual-grid
//! cell looks at the four map tiles under its corners and picks one of 16
//! tile images (`DUAL_GRID_UV_TABLE`) from that corner mask.

use super::TILE_SIZE;
use super::batch::{Vertex, WHITE};
use crate::camera::Camera;

/// Which image of `tilemap16.png` to use for each corner mask
/// (bit 1 = top-left, 2 = top-right, 4 = bottom-left, 8 = bottom-right),
/// in tile units inside a layer's 4x4 block.
const DUAL_GRID_UV_TABLE: [(u32, u32); 16] = [
    (0, 0), // 0
    (1, 1), // 1
    (0, 1), // 2
    (0, 3), // 3
    (1, 0), // 4
    (1, 3), // 5
    (3, 2), // 6
    (2, 0), // 7
    (0, 0), // 8
    (2, 2), // 9
    (1, 2), // A
    (3, 0), // B
    (0, 2), // C
    (2, 1), // D
    (3, 1), // E
    (2, 3), // F
];

/// Tile layers: a 4x4 block of images in `tilemap16.png` and a 64x64 region
/// of `tile_backgrounds.png` (shown through the magenta pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileLayer {
    Stone = 0,
    Wood = 1,
    NotPartOfRoom = 2,
    Dark = 3,
    Background = 4,
}

/// A rectangle of dual-grid cells, `x0..x1` by `y0..y1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl CellRange {
    /// The dual-grid cells the camera can see, plus a small margin.
    pub fn visible(camera: &Camera) -> CellRange {
        let (min_x, min_y, max_x, max_y) = camera.visible_world_px();
        let offset = 0.5 * TILE_SIZE;
        CellRange {
            x0: ((min_x - offset) / TILE_SIZE).floor() as i32 - 1,
            y0: ((min_y - offset) / TILE_SIZE).floor() as i32 - 1,
            x1: ((max_x - offset) / TILE_SIZE).ceil() as i32 + 1,
            y1: ((max_y - offset) / TILE_SIZE).ceil() as i32 + 1,
        }
    }

    pub fn expanded(&self, margin: i32) -> CellRange {
        CellRange {
            x0: self.x0 - margin,
            y0: self.y0 - margin,
            x1: self.x1 + margin,
            y1: self.y1 + margin,
        }
    }

    pub fn contains(&self, other: &CellRange) -> bool {
        self.x0 <= other.x0 && self.y0 <= other.y0 && self.x1 >= other.x1 && self.y1 >= other.y1
    }

    fn cell_count(&self) -> usize {
        ((self.x1 - self.x0).max(0) * (self.y1 - self.y0).max(0)) as usize
    }
}

/// How many cells around the visible area a cached tile mesh covers, so the
/// camera can move a bit before the mesh has to be rebuilt.
pub const CACHE_MARGIN: i32 = 16;

/// What a cached tile mesh was built for: a map revision and an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileCacheKey {
    pub revision: u64,
    pub covered: CellRange,
}

impl TileCacheKey {
    /// Whether the mesh can draw the `needed` cells of a map at `revision`.
    pub fn covers(&self, revision: u64, needed: &CellRange) -> bool {
        self.revision == revision && self.covered.contains(needed)
    }
}

/// The corner mask of dual-grid cell `(x, y)`: which of the map tiles
/// `(x, y)`, `(x + 1, y)`, `(x, y + 1)` and `(x + 1, y + 1)` are filled.
fn corner_mask(filled: &impl Fn(i32, i32) -> bool, x: i32, y: i32) -> usize {
    (filled(x, y) as usize)
        | (filled(x + 1, y) as usize) << 1
        | (filled(x, y + 1) as usize) << 2
        | (filled(x + 1, y + 1) as usize) << 3
}

/// The indices of one layer inside a [`TileMesh`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerRange {
    pub layer: TileLayer,
    pub first_index: usize,
    pub index_count: usize,
}

/// Several tile layers in one vertex/index buffer, drawn in order.
#[derive(Default)]
pub struct TileMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub layers: Vec<LayerRange>,
}

impl TileMesh {
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.layers.clear();
    }

    /// Appends a layer covering `range`, where `filled(tx, ty)` says which map
    /// tiles belong to it. `tex_size` is the size of `tilemap16.png` in pixels.
    pub fn push_layer(
        &mut self,
        layer: TileLayer,
        range: CellRange,
        tex_size: (f32, f32),
        filled: impl Fn(i32, i32) -> bool,
    ) {
        let (tex_w, tex_h) = tex_size;
        let first_index = self.indices.len();
        self.vertices.reserve(range.cell_count() * 4);
        self.indices.reserve(range.cell_count() * 6);

        // Inset UVs by half a texel to avoid sampling across tile boundaries.
        let half_u = 0.5 / tex_w;
        let half_v = 0.5 / tex_h;
        let du = (TILE_SIZE - 1.0) / tex_w;
        let dv = (TILE_SIZE - 1.0) / tex_h;
        let layer_u = layer as u32 as f32 * 4.0 * TILE_SIZE / tex_w;
        let offset = 0.5 * TILE_SIZE;

        for y in range.y0..range.y1 {
            for x in range.x0..range.x1 {
                let mask = corner_mask(&filled, x, y);
                if mask == 0 {
                    continue;
                }
                let (u, v) = DUAL_GRID_UV_TABLE[mask];
                let u0 = u as f32 * TILE_SIZE / tex_w + half_u + layer_u;
                let v0 = v as f32 * TILE_SIZE / tex_h + half_v;
                let px = x as f32 * TILE_SIZE + offset;
                let py = y as f32 * TILE_SIZE + offset;

                let base = self.vertices.len() as u32;
                for (pos, uv) in [
                    ([px, py], [u0, v0]),
                    ([px + TILE_SIZE, py], [u0 + du, v0]),
                    ([px + TILE_SIZE, py + TILE_SIZE], [u0 + du, v0 + dv]),
                    ([px, py + TILE_SIZE], [u0, v0 + dv]),
                ] {
                    self.vertices.push(Vertex {
                        pos,
                        uv,
                        color: WHITE,
                    });
                }
                self.indices
                    .extend([0, 1, 2, 0, 2, 3].iter().map(|i| base + i));
            }
        }

        self.layers.push(LayerRange {
            layer,
            first_index,
            index_count: self.indices.len() - first_index,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corner_masks_around_a_single_tile() {
        let filled = |x: i32, y: i32| (x, y) == (0, 0);
        // The four dual-grid cells touching tile (0, 0) see it in different corners.
        assert_eq!(corner_mask(&filled, -1, -1), 8);
        assert_eq!(corner_mask(&filled, 0, -1), 4);
        assert_eq!(corner_mask(&filled, -1, 0), 2);
        assert_eq!(corner_mask(&filled, 0, 0), 1);
        assert_eq!(corner_mask(&filled, 1, 1), 0);
    }

    #[test]
    fn layers_append_to_one_mesh() {
        let mut mesh = TileMesh::default();
        let range = CellRange {
            x0: -2,
            y0: -2,
            x1: 2,
            y1: 2,
        };
        mesh.push_layer(TileLayer::Stone, range, (320.0, 64.0), |x, y| {
            (x, y) == (0, 0)
        });
        mesh.push_layer(TileLayer::Background, range, (320.0, 64.0), |_, _| true);

        assert_eq!(mesh.layers[0].first_index, 0);
        assert_eq!(mesh.layers[0].index_count, 4 * 6); // 4 cells touch the tile
        assert_eq!(mesh.layers[1].first_index, 4 * 6);
        assert_eq!(mesh.layers[1].index_count, 16 * 6); // every cell in range
        assert_eq!(mesh.vertices.len(), (4 + 16) * 4);
        // Cell (0, 0) starts half a tile right and down from tile (0, 0).
        assert_eq!(mesh.vertices[3 * 4].pos, [8.0, 8.0]);
    }

    #[test]
    fn cache_key_needs_same_revision_and_area() {
        let visible = CellRange {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        let key = TileCacheKey {
            revision: 7,
            covered: visible.expanded(CACHE_MARGIN),
        };
        assert!(key.covers(7, &visible));
        assert!(!key.covers(8, &visible), "map changed");
        let moved = CellRange {
            x0: visible.x0 + CACHE_MARGIN + 1,
            x1: visible.x1 + CACHE_MARGIN + 1,
            ..visible
        };
        assert!(!key.covers(7, &moved), "camera left the cached area");
    }

    #[test]
    fn range_containment() {
        let inner = CellRange {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        assert!(inner.expanded(2).contains(&inner));
        assert!(!inner.contains(&inner.expanded(1)));
    }
}
