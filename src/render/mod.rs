//! Rendering. Owns the miniquad context, the GPU resources and the per-frame
//! batches. Game state never draws directly; it describes what to draw through
//! the methods here, and [`Renderer::draw`] decides the order:
//!
//! 1. Tiles (dual-grid layers, tile shader).
//! 2. World sprites and rectangles, in the order they were submitted
//!    (overlay tiles, doors, items, enemies, player, health bars).
//! 3. The "dark" overlay over rooms other than the current one (tile shader).
//! 4. The HUD in screen pixels (player health bar, minimap).
//!
//! World coordinates passed to the drawing methods are in tiles; the vertex
//! data is in world pixels (`TILE_SIZE` pixels per tile).

// Rendering does not affect game state, so platform-dependent float math is fine here.
#![allow(clippy::disallowed_methods)]

pub mod batch;
mod game;
mod shaders;
pub mod tiles;

pub use batch::UvRect;
pub use game::DrawableGameState;

use crate::atlas_info::AtlasInfo;
use crate::camera::Camera;
use crate::state::draw::{DrawItem, DrawList, Sprite};
use crate::state::map_like::MapLike;
use crate::state::{BaseTile, OverlayTile};
use batch::{QuadBatch, Vertex, WHITE};
use glam::Mat4;
use image::GenericImageView;
use miniquad::*;
use tiles::{CACHE_MARGIN, CellRange, TileCacheKey, TileLayer, TileMesh};

pub use crate::state::common::TILE_SIZE;

/// Ladders, platforms and start doors are atlas sprites drawn over the tiles.
fn overlay_tile_sprites(map: &dyn MapLike, out: &mut DrawList) {
    for item in map.get_overlays() {
        let frames: &[u32] = match item.tile {
            OverlayTile::None => &[],
            OverlayTile::Ladder => &[0],
            OverlayTile::Platform => &[2],
            OverlayTile::LadderPlatform => &[0, 2],
            OverlayTile::StartDoor => &[7],
        };
        for &frame in frames {
            let (x, y) = (item.x as f32, item.y as f32);
            out.sprite(Sprite::new("tiles", frame, x, y, 1.0, 1.0));
        }
    }
}

/// Size of one repeating background region in `tile_backgrounds.png`, in pixels.
const TILE_BACKGROUND_REGION: f32 = 64.0;

struct TextureInfo {
    w: f32,
    h: f32,
    texture: TextureId,
}

fn load_texture(ctx: &mut Context, path: &str) -> TextureInfo {
    let img = image::open(path).unwrap_or_else(|e| panic!("failed to load {path}: {e}"));
    let (w, h) = img.dimensions();
    texture_from_rgba8(ctx, w, h, &img.to_rgba8())
}

/// Nearest filtering for pixel art, clamped edges.
fn texture_from_rgba8(ctx: &mut Context, w: u32, h: u32, pixels: &[u8]) -> TextureInfo {
    let texture = ctx.new_texture_from_rgba8(w as u16, h as u16, pixels);
    ctx.texture_set_filter(texture, FilterMode::Nearest, MipmapFilterMode::None);
    ctx.texture_set_wrap(texture, TextureWrap::Clamp, TextureWrap::Clamp);
    TextureInfo {
        w: w as f32,
        h: h as f32,
        texture,
    }
}

struct Textures {
    /// 1x1 white, for colored rectangles.
    white: TextureInfo,
    /// `tilemap16.png`: the dual-grid tile images, one 4x4 block per layer.
    tiles: TextureInfo,
    /// `tile_backgrounds.png`: shown through the magenta pixels of the tiles.
    tile_backgrounds: TextureInfo,
    /// `atlas.png`: all sprites.
    atlas: TextureInfo,
    /// Built on the CPU by the minimap (see [`Renderer::set_minimap_image`]).
    minimap: TextureInfo,
}

