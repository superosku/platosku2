use crate::camera::Camera;
use crate::state::GameState;
use crate::state::OverlayTile;
use crate::state::game_map::{DoorDir, MapLike};
use crate::state::game_state::{Editor, Game};
use crate::state::{BaseTile, Dir};

use image::GenericImageView;
use miniquad::*;
use std::collections::HashMap;
use crate::state::enemies::Enemy;

#[repr(C)]
struct Uniforms {
    mvp: [f32; 16],
    color: [f32; 4],
    uv_base: [f32; 4],          // xy used
    uv_scale: [f32; 4],         // xy used
    world_base: [f32; 4],       // xy used (world pixel origin of this quad)
    world_scale: [f32; 4],      // xy used (world pixel size of this quad)
    color_key: [f32; 4],        // rgb = key color, a = threshold
    bg_tile_size: [f32; 4],     // xy used (repeat period in pixels)
    bg_region_origin: [f32; 4], // xy used (top-left of 64x64 region in bg texture, in pixels)
    bg_tex_size: [f32; 4],      // xy used (bg texture size in pixels)
}

#[repr(C)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub struct Renderer {
    pub ctx: Box<Context>,
    pipeline: Pipeline,
    pipeline_tiles: Pipeline,
    pipeline_hud: Pipeline,
    bindings: Bindings,
    textures: HashMap<TextureIndexes, TextureInfo>,
}

#[derive(Eq, PartialEq, Hash)]
pub enum TextureIndexes {
    White1x1,
    Tile,
    TileBackground,
    Player,
    Bat,
    Slime,
    Door,
}

struct TextureInfo {
    w: f32,
    h: f32,
    texture: TextureId,
}

fn load_texture(ctx: &mut Box<dyn RenderingBackend>, path: &str) -> TextureInfo {
    // Load background texture (tiled 64x64 area) from assets
    let img = image::open(path).expect("failed to load assets/tile_backgrounds.png");
    let (w, h) = img.dimensions();
    let bg_rgba8 = img.to_rgba8();
    let texture = ctx.new_texture_from_rgba8(w as u16, h as u16, &bg_rgba8);
    // Nearest for pixel art, Repeat so UVs can wrap every 64 px
    ctx.texture_set_filter(texture, FilterMode::Nearest, MipmapFilterMode::None);
    ctx.texture_set_wrap(texture, TextureWrap::Clamp, TextureWrap::Clamp);

    TextureInfo {
        w: w as f32,
        h: h as f32,
        texture,
    }
}

const TILE_SIZE: f32 = 16.0;

const DUAL_GRID_UV_TABLE: [(u32, u32); 16] = [
    (0, 0), // 0
    (1, 1), // 1 # DONE
    (0, 1), // 2 # DONE
    (0, 3), // 3 # DONE
    (1, 0), // 4 # DONE
    (1, 3), // 5 # DONE
    (3, 2), // 6
    (2, 0), // 7 # DONE
    (0, 0), // 8 # DONE
    (2, 2), // 9
    (1, 2), // A # DONE
    (3, 0), // B
    (0, 2), // C # DONE
    (2, 1), // D # DONE
    (3, 1), // E # DONE
    (2, 3), // F # DONE
];

pub trait DrawableGameState: GameState {
    fn draw_extra(&self, camera: &Camera, renderer: &mut Renderer, show_dark: bool);
}

impl DrawableGameState for Game {
    fn draw_extra(&self, camera: &Camera, renderer: &mut Renderer, show_dark: bool) {
        // Draw the doors
        for door in &self.map.doors {
            renderer.draw_from_texture_atlas(
                camera,
                TextureIndexes::Door,
                door.get_atlas_index() as f32,
                false,
                door.x as f32,
                door.y as f32,
                1.0,
                1.0,
                1.0,
            );
        }

        // Coins
        for coin in &self.coins {
            renderer.draw_rect(
                camera,
                coin.bb.x,
                coin.bb.y,
                coin.bb.w,
                coin.bb.h,
                [1.0, 0.85, 0.2, 1.0],
            );
        }

        // draw enemies
        for enemy in &self.enemies {
            let bb = enemy.bb();
            // self.draw_rect(state, bb.x, bb.y, bb.w, bb.h, [0.5, 0.25, 0.25, 1.0]);
            renderer.draw_from_texture_atlas(
                camera,
                enemy.get_texture_index(),
                enemy.get_atlas_index() as f32,
                !enemy.goes_right(),
                bb.x - 1.0 / TILE_SIZE,
                bb.y - 1.0 / TILE_SIZE,
                bb.w + 2.0 / TILE_SIZE,
                bb.h + 2.0 / TILE_SIZE,
                1.0,
            );
            renderer.draw_enemy_health_bar(camera, enemy);
        }

        // Draw "the dark" (the overaly)
        if show_dark {
            let rooms = self.get_rooms_for_display();
            let ratio = rooms.2;
            renderer.draw_base_dual_grid(
                |x, y| {
                    if let Some(room) = rooms.0 {
                        if let Some((base, _overlay)) = room.get_relative(x, y) {
                            if base != BaseTile::NotPartOfRoom {
                                return false;
                            }
                        }
                    }
                    if let Some(room) = rooms.1 {
                        if let Some((base, _overlay)) = room.get_relative(x, y) {
                            if base != BaseTile::NotPartOfRoom {
                                return false;
                            }
                        }
                    }
                    true
                },
                camera,
                3,
                1.0,
            );
            // Draw again with opacity to get the fading effect
            if ratio != 1.0 && ratio != 0.0 {
                renderer.draw_base_dual_grid(
                    |x, y| {
                        if let Some(room) = rooms.1 {
                            if let Some((base, _overlay)) = room.get_relative(x, y) {
                                if base != BaseTile::NotPartOfRoom {
                                    return false;
                                }
                            }
                        }
                        true
                    },
                    camera,
                    3,
                    ratio,
                );
                renderer.draw_base_dual_grid(
                    |x, y| {
                        if let Some(room) = rooms.0 {
                            if let Some((base, _overlay)) = room.get_relative(x, y) {
                                if base != BaseTile::NotPartOfRoom {
                                    return false;
                                }
                            }
                        }
                        true
                    },
                    camera,
                    3,
                    1.0 - ratio,
                );
            }
        }
    }
}

