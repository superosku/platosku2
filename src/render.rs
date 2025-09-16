use miniquad::*;
use image::GenericImageView;

use crate::state::GameState;

#[repr(C)]
struct Uniforms {
    mvp: [f32; 16],
    color: [f32; 4],
    uv_base: [f32; 4],  // xy used
    uv_scale: [f32; 4], // xy used
}

#[repr(C)]
struct Vertex { pos: [f32; 2], uv: [f32; 2] }

pub struct Renderer {
    ctx: Box<Context>,
    pipeline: Pipeline,
    bindings: Bindings,
    tile_texture: TextureId,
    white_texture: TextureId,
    tilemap_w: f32,
    tilemap_h: f32,
}

const TILE_SIZE: f32 = 32.0;

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

impl Renderer {
    pub fn new() -> Renderer {
        let mut ctx = window::new_rendering_backend();

        // unit quad with UVs (0..1)
        let vertices: [Vertex; 4] = [
            Vertex { pos: [0.0, 0.0], uv: [0.0, 0.0] },
            Vertex { pos: [1.0, 0.0], uv: [1.0, 0.0] },
            Vertex { pos: [1.0, 1.0], uv: [1.0, 1.0] },
            Vertex { pos: [0.0, 1.0], uv: [0.0, 1.0] },
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

        // Load tilemap texture from assets
        // let dyn_img = image::open("assets/tilemap.png").expect("failed to load assets/tilemap.png");
        let dyn_img = image::open("assets/tilemap16.png").expect("failed to load assets/tilemap.png");
        let (img_w, img_h) = dyn_img.dimensions();
        let rgba8 = dyn_img.to_rgba8();
        let tile_texture = ctx.new_texture_from_rgba8(img_w as u16, img_h as u16, &rgba8);
        // Use nearest filtering for crisp pixel art
        ctx.texture_set_filter(tile_texture, FilterMode::Nearest, MipmapFilterMode::None);
        // Clamp to edge to avoid atlas bleeding at tile borders
        ctx.texture_set_wrap(tile_texture, TextureWrap::Clamp, TextureWrap::Clamp);

        // Create a 1x1 white texture for colored rectangles
        let white_tex_bytes: [u8; 4] = [255, 255, 255, 255];
        let white_texture = ctx.new_texture_from_rgba8(1, 1, &white_tex_bytes);
        ctx.texture_set_filter(white_texture, FilterMode::Nearest, MipmapFilterMode::None);
        ctx.texture_set_wrap(white_texture, TextureWrap::Clamp, TextureWrap::Clamp);

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: FRAGMENT_SHADER },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("color", UniformType::Float4),
                            UniformDesc::new("uv_base", UniformType::Float4),
                            UniformDesc::new("uv_scale", UniformType::Float4),
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
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                cull_face: CullFace::Nothing,
                ..Default::default()
            },
        );

        let mut bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![tile_texture],
        };

        // set default texture to tile texture
        bindings.images[0] = tile_texture;

        Renderer {
            ctx,
            pipeline,
            bindings,
            tile_texture,
            white_texture,
            tilemap_w: img_w as f32,
            tilemap_h: img_h as f32,
        }
    }

    pub fn resize(&mut self, _w: f32, _h: f32) {
        // Nothing to do yet
    }

    pub fn draw(&mut self, state: &GameState) {
        let clear = PassAction::Clear { color: Some((0.08, 0.09, 0.10, 1.0)), depth: Some(1.0), stencil: Some(0) };
        self.ctx.begin_default_pass(clear);
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);

        // draw base grid using dual-grid textured tiles
        self.draw_base_dual_grid(state);

        // draw coins
        for coin in &state.coins {
            self.draw_rect(state, coin.bb.x, coin.bb.y, coin.bb.w, coin.bb.h, [1.0, 0.85, 0.2, 1.0]);
        }

        // draw player on top
        let px = state.player.bb.x;
        let py = state.player.bb.y;
        let pw = state.player.bb.w;
        let ph = state.player.bb.h;
        self.draw_rect(state, px, py, pw, ph, [0.20, 1.0, 0.40, 1.0]);

        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn draw_tile_textured(&mut self, state: &GameState, px: f32, py: f32, color: [f32; 4], uv_base: [f32; 2], uv_scale: [f32; 2]) {
        // ensure tile texture bound
        self.bindings.images[0] = self.tile_texture;
        self.ctx.apply_bindings(&self.bindings);

        let view = Self::camera_view(state);
        let proj = Self::ortho_mvp(state.screen_w, state.screen_h);
        let model = Self::mat4_mul(Self::mat4_translation(px, py), Self::mat4_scale(TILE_SIZE, TILE_SIZE));
        let vp = Self::mat4_mul(proj, view);
        let mvp = Self::mat4_mul(vp, model);

        let uniforms = Uniforms { mvp, color, uv_base: [uv_base[0], uv_base[1], 0.0, 0.0], uv_scale: [uv_scale[0], uv_scale[1], 0.0, 0.0] };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_rect(&mut self, state: &GameState, px: f32, py: f32, w: f32, h: f32, color: [f32; 4]) {
        // bind white texture and use full-quad UVs
        self.bindings.images[0] = self.white_texture;
        self.ctx.apply_bindings(&self.bindings);

        let view = Self::camera_view(state);
        let proj = Self::ortho_mvp(state.screen_w, state.screen_h);
        let model = Self::mat4_mul(Self::mat4_translation(px * TILE_SIZE, py * TILE_SIZE), Self::mat4_scale(w * TILE_SIZE, h * TILE_SIZE));
        let vp = Self::mat4_mul(proj, view);
        let mvp = Self::mat4_mul(vp, model);

        let uniforms = Uniforms { mvp, color, uv_base: [0.0, 0.0, 0.0, 0.0], uv_scale: [1.0, 1.0, 0.0, 0.0] };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_base_dual_grid(&mut self, state: &GameState) {
        let width = state.map.base.first().map(|r| r.len()).unwrap_or(0);
        let height = state.map.base.len();
        if width == 0 || height == 0 { return; }

        let tile_px: f32 = 16.0; // source tile size in pixels inside the atlas

        let tex_w = self.tilemap_w;
        let tex_h = self.tilemap_h;

        // Apply half-tile offset: 0.5 left (negative X), 0.5 down (positive Y)
        let offset_x = 0.5 * TILE_SIZE;
        let offset_y = 0.5 * TILE_SIZE;

        // Compute visible world bounds from camera (expand slightly to avoid edge gaps)
        let zoom = state.camera.zoom;
        let half_w_world = state.screen_w * 0.5 / zoom;
        let half_h_world = state.screen_h * 0.5 / zoom;
        let world_min_x = state.camera.x * TILE_SIZE - half_w_world - TILE_SIZE;
        let world_min_y = state.camera.y * TILE_SIZE - half_h_world - TILE_SIZE;
        let world_max_x = state.camera.x * TILE_SIZE + half_w_world + TILE_SIZE;
        let world_max_y = state.camera.y * TILE_SIZE + half_h_world + TILE_SIZE;

        // Convert world bounds to dual-grid tile indices
        let start_x = ((world_min_x - offset_x) / TILE_SIZE).floor() as i32;
        let end_x = ((world_max_x - offset_x) / TILE_SIZE).ceil() as i32;
        let start_y = ((world_min_y - offset_y) / TILE_SIZE).floor() as i32;
        let end_y = ((world_max_y - offset_y) / TILE_SIZE).ceil() as i32;

        for y in start_y..end_y {
            for x in start_x..end_x {
                let (tl, _o1) = state.map.get_at(x, y);
                let (tr, _o2) = state.map.get_at(x + 1, y);
                let (bl, _o3) = state.map.get_at(x, y + 1);
                let (br, _o4) = state.map.get_at(x + 1, y + 1);

                let mut mask: u32 = 0;
                if tl != 0 { mask |= 1; }
                if tr != 0 { mask |= 2; }
                if bl != 0 { mask |= 4; }
                if br != 0 { mask |= 8; }

                if mask == 0 { continue; }

                let (u, v) = DUAL_GRID_UV_TABLE[mask as usize];
                let uv_base_px = [u as f32 * tile_px, v as f32 * tile_px];
                // Inset UVs by half a texel to avoid sampling across tile boundaries
                let half_u = 0.5 / tex_w;
                let half_v = 0.5 / tex_h;
                let uv_base = [uv_base_px[0] / tex_w + half_u, uv_base_px[1] / tex_h + half_v];
                let uv_scale = [(tile_px - 1.0) / tex_w, (tile_px - 1.0) / tex_h];

                let px = x as f32 * TILE_SIZE + offset_x;
                let py = y as f32 * TILE_SIZE + offset_y;

                self.draw_tile_textured(state, px, py, [1.0, 1.0, 1.0, 1.0], uv_base, uv_scale);
            }
        }
    }

    fn base_color(tile: u8) -> [f32; 4] {
        match tile {
            1 => [0.35, 0.25, 0.15, 1.0],
            2 => [0.40, 0.40, 0.45, 1.0],
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    }

    fn overlay_color(tile: u8) -> [f32; 4] {
        match tile {
            1 => [0.20, 0.60, 1.0, 0.8],
            2 => [1.0, 0.85, 0.2, 0.9],
            3 => [1.0, 0.2, 0.2, 0.9],
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    }

    fn ortho_mvp(screen_w: f32, screen_h: f32) -> [f32; 16] {
        let l = 0.0;
        let r = screen_w;
        let t = 0.0;
        let b = screen_h;
        let n = -1.0;
        let f = 1.0;
        let sx = 2.0 / (r - l);
        let sy = 2.0 / (t - b);
        let sz = -2.0 / (f - n);
        let tx = -((r + l) / (r - l));
        let ty = -((t + b) / (t - b));
        let tz = -((f + n) / (f - n));
        [
            sx, 0.0, 0.0, 0.0,
            0.0, sy, 0.0, 0.0,
            0.0, 0.0, sz, 0.0,
            tx, ty, tz, 1.0,
        ]
    }

    fn camera_view(state: &GameState) -> [f32; 16] {
        let cx = state.camera.x * TILE_SIZE;
        let cy = state.camera.y * TILE_SIZE;
        let zoom = state.camera.zoom;

        // Pixel-snap the camera to avoid subpixel seams at various zoom levels
        let snapped_cx = (cx * zoom).round() / zoom;
        let snapped_cy = (cy * zoom).round() / zoom;

        // View should transform world so that camera center maps to screen center
        // Pipeline: translate (-snapped_cx, -snapped_cy) -> scale (zoom) -> translate (screen_w/2, screen_h/2)
        let translate_to_origin = Self::mat4_translation(-snapped_cx, -snapped_cy);
        let scale_zoom = Self::mat4_scale(zoom, zoom);
        let translate_to_screen_center = Self::mat4_translation(state.screen_w * 0.5, state.screen_h * 0.5);

        let ts = Self::mat4_mul(scale_zoom, translate_to_origin);
        Self::mat4_mul(translate_to_screen_center, ts)
    }

    fn mat4_mul(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
        let mut out = [0.0f32; 16];
        for row in 0..4 {
            for col in 0..4 {
                let mut sum = 0.0;
                for k in 0..4 { sum += a[k * 4 + row] * b[col * 4 + k]; }
                out[col * 4 + row] = sum;
            }
        }
        out
    }

    fn mat4_translation(tx: f32, ty: f32) -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            tx,  ty,  0.0, 1.0,
        ]
    }

    fn mat4_scale(sx: f32, sy: f32) -> [f32; 16] {
        [
            sx,  0.0, 0.0, 0.0,
            0.0, sy,  0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
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
varying vec4 v_color;
varying vec2 v_uv;
void main() {
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
    v_color = color;
    v_uv = uv_base.xy + uv * uv_scale.xy;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;
varying vec4 v_color;
varying vec2 v_uv;
uniform sampler2D tex;
void main() {
    vec4 texel = texture2D(tex, v_uv);
    gl_FragColor = texel * v_color;
}
"#;