/// A vertex and index buffer that grows as needed. Each buffer is uploaded at
/// most once per frame.
struct GpuMesh {
    vb: BufferId,
    ib: BufferId,
    vb_cap: usize,
    ib_cap: usize,
}

impl GpuMesh {
    fn new(ctx: &mut Context, quads: usize) -> GpuMesh {
        GpuMesh {
            vb: Self::vertex_buffer(ctx, quads * 4),
            ib: Self::index_buffer(ctx, quads * 6),
            vb_cap: quads * 4,
            ib_cap: quads * 6,
        }
    }

    fn vertex_buffer(ctx: &mut Context, cap: usize) -> BufferId {
        ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<Vertex>(cap),
        )
    }

    fn index_buffer(ctx: &mut Context, cap: usize) -> BufferId {
        ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<u32>(cap),
        )
    }

    fn upload(&mut self, ctx: &mut Context, vertices: &[Vertex], indices: &[u32]) {
        if vertices.len() > self.vb_cap {
            ctx.delete_buffer(self.vb);
            self.vb_cap = vertices.len().next_power_of_two();
            self.vb = Self::vertex_buffer(ctx, self.vb_cap);
        }
        if indices.len() > self.ib_cap {
            ctx.delete_buffer(self.ib);
            self.ib_cap = indices.len().next_power_of_two();
            self.ib = Self::index_buffer(ctx, self.ib_cap);
        }
        if !indices.is_empty() {
            ctx.buffer_update(self.vb, BufferSource::slice(vertices));
            ctx.buffer_update(self.ib, BufferSource::slice(indices));
        }
    }

    fn bindings(&self, images: Vec<TextureId>) -> Bindings {
        Bindings {
            vertex_buffers: vec![self.vb],
            index_buffer: self.ib,
            images,
        }
    }
}

pub struct Renderer {
    pub ctx: Box<Context>,
    sprite_pipeline: Pipeline,
    tile_pipeline: Pipeline,
    textures: Textures,
    atlas_info: AtlasInfo,

    world: QuadBatch<TextureId>,
    world_gpu: GpuMesh,
    hud: QuadBatch<TextureId>,
    hud_gpu: GpuMesh,
    tiles: TileMesh,
    tiles_gpu: GpuMesh,
    /// What `tiles` (already uploaded to `tiles_gpu`) was built for.
    tile_cache: Option<TileCacheKey>,
    dark: TileMesh,
    dark_gpu: GpuMesh,
    /// Reused every frame for the world draw items.
    draw_list: DrawList,

    /// Offscreen target for the game rendering (used by screenshot capture).
    /// `None` renders to the window.
    render_target: Option<RenderPass>,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut ctx = window::new_rendering_backend();