impl DrawableGameState for Editor {
    fn draw_extra(&self, camera: &Camera, renderer: &mut Renderer, _show_dark: bool) {
        let tilemap = renderer.textures.get(&TextureIndexes::Tile).unwrap();
        let tex_w = tilemap.w;
        let tex_h = tilemap.h;
        let uv_scale = [(TILE_SIZE) / tex_w, (TILE_SIZE) / tex_h];

        for door in self.room.get_doors() {
            let room_pos = self.room.get_pos();
            let px = (room_pos.0 as f32 + door.x as f32) * TILE_SIZE;
            let py = (room_pos.1 as f32 + door.y as f32) * TILE_SIZE;

            let uv_base_px = [
                match door.dir {
                    DoorDir::Up => 0,
                    DoorDir::Right => 1,
                    DoorDir::Down => 2,
                    DoorDir::Left => 3,
                } as f32
                    * TILE_SIZE,
                5.0_f32 * TILE_SIZE,
            ];
            let uv_base = [uv_base_px[0] / tex_w, uv_base_px[1] / tex_h];

            renderer.draw_tile_textured(camera, px, py, [1.0, 1.0, 1.0, 1.0], uv_base, uv_scale, 0);
        }

        // draw enemy templates
        for template in &self.room.object_templates {
            let bb = template.get_bb();
            let texture_index = template.get_texture_index();
            renderer.draw_from_texture_atlas(
                camera,
                texture_index,
                0.0,
                false,
                bb.x - 1.0 / TILE_SIZE,
                bb.y - 1.0 / TILE_SIZE,
                bb.w + 2.0 / TILE_SIZE,
                bb.h + 2.0 / TILE_SIZE,
                1.0,
            );
        }
    }
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut ctx = window::new_rendering_backend();