        let vertex_attributes = [
            VertexAttribute::new("pos", VertexFormat::Float2),
            VertexAttribute::new("uv", VertexFormat::Float2),
            VertexAttribute::new("color", VertexFormat::Float4),
        ];
        let params = PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            cull_face: CullFace::Nothing,
            ..Default::default()
        };

        let sprite_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: shaders::sprite::VERTEX,
                    fragment: shaders::sprite::FRAGMENT,
                },
                shaders::sprite::meta(),
            )
            .expect("failed to compile the sprite shader");
        let sprite_pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &vertex_attributes,
            sprite_shader,
            params,
        );

        let tile_shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: shaders::tile::VERTEX,
                    fragment: shaders::tile::FRAGMENT,
                },
                shaders::tile::meta(),
            )
            .expect("failed to compile the tile shader");
        let tile_pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &vertex_attributes,
            tile_shader,
            params,
        );

        let textures = Textures {
            white: texture_from_rgba8(&mut *ctx, 1, 1, &[255, 255, 255, 255]),
            tiles: load_texture(&mut *ctx, "assets/tilemap16.png"),
            tile_backgrounds: load_texture(&mut *ctx, "assets/tile_backgrounds.png"),
            atlas: load_texture(&mut *ctx, "assets/atlas.png"),
            // Transparent placeholder until the minimap builds its image.
            minimap: texture_from_rgba8(&mut *ctx, 1, 1, &[0, 0, 0, 0]),
        };

        let world_gpu = GpuMesh::new(&mut *ctx, 4096);
        let hud_gpu = GpuMesh::new(&mut *ctx, 64);
        let tiles_gpu = GpuMesh::new(&mut *ctx, 4096);
        let dark_gpu = GpuMesh::new(&mut *ctx, 4096);

        Renderer {
            ctx,
            sprite_pipeline,
            tile_pipeline,
            textures,
            atlas_info: AtlasInfo::load_from_file(),
            world: QuadBatch::new(),
            world_gpu,
            hud: QuadBatch::new(),
            hud_gpu,
            tiles: TileMesh::default(),
            tiles_gpu,
            tile_cache: None,
            dark: TileMesh::default(),
            dark_gpu,
            draw_list: DrawList::new(),
            render_target: None,
        }
    }

    /// Renders into `pass` instead of the window (see [`Renderer::new_capture_target`]).
    pub fn set_render_target(&mut self, pass: Option<RenderPass>) {
        self.render_target = pass;
    }

    /// Creates an offscreen RGBA8 render target of the given size, for screenshots.
    pub fn new_capture_target(&mut self, width: u32, height: u32) -> (RenderPass, TextureId) {
        let texture = self.ctx.new_render_texture(TextureParams {
            width,
            height,
            format: TextureFormat::RGBA8,
            ..Default::default()
        });
        let pass = self.ctx.new_render_pass(texture, None);
        (pass, texture)
    }

    /// Reads an RGBA8 render texture back to the CPU, with the first row at the top.
    pub fn read_capture_pixels(&mut self, texture: TextureId, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        self.ctx.texture_read_pixels(texture, &mut pixels);
        // OpenGL returns the bottom row first; flip so the image is upright.
        let row = (width * 4) as usize;
        let mut flipped = Vec::with_capacity(pixels.len());
        for y in (0..height as usize).rev() {
            flipped.extend_from_slice(&pixels[y * row..(y + 1) * row]);
        }
        flipped
    }

    /// Draws one frame of `state`. See the module docs for the order.
    pub fn draw(
        &mut self,
        state: &mut dyn DrawableGameState,
        camera: &Camera,
        show_dark: bool,
        draw_big_map: bool,
    ) {
        let clear = PassAction::Clear {
            color: Some((0.08, 0.09, 0.10, 1.0)),
            depth: Some(1.0),
            stencil: Some(0),
        };
        self.ctx.begin_pass(self.render_target, clear);
        let view_proj = camera.view_proj();

        // 1. Tiles (rebuilt only when the map changes or the camera leaves the cached area)
        let range = CellRange::visible(camera);
        let tex_size = (self.textures.tiles.w, self.textures.tiles.h);
        let map = state.map();
        let revision = map.tile_revision();
        if !self
            .tile_cache
            .is_some_and(|key| key.covers(revision, &range))
        {
            let covered = range.expanded(CACHE_MARGIN);
            let base_is = |tile: BaseTile| move |x: i32, y: i32| map.get_at(x, y).0 == tile;
            self.tiles.clear();
            self.tiles
                .push_layer(TileLayer::Background, covered, tex_size, |_, _| true);
            self.tiles.push_layer(
                TileLayer::NotPartOfRoom,
                covered,
                tex_size,
                base_is(BaseTile::NotPartOfRoom),
            );
            self.tiles.push_layer(
                TileLayer::Stone,
                covered,
                tex_size,
                base_is(BaseTile::Stone),
            );
            self.tiles
                .push_layer(TileLayer::Wood, covered, tex_size, base_is(BaseTile::Wood));
            self.tiles_gpu
                .upload(&mut *self.ctx, &self.tiles.vertices, &self.tiles.indices);
            self.tile_cache = Some(TileCacheKey { revision, covered });
        }
        self.draw_tile_mesh(false, view_proj);

        // 2. World sprites and rectangles
        let mut list = std::mem::take(&mut self.draw_list);
        list.clear();
        overlay_tile_sprites(state.map(), &mut list);
        state.draw_world(&mut list);
        state.player().draw(&mut list);
        state.draw_world_top(&mut list);
        self.world.clear();
        for item in list.items() {
            self.push_world_item(item);
        }
        self.draw_list = list;
        Self::flush(
            &mut *self.ctx,
            &self.sprite_pipeline,
            &self.world,
            &mut self.world_gpu,
            view_proj,
        );

        // 3. The dark
        if show_dark && let Some(is_dark) = state.dark_tiles() {
            self.dark.clear();
            self.dark
                .push_layer(TileLayer::Dark, range, tex_size, is_dark);
            self.dark_gpu
                .upload(&mut *self.ctx, &self.dark.vertices, &self.dark.indices);
            self.draw_tile_mesh(true, view_proj);
        }

        // 4. HUD
        self.hud.clear();
        self.draw_player_health_bar(state.player().health.ratio(), camera);
        state.update_and_draw_minimap(self, camera, draw_big_map);
        Self::flush(
            &mut *self.ctx,
            &self.sprite_pipeline,
            &self.hud,
            &mut self.hud_gpu,
            camera.screen_proj(),
        );

        self.ctx.end_render_pass();
    }

    /// Uploads a quad batch and draws it, one draw call per texture segment.
    fn flush(
        ctx: &mut Context,
        pipeline: &Pipeline,
        batch: &QuadBatch<TextureId>,
        gpu: &mut GpuMesh,
        mvp: Mat4,
    ) {
        if batch.is_empty() {
            return;
        }
        gpu.upload(ctx, batch.vertices(), batch.indices());
        ctx.apply_pipeline(pipeline);
        for segment in batch.segments() {
            ctx.apply_bindings(&gpu.bindings(vec![segment.texture]));
            ctx.apply_uniforms(UniformsSource::table(&shaders::sprite::Uniforms {
                mvp: mvp.to_cols_array(),
            }));
            ctx.draw(segment.first_index as i32, segment.index_count as i32, 1);
        }
    }

    /// Draws the already uploaded tile mesh (or the dark mesh), one draw call per layer.
    fn draw_tile_mesh(&mut self, dark: bool, mvp: Mat4) {
        let (mesh, gpu) = if dark {
            (&self.dark, &self.dark_gpu)
        } else {
            (&self.tiles, &self.tiles_gpu)
        };
        let backgrounds = &self.textures.tile_backgrounds;
        self.ctx.apply_pipeline(&self.tile_pipeline);
        self.ctx
            .apply_bindings(&gpu.bindings(vec![self.textures.tiles.texture, backgrounds.texture]));
        for layer in &mesh.layers {
            if layer.index_count == 0 {
                continue;
            }
            let region_x = TILE_BACKGROUND_REGION * layer.layer as u32 as f32;
            self.ctx
                .apply_uniforms(UniformsSource::table(&shaders::tile::Uniforms {
                    mvp: mvp.to_cols_array(),
                    bg_region: [
                        region_x,
                        0.0,
                        TILE_BACKGROUND_REGION,
                        TILE_BACKGROUND_REGION,
                    ],
                    bg_tex_size: [backgrounds.w, backgrounds.h, 0.0, 0.0],
                }));
            self.ctx
                .draw(layer.first_index as i32, layer.index_count as i32, 1);
        }
    }

    fn draw_player_health_bar(&mut self, ratio: f32, camera: &Camera) {
        let max_width = 200.0;
        let height = 20.0;
        let padding = 10.0;

        let x = camera.screen_w - max_width - padding;
        let y = padding;

        self.draw_hud_rect(x, y, max_width, height, [0.1, 0.1, 0.1, 1.0]);
        self.draw_hud_rect(x, y, max_width * ratio, height, [0.65, 0.11, 0.11, 1.0]);
    }

    /// Turns a draw item (in tiles) into quads in the world batch (world pixels).
    fn push_world_item(&mut self, item: &DrawItem) {
        let white = self.textures.white.texture;
        match item {
            DrawItem::Sprite(sprite) => {
                let atlas = &self.textures.atlas;
                let rect = self.atlas_info.get_rect(sprite.name, sprite.frame as i32);
                let uv = UvRect {
                    u0: rect.x as f32 / atlas.w,
                    v0: rect.y as f32 / atlas.h,
                    u1: (rect.x + rect.w) as f32 / atlas.w,
                    v1: (rect.y + rect.h) as f32 / atlas.h,
                };
                self.world.push_rect(
                    atlas.texture,
                    sprite.x * TILE_SIZE,
                    sprite.y * TILE_SIZE,
                    sprite.w * TILE_SIZE,
                    sprite.h * TILE_SIZE,
                    uv.flipped_x(sprite.flip),
                    [1.0, 1.0, 1.0, sprite.alpha],
                );
            }
            &DrawItem::Rect { x, y, w, h, color } => {
                self.world.push_rect(
                    white,
                    x * TILE_SIZE,
                    y * TILE_SIZE,
                    w * TILE_SIZE,
                    h * TILE_SIZE,
                    UvRect::FULL,
                    color,
                );
            }
            &DrawItem::RotatedRect {
                x,
                y,
                w,
                h,
                pivot_x,
                pivot_y,
                angle_rad,
                color,
            } => {
                let (s, c) = angle_rad.sin_cos();
                let corner = |cx: f32, cy: f32| {
                    let dx = cx - pivot_x;
                    let dy = cy - pivot_y;
                    [
                        (pivot_x + c * dx - s * dy) * TILE_SIZE,
                        (pivot_y + s * dx + c * dy) * TILE_SIZE,
                    ]
                };
                let corners = [
                    corner(x, y),
                    corner(x + w, y),
                    corner(x + w, y + h),
                    corner(x, y + h),
                ];
                self.world.push_quad(white, corners, UvRect::FULL, color);
            }
            &DrawItem::HealthBar { x, y, w, ratio } => {
                let padding = 0.3;
                let height = 0.1;
                let max_width = w + padding + padding;
                let x = x - padding;
                let y = y - height - padding;
                for (width, color) in [
                    (max_width, [0.1, 0.1, 0.1, 1.0]),
                    (max_width * ratio, [0.65, 0.11, 0.11, 1.0]),
                ] {
                    self.push_world_item(&DrawItem::Rect {
                        x,
                        y,
                        w: width,
                        h: height,
                        color,
                    });
                }
            }
        }
    }

    /// A colored rectangle on the HUD (screen pixels).
    pub fn draw_hud_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.hud
            .push_rect(self.textures.white.texture, x, y, w, h, UvRect::FULL, color);
    }

    /// Size of the current minimap image in pixels.
    pub fn minimap_image_size(&self) -> (u32, u32) {
        (
            self.textures.minimap.w as u32,
            self.textures.minimap.h as u32,
        )
    }

    /// Replaces the minimap image (RGBA8, `width * height * 4` bytes).
    pub fn set_minimap_image(&mut self, pixels: &[u8], width: u32, height: u32) {
        if self.minimap_image_size() == (width, height) {
            self.ctx
                .texture_update(self.textures.minimap.texture, pixels);
        } else {
            self.ctx.delete_texture(self.textures.minimap.texture);
            self.textures.minimap = texture_from_rgba8(&mut *self.ctx, width, height, pixels);
        }
    }

    /// Draws part of the minimap image on the HUD (screen pixels).
    pub fn draw_minimap_image(&mut self, x: f32, y: f32, w: f32, h: f32, uv: UvRect) {
        self.hud
            .push_rect(self.textures.minimap.texture, x, y, w, h, uv, WHITE);
    }
}