        // unit quad with UVs (0..1)
        let vertices: [Vertex; 4] = [
            Vertex {
                pos: [0.0, 0.0],
                uv: [0.0, 0.0],
            },
            Vertex {
                pos: [1.0, 0.0],
                uv: [1.0, 0.0],
            },
            Vertex {
                pos: [1.0, 1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                pos: [0.0, 1.0],
                uv: [0.0, 1.0],
            },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        // Create a 1x1 white texture for colored rectangles
        let white_tex_bytes: [u8; 4] = [255, 255, 255, 255];
        let white_texture = ctx.new_texture_from_rgba8(1, 1, &white_tex_bytes);
        ctx.texture_set_filter(white_texture, FilterMode::Nearest, MipmapFilterMode::None);
        ctx.texture_set_wrap(white_texture, TextureWrap::Clamp, TextureWrap::Clamp);

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX_SHADER,
                    fragment: FRAGMENT_SHADER,
                },
                ShaderMeta {
                    images: vec!["tex".to_string(), "bg_tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("color", UniformType::Float4),
                            UniformDesc::new("uv_base", UniformType::Float4),
                            UniformDesc::new("uv_scale", UniformType::Float4),
                            UniformDesc::new("world_base", UniformType::Float4),
                            UniformDesc::new("world_scale", UniformType::Float4),
                            UniformDesc::new("color_key", UniformType::Float4),
                            UniformDesc::new("bg_tile_size", UniformType::Float4),
                            UniformDesc::new("bg_region_origin", UniformType::Float4),
                            UniformDesc::new("bg_tex_size", UniformType::Float4),
                        ],
                    },
                },
            )
            .expect("failed to compile shader");

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                cull_face: CullFace::Nothing,
                ..Default::default()
            },
        );

        let pipeline_hud = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams {
                depth_write: false,
                depth_test: Comparison::Always,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                cull_face: CullFace::Nothing,
                ..Default::default()
            },
        );

        // A second pipeline for batched tilemap rendering (positions in world pixels, UVs precomputed)
        let shader_tiles = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX_SHADER_TILES_BATCHED,
                    fragment: FRAGMENT_SHADER,
                },
                ShaderMeta {
                    images: vec!["tex".to_string(), "bg_tex".to_string()],
                    // Keep the same uniform block layout so we can reuse Uniforms struct
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("color", UniformType::Float4),
                            UniformDesc::new("uv_base", UniformType::Float4),
                            UniformDesc::new("uv_scale", UniformType::Float4),
                            UniformDesc::new("world_base", UniformType::Float4),
                            UniformDesc::new("world_scale", UniformType::Float4),
                            UniformDesc::new("color_key", UniformType::Float4),
                            UniformDesc::new("bg_tile_size", UniformType::Float4),
                            UniformDesc::new("bg_region_origin", UniformType::Float4),
                            UniformDesc::new("bg_tex_size", UniformType::Float4),
                        ],
                    },
                },
            )
            .expect("failed to compile batched tile shader");

        let pipeline_tiles = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("pos", VertexFormat::Float2),
                VertexAttribute::new("uv", VertexFormat::Float2),
            ],
            shader_tiles,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                cull_face: CullFace::Nothing,
                ..Default::default()
            },
        );

        let mut textures = HashMap::new();
        textures.insert(
            TextureIndexes::Tile,
            load_texture(&mut ctx, "assets/tilemap16.png"),
        );
        textures.insert(
            TextureIndexes::TileBackground,
            load_texture(&mut ctx, "assets/tile_backgrounds.png"),
        );
        textures.insert(
            TextureIndexes::Player,
            load_texture(&mut ctx, "assets/character.png"),
        );
        textures.insert(
            TextureIndexes::Bat,
            load_texture(&mut ctx, "assets/bat.png"),
        );
        textures.insert(
            TextureIndexes::Slime,
            load_texture(&mut ctx, "assets/slime.png"),
        );
        textures.insert(
            TextureIndexes::Door,
            load_texture(&mut ctx, "assets/door.png"),
        );
        textures.insert(
            TextureIndexes::White1x1,
            TextureInfo {
                w: 1.0,
                h: 1.0,
                texture: white_texture,
            },
        );

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![
                textures.get(&TextureIndexes::Tile).unwrap().texture,
                textures
                    .get(&TextureIndexes::TileBackground)
                    .unwrap()
                    .texture,
            ],
        };

        Renderer {
            ctx,
            pipeline,
            pipeline_tiles,
            pipeline_hud,
            bindings,
            textures,
        }
    }

    pub fn resize(&mut self, _w: f32, _h: f32) {
        // Nothing to do yet
    }

    pub fn draw(&mut self, state: &dyn DrawableGameState, camera: &Camera, show_dark: bool) {
        let clear = PassAction::Clear {
            color: Some((0.08, 0.09, 0.10, 1.0)),
            depth: Some(1.0),
            stencil: Some(0),
        };

        self.ctx.begin_default_pass(clear);
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);

        // Draw base grid using dual-grid textured tiles
        self.draw_base_dual_grid(
            |x, y| matches!(state.map().get_at(x, y).0, BaseTile::NotPartOfRoom),
            camera,
            2,
            1.0,
        );
        self.draw_base_dual_grid(
            |x, y| matches!(state.map().get_at(x, y).0, BaseTile::Stone),
            camera,
            0,
            1.0,
        );
        self.draw_base_dual_grid(
            |x, y| matches!(state.map().get_at(x, y).0, BaseTile::Wood),
            camera,
            1,
            1.0,
        );

        // Draw overlay tiles
        self.draw_overlay(state.map(), camera);

        // draw (coins and enemies) OR (doors)
        state.draw_extra(camera, self, show_dark);

        // draw player on top
        let px = state.player().bb.x;
        let py = state.player().bb.y;
        let pw = state.player().bb.w;
        let ph = state.player().bb.h;

        let alpha = if !state.player().can_be_hit() {
            ((state.player().immunity_frames / 10) % 2) as f32
        } else {
            1.0
        };

        // self.draw_rect(state, px, py, pw, ph, [0.20, 0.3, 0.40, 1.0], alpha);
        self.draw_from_texture_atlas(
            camera,
            TextureIndexes::Player,
            state.player().get_atlas_index() as f32,
            match state.player().dir {
                Dir::Left => true,
                Dir::Right => false,
            },
            px - 1.0 / TILE_SIZE,
            py - 1.0 / TILE_SIZE,
            pw + 2.0 / TILE_SIZE,
            ph + 2.0 / TILE_SIZE,
            alpha,
        );

        if let Some(swing_info) = state.player().get_swing_info() {
            self.draw_rect_rotated(
                camera,
                swing_info.pivot.x - 0.05,
                swing_info.pivot.y - 0.15,
                0.1,
                swing_info.length + 0.15,
                swing_info.pivot.x,
                swing_info.pivot.y,
                swing_info.angle_rad,
                [0.5, 0.5, 0.5, 1.0],
            );

            self.draw_rect(
                camera,
                swing_info.end.x - 0.05,
                swing_info.end.y - 0.05,
                0.1,
                0.1,
                [1.0, 0.5, 0.5, 1.0],
            )
        }
        self.ctx.end_render_pass();

        // Draw hud new HUD pipeline
        let no_clear = PassAction::Nothing;
        self.ctx.begin_default_pass(no_clear);
        self.ctx.apply_pipeline(&self.pipeline_hud);
        self.ctx.apply_bindings(&self.bindings);

        self.draw_hud(state, camera);

        self.ctx.end_render_pass();

    }

    pub fn draw_hud(&mut self, state: &dyn GameState, camera: &Camera) {
        self.draw_player_health_bar(state, camera);

        // currently only hp bar. possibility to add other things.
	}

    fn draw_player_health_bar(&mut self, state: &dyn GameState, camera: &Camera) {
        let max_width = 200.0;
        let height = 20.0;
        let padding = 10.0;

        let x = camera.screen_w - max_width - padding;
        let y = padding;

        let health_ratio = state.player().health.current as f32 / state.player().health.max as f32;
        let filled_width = max_width * health_ratio;

        self.draw_rect_hud(camera, x, y, max_width, height, [0.1, 0.1, 0.1, 1.0]);
        self.draw_rect_hud(camera, x, y, filled_width, height, [0.65, 0.11, 0.11, 1.0]);
    }

    fn draw_enemy_health_bar(&mut self, camera: &Camera, enemy: &Box<dyn Enemy>) {
        let padding = 0.3;
        let height = 0.1;
        let max_width = enemy.bb().w + padding + padding;
        let x = enemy.bb().x - padding;
        let y = enemy.bb().y - height - padding ;
        
        let health_ratio = enemy.get_health().current as f32 / enemy.get_health().max as f32;
        let filled_width = max_width * health_ratio;

		self.draw_rect(camera, x, y, max_width, height, [0.1, 0.1, 0.1, 1.0]);
        self.draw_rect(camera, x, y, filled_width, height, [0.65, 0.11, 0.11, 1.0]);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_from_texture_atlas(
        &mut self,
        camera: &Camera,
        texture_index: TextureIndexes,
        atlas_index: f32,
        flip: bool,
        px: f32,
        py: f32,
        w: f32,
        h: f32,
        alpha: f32,
    ) {
        // ensure tile texture bound
        let background = self.textures.get(&TextureIndexes::TileBackground).unwrap();
        let texture = self.textures.get(&texture_index).unwrap();

        self.bindings.images[0] = texture.texture;
        self.bindings.images[1] = background.texture;

        self.ctx.apply_bindings(&self.bindings);

        let view = Self::camera_view(camera);
        let proj = Self::ortho_mvp(camera);
        let model = Self::mat4_mul(
            Self::mat4_translation(px * TILE_SIZE, py * TILE_SIZE),
            Self::mat4_scale(w * TILE_SIZE, h * TILE_SIZE),
        );
        let vp = Self::mat4_mul(proj, view);
        let mvp = Self::mat4_mul(vp, model);

        let width_ratio = w * TILE_SIZE / texture.w;
        let height_ratio = h * TILE_SIZE / texture.h;

        let mut uv_base_x = atlas_index * width_ratio;
        let mut uv_scale_x = width_ratio;

        if flip {
            uv_base_x += width_ratio;
            uv_scale_x = -uv_scale_x;
        }

        let uniforms = Uniforms {
            mvp,
            color: [1.0, 1.0, 1.0, alpha],

            uv_base: [uv_base_x, 0.0, 0.0, 0.0],
            uv_scale: [uv_scale_x, height_ratio, 0.0, 0.0],

            world_base: [px * TILE_SIZE, py * TILE_SIZE, 0.0, 0.0],
            world_scale: [w * TILE_SIZE, h * TILE_SIZE, 0.0, 0.0],

            color_key: [1.0, 0.0, 1.0, 0.01], // bright magenta with small threshold

            bg_tile_size: [background.w, background.h, 0.0, 0.0],
            bg_region_origin: [0.0, 0.0, 0.0, 0.0],
            bg_tex_size: [background.w, background.h, 0.0, 0.0],
        };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_tile_textured(
        &mut self,
        camera: &Camera,
        px: f32,
        py: f32,
        color: [f32; 4],
        uv_base: [f32; 2],
        uv_scale: [f32; 2],
        tile_type_index: u8,
    ) {
        // ensure tile texture bound
        let background = self.textures.get(&TextureIndexes::TileBackground).unwrap();
        let tile = self.textures.get(&TextureIndexes::Tile).unwrap();
        self.bindings.images[0] = tile.texture;
        self.bindings.images[1] = background.texture;
        self.ctx.apply_bindings(&self.bindings);

        let view = Self::camera_view(camera);
        let proj = Self::ortho_mvp(camera);
        let model = Self::mat4_mul(
            Self::mat4_translation(px, py),
            Self::mat4_scale(TILE_SIZE, TILE_SIZE),
        );
        let vp = Self::mat4_mul(proj, view);
        let mvp = Self::mat4_mul(vp, model);

        let uniforms = Uniforms {
            mvp,
            color,
            uv_base: [uv_base[0], uv_base[1], 0.0, 0.0],
            uv_scale: [uv_scale[0], uv_scale[1], 0.0, 0.0],
            world_base: [px, py, 0.0, 0.0],
            world_scale: [TILE_SIZE, TILE_SIZE, 0.0, 0.0],
            color_key: [1.0, 0.0, 1.0, 0.01], // bright magenta with small threshold
            bg_tile_size: [64.0, 64.0, 0.0, 0.0],
            bg_region_origin: [64.0 * tile_type_index as f32, 0.0, 0.0, 0.0],
            bg_tex_size: [background.w, background.h, 0.0, 0.0],
        };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_rect(&mut self, camera: &Camera, px: f32, py: f32, w: f32, h: f32, color: [f32; 4]) {
        // bind white texture and use full-quad UVs
        let background = self.textures.get(&TextureIndexes::TileBackground).unwrap();
        let white = self.textures.get(&TextureIndexes::White1x1).unwrap();

        self.bindings.images[0] = white.texture;
        self.bindings.images[1] = background.texture;

        self.ctx.apply_bindings(&self.bindings);

        let view = Self::camera_view(camera);
        let proj = Self::ortho_mvp(camera);
        let model = Self::mat4_mul(
            Self::mat4_translation(px * TILE_SIZE, py * TILE_SIZE),
            Self::mat4_scale(w * TILE_SIZE, h * TILE_SIZE),
        );
        let vp = Self::mat4_mul(proj, view);
        let mvp = Self::mat4_mul(vp, model);

        let uniforms = Uniforms {
            mvp,
            color,
            uv_base: [0.0, 0.0, 0.0, 0.0],
            uv_scale: [1.0, 1.0, 0.0, 0.0],
            world_base: [px * TILE_SIZE, py * TILE_SIZE, 0.0, 0.0],
            world_scale: [w * TILE_SIZE, h * TILE_SIZE, 0.0, 0.0],
            color_key: [1.0, 0.0, 1.0, 0.01],
            bg_tile_size: [64.0, 64.0, 0.0, 0.0],
            bg_region_origin: [0.0, 0.0, 0.0, 0.0],
            bg_tex_size: [background.w, background.h, 0.0, 0.0],
        };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_rect_hud(&mut self, camera: &Camera, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let background = self.textures.get(&TextureIndexes::TileBackground).unwrap();
        let white = self.textures.get(&TextureIndexes::White1x1).unwrap();

        self.bindings.images[0] = white.texture;
        self.bindings.images[1] = background.texture;
        self.ctx.apply_bindings(&self.bindings);

        let proj = Self::ortho_mvp(camera);
        let model = Self::mat4_mul(
            Self::mat4_translation(x, y),
            Self::mat4_scale(w, h),
        );
        let mvp = Self::mat4_mul(proj, model);

        let uniforms = Uniforms {
            mvp,
            color,
            uv_base: [0.0, 0.0, 0.0, 0.0],
            uv_scale: [1.0, 1.0, 0.0, 0.0],
            world_base: [x, y, 0.0, 0.0],
            world_scale: [w, h, 0.0, 0.0],
            color_key: [1.0, 0.0, 1.0, 0.01],
            bg_tile_size: [64.0, 64.0, 0.0, 0.0],
            bg_region_origin: [0.0, 0.0, 0.0, 0.0],
            bg_tex_size: [background.w, background.h, 0.0, 0.0],
        };

        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_rect_rotated(
        &mut self,
        camera: &Camera,
        px: f32,
        py: f32,
        w: f32,
        h: f32,
        pivot_x: f32,
        pivot_y: f32,
        angle_rad: f32,
        color: [f32; 4],
    ) {
        let background = self.textures.get(&TextureIndexes::TileBackground).unwrap();
        let white = self.textures.get(&TextureIndexes::White1x1).unwrap();
        // bind white texture and use full-quad UVs
        self.bindings.images[0] = white.texture;
        self.bindings.images[1] = background.texture;
        self.ctx.apply_bindings(&self.bindings);

        let view = Self::camera_view(camera);
        let proj = Self::ortho_mvp(camera);

        // --- build model matrix with pivot rotation ---
        let pxw = px * TILE_SIZE;
        let pyw = py * TILE_SIZE;
        let ww = w * TILE_SIZE;
        let hw = h * TILE_SIZE;

        let pivot_wx = pivot_x * TILE_SIZE;
        let pivot_wy = pivot_y * TILE_SIZE;

        // Order (column-vector convention): M = T(pivot) * R * T(-pivot) * T(pos) * S
        // But because your rect is positioned by translating its origin (px,py),
        // a clean way is: T(pivot) * R * T(pos - pivot) * S
        let t_pivot = Self::mat4_translation(pivot_wx, pivot_wy);
        let r = Self::mat4_rotation_z(angle_rad);
        let t_from_pivot = Self::mat4_translation(pxw - pivot_wx, pyw - pivot_wy);
        let s = Self::mat4_scale(ww, hw);

        let model = Self::mat4_mul(Self::mat4_mul(Self::mat4_mul(t_pivot, r), t_from_pivot), s);

        let vp = Self::mat4_mul(proj, view);
        let mvp = Self::mat4_mul(vp, model);

        let uniforms = Uniforms {
            mvp,
            color,
            uv_base: [0.0, 0.0, 0.0, 0.0],
            uv_scale: [1.0, 1.0, 0.0, 0.0],
            world_base: [pxw, pyw, 0.0, 0.0],
            world_scale: [ww, hw, 0.0, 0.0],
            color_key: [1.0, 0.0, 1.0, 0.01],
            bg_tile_size: [64.0, 64.0, 0.0, 0.0],
            bg_region_origin: [0.0, 0.0, 0.0, 0.0],
            bg_tex_size: [background.w, background.h, 0.0, 0.0],
        };

        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_overlay(&mut self, map: &dyn MapLike, camera: &Camera) {
        // let width = state.map.base.first().map(|r| r.len()).unwrap_or(0);
        // let height = state.map.base.len();
        // if width == 0 || height == 0 {
        //     return;
        // }

        let tilemap = self.textures.get(&TextureIndexes::Tile).unwrap();
        let tex_w = tilemap.w;
        let tex_h = tilemap.h;

        // Apply half-tile offset: 0.5 left (negative X), 0.5 down (positive Y)
        let offset_x = 0.0;
        let offset_y = 0.0;

        // Compute visible world bounds from camera (expand slightly to avoid edge gaps)
        let zoom = camera.zoom;
        let half_w_world = camera.screen_w * 0.5 / zoom;
        let half_h_world = camera.screen_h * 0.5 / zoom;
        let world_min_x = camera.x * TILE_SIZE - half_w_world - TILE_SIZE;
        let world_min_y = camera.y * TILE_SIZE - half_h_world - TILE_SIZE;
        let world_max_x = camera.x * TILE_SIZE + half_w_world + TILE_SIZE;
        let world_max_y = camera.y * TILE_SIZE + half_h_world + TILE_SIZE;

        // Convert world bounds to dual-grid tile indices
        let start_x = ((world_min_x - offset_x) / TILE_SIZE).floor() as i32;
        let end_x = ((world_max_x - offset_x) / TILE_SIZE).ceil() as i32;
        let start_y = ((world_min_y - offset_y) / TILE_SIZE).floor() as i32;
        let end_y = ((world_max_y - offset_y) / TILE_SIZE).ceil() as i32;

        for y in start_y..end_y {
            for x in start_x..end_x {
                let uv_scale = [(TILE_SIZE) / tex_w, (TILE_SIZE) / tex_h];

                let px = x as f32 * TILE_SIZE;
                let py = y as f32 * TILE_SIZE;

                let uv_base = match map.get_at(x, y).1 {
                    OverlayTile::None => {
                        continue;
                    }
                    OverlayTile::Ladder => {
                        let uv_base_px = if map.is_ladder_at(x, y - 1) || map.is_solid_at(x, y - 1)
                        {
                            [0.0_f32 * TILE_SIZE, 4.0_f32 * TILE_SIZE]
                        } else {
                            [1.0_f32 * TILE_SIZE, 4.0_f32 * TILE_SIZE]
                        };
                        [uv_base_px[0] / tex_w, uv_base_px[1] / tex_h]
                    }
                };

                self.draw_tile_textured(camera, px, py, [1.0, 1.0, 1.0, 1.0], uv_base, uv_scale, 0);
            }
        }
    }

    fn draw_base_dual_grid(
        &mut self,
        checker_fn: impl Fn(i32, i32) -> bool,
        camera: &Camera,
        tile_type_index: u8,
        opacity: f32,
    ) {
        // let width = state.map.base.first().map(|r| r.len()).unwrap_or(0);
        // let height = state.map.base.len();
        // if width == 0 || height == 0 {
        //     return;
        // }

        let tilemap = self.textures.get(&TextureIndexes::Tile).unwrap();
        let tex_w = tilemap.w;
        let tex_h = tilemap.h;

        // Apply half-tile offset: 0.5 left (negative X), 0.5 down (positive Y)
        let offset_x = 0.5 * TILE_SIZE;
        let offset_y = 0.5 * TILE_SIZE;

        // Compute visible world bounds from camera (expand slightly to avoid edge gaps)
        let zoom = camera.zoom;
        let half_w_world = camera.screen_w * 0.5 / zoom;
        let half_h_world = camera.screen_h * 0.5 / zoom;
        let world_min_x = camera.x * TILE_SIZE - half_w_world - TILE_SIZE;
        let world_min_y = camera.y * TILE_SIZE - half_h_world - TILE_SIZE;
        let world_max_x = camera.x * TILE_SIZE + half_w_world + TILE_SIZE;
        let world_max_y = camera.y * TILE_SIZE + half_h_world + TILE_SIZE;

        // Convert world bounds to dual-grid tile indices
        let start_x = ((world_min_x - offset_x) / TILE_SIZE).floor() as i32;
        let end_x = ((world_max_x - offset_x) / TILE_SIZE).ceil() as i32;
        let start_y = ((world_min_y - offset_y) / TILE_SIZE).floor() as i32;
        let end_y = ((world_max_y - offset_y) / TILE_SIZE).ceil() as i32;

        // Build a single batched mesh (one quad per visible dual-grid tile) and draw in one call
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        let mut base_index: u16 = 0;

        for y in start_y..end_y {
            for x in start_x..end_x {
                let tl = checker_fn(x, y);
                let tr = checker_fn(x + 1, y);
                let bl = checker_fn(x, y + 1);
                let br = checker_fn(x + 1, y + 1);

                let mut mask: u32 = 0;
                if tl {
                    mask |= 1;
                }
                if tr {
                    mask |= 2;
                }
                if bl {
                    mask |= 4;
                }
                if br {
                    mask |= 8;
                }

                if mask == 0 {
                    continue;
                }

                let (u, v) = DUAL_GRID_UV_TABLE[mask as usize];
                let uv_base_px = [u as f32 * TILE_SIZE, v as f32 * TILE_SIZE];
                // Inset UVs by half a texel to avoid sampling across tile boundaries
                let half_u = 0.5 / tex_w;
                let half_v = 0.5 / tex_h;
                let base_u = uv_base_px[0] / tex_w
                    + half_u
                    + tile_type_index as f32 * 4.0 * TILE_SIZE / tex_w;
                let base_v = uv_base_px[1] / tex_h + half_v;
                let du = (TILE_SIZE - 1.0) / tex_w;
                let dv = (TILE_SIZE - 1.0) / tex_h;

                let px = x as f32 * TILE_SIZE + offset_x;
                let py = y as f32 * TILE_SIZE + offset_y;

                // Quad vertices in world pixels and precomputed UVs
                vertices.push(Vertex {
                    pos: [px, py],
                    uv: [base_u, base_v],
                }); // top-left
                vertices.push(Vertex {
                    pos: [px + TILE_SIZE, py],
                    uv: [base_u + du, base_v],
                }); // top-right
                vertices.push(Vertex {
                    pos: [px + TILE_SIZE, py + TILE_SIZE],
                    uv: [base_u + du, base_v + dv],
                }); // bottom-right
                vertices.push(Vertex {
                    pos: [px, py + TILE_SIZE],
                    uv: [base_u, base_v + dv],
                }); // bottom-left

                indices.extend_from_slice(&[
                    base_index,
                    base_index + 1,
                    base_index + 2,
                    base_index,
                    base_index + 2,
                    base_index + 3,
                ]);
                base_index = base_index.wrapping_add(4);
            }
        }

        if vertices.is_empty() {
            return;
        }

        // Create transient buffers for this frame's batched draw
        let vb = self.ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );
        let ib = self.ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        // Bind textures
        let background = self.textures.get(&TextureIndexes::TileBackground).unwrap();
        let tile = self.textures.get(&TextureIndexes::Tile).unwrap();

        let batched_bindings = Bindings {
            vertex_buffers: vec![vb],
            index_buffer: ib,
            images: vec![tile.texture, background.texture],
        };

        // Switch to batched pipeline
        self.ctx.apply_pipeline(&self.pipeline_tiles);
        self.ctx.apply_bindings(&batched_bindings);

        // Build VP (no per-tile model matrix since positions are in world pixels)
        let view = Self::camera_view(camera);
        let proj = Self::ortho_mvp(camera);
        let vp = Self::mat4_mul(proj, view);

        let uniforms = Uniforms {
            mvp: vp,
            color: [1.0, 1.0, 1.0, opacity],
            uv_base: [0.0, 0.0, 0.0, 0.0],
            uv_scale: [1.0, 1.0, 0.0, 0.0],
            world_base: [0.0, 0.0, 0.0, 0.0],
            world_scale: [TILE_SIZE, TILE_SIZE, 0.0, 0.0],
            color_key: [1.0, 0.0, 1.0, 0.01],
            bg_tile_size: [64.0, 64.0, 0.0, 0.0],
            bg_region_origin: [64.0 * tile_type_index as f32, 0.0, 0.0, 0.0],
            bg_tex_size: [background.w, background.h, 0.0, 0.0],
        };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, indices.len() as i32, 1);

        // Restore default pipeline and bindings for subsequent draws
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
    }

    fn ortho_mvp(camera: &Camera) -> [f32; 16] {
        let l = 0.0;
        let r = camera.screen_w;
        let t = 0.0;
        let b = camera.screen_h;
        let n = -1.0;
        let f = 1.0;
        let sx = 2.0 / (r - l);
        let sy = 2.0 / (t - b);
        let sz = -2.0 / (f - n);
        let tx = -((r + l) / (r - l));
        let ty = -((t + b) / (t - b));
        let tz = -((f + n) / (f - n));
        [
            sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, sz, 0.0, tx, ty, tz, 1.0,
        ]
    }

    fn camera_view(camera: &Camera) -> [f32; 16] {
        let cx = camera.x * TILE_SIZE;
        let cy = camera.y * TILE_SIZE;
        let zoom = camera.zoom;

        // Pixel-snap the camera to avoid subpixel seams at various zoom levels
        let snapped_cx = (cx * zoom).round() / zoom;
        let snapped_cy = (cy * zoom).round() / zoom;

        // View should transform world so that camera center maps to screen center
        // Pipeline: translate (-snapped_cx, -snapped_cy) -> scale (zoom) -> translate (screen_w/2, screen_h/2)
        let translate_to_origin = Self::mat4_translation(-snapped_cx, -snapped_cy);
        let scale_zoom = Self::mat4_scale(zoom, zoom);
        let translate_to_screen_center =
            Self::mat4_translation(camera.screen_w * 0.5, camera.screen_h * 0.5);

        let ts = Self::mat4_mul(scale_zoom, translate_to_origin);
        Self::mat4_mul(translate_to_screen_center, ts)
    }

    fn mat4_rotation_z(angle_rad: f32) -> [f32; 16] {
        let c = angle_rad.cos();
        let s = angle_rad.sin();

        // Column-major 4x4 rotation around Z
        [
            c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    fn mat4_mul(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
        let mut out = [0.0f32; 16];
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += a[k * 4 + row] * b[col * 4 + k];
                }
                out[col * 4 + row] = sum;
            }
        }
        out
    }

    fn mat4_translation(tx: f32, ty: f32) -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, 0.0, 1.0,
        ]
    }

    fn mat4_scale(sx: f32, sy: f32) -> [f32; 16] {
        [
            sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }
}

const VERTEX_SHADER: &str = r#"#version 100
attribute vec2 pos;
attribute vec2 uv;
uniform mat4 mvp;
uniform vec4 color;
uniform vec4 uv_base;
uniform vec4 uv_scale;
uniform vec4 world_base;
uniform vec4 world_scale;
varying vec4 v_color;
varying vec2 v_uv;
varying vec2 v_world;
void main() {
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
    v_color = color;
    v_uv = uv_base.xy + uv * uv_scale.xy;
    v_world = world_base.xy + pos * world_scale.xy;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;
varying vec4 v_color;
varying vec2 v_uv;
uniform sampler2D tex;
uniform sampler2D bg_tex;
uniform vec4 color_key; // rgb + threshold in a
uniform vec4 bg_tile_size; // xy repeat period in pixels
uniform vec4 bg_region_origin; // xy top-left of the region in pixels
uniform vec4 bg_tex_size; // xy bg texture size in pixels
varying vec2 v_world;
void main() {
    vec4 texel = texture2D(tex, v_uv);
    float is_key = step(distance(texel.rgb, color_key.rgb), color_key.a);
    // Repeat inside the specified region, regardless of texture size
    vec2 region_uv = fract(v_world / bg_tile_size.xy);
    vec2 bg_px = bg_region_origin.xy + region_uv * bg_tile_size.xy;
    vec2 uv_bg = bg_px / bg_tex_size.xy;
    vec4 bg = texture2D(bg_tex, uv_bg);
    vec4 out_color = mix(texel, bg, is_key);
    vec4 final_color = out_color * v_color;

    gl_FragColor = final_color;
}
"#;

// Batched tile vertex shader: positions are already in world pixels; UVs are precomputed.
const VERTEX_SHADER_TILES_BATCHED: &str = r#"#version 100
attribute vec2 pos;
attribute vec2 uv;
uniform mat4 mvp;      // here this is VP = Projection * View
uniform vec4 color;
varying vec4 v_color;
varying vec2 v_uv;
varying vec2 v_world;
void main() {
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
    v_color = color;
    v_uv = uv;
    v_world = pos;
}
"#;
