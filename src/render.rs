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
        let dyn_img = image::open("assets/tilemap.png").expect("failed to load assets/tilemap.png");
        let (img_w, img_h) = dyn_img.dimensions();
        let rgba8 = dyn_img.to_rgba8();
        let tile_texture = ctx.new_texture_from_rgba8((img_w as u16), (img_h as u16), &rgba8);
        // Use nearest filtering for crisp pixel art
        ctx.texture_set_filter(tile_texture, FilterMode::Nearest, MipmapFilterMode::None);

        // Create a 1x1 white texture for colored rectangles
        let white_tex_bytes: [u8; 4] = [255, 255, 255, 255];
        let white_texture = ctx.new_texture_from_rgba8(1, 1, &white_tex_bytes);
        ctx.texture_set_filter(white_texture, FilterMode::Nearest, MipmapFilterMode::None);

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

        // draw overlay grid on top (keep as simple colored rects)
        // for y in 0..state.map.overlay.len() {
        //     for x in 0..state.map.overlay[y].len() {
        //         let tile = state.map.overlay[y][x];
        //         if tile == 0 { continue; }
        //         let color = Self::overlay_color(tile);
        //         self.draw_rect(state, x as i32 as f32 * state.map.tile_size, y as i32 as f32 * state.map.tile_size, state.map.tile_size, state.map.tile_size, color);
        //     }
        // }

        // draw coins
        for coin in &state.coins {
            self.draw_rect(state, coin.x, coin.y, coin.size, coin.size, [1.0, 0.85, 0.2, 1.0]);
        }

        // draw player on top
        let px = state.player.x;
        let py = state.player.y;
        let ps = state.player.size;
        self.draw_rect(state, px, py, ps, ps, [0.20, 1.0, 0.40, 1.0]);

        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn draw_tile_textured(&mut self, state: &GameState, px: f32, py: f32, w: f32, h: f32, color: [f32; 4], uv_base: [f32; 2], uv_scale: [f32; 2]) {
        // ensure tile texture bound
        self.bindings.images[0] = self.tile_texture;
        self.ctx.apply_bindings(&self.bindings);

        let ortho = Self::ortho_mvp(state.screen_w, state.screen_h);
        let model = Self::mat4_mul(Self::mat4_translation(px, py), Self::mat4_scale(w, h));
        let mvp = Self::mat4_mul(ortho, model);

        let uniforms = Uniforms { mvp, color, uv_base: [uv_base[0], uv_base[1], 0.0, 0.0], uv_scale: [uv_scale[0], uv_scale[1], 0.0, 0.0] };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_rect(&mut self, state: &GameState, px: f32, py: f32, w: f32, h: f32, color: [f32; 4]) {
        // bind white texture and use full-quad UVs
        self.bindings.images[0] = self.white_texture;
        self.ctx.apply_bindings(&self.bindings);

        let ortho = Self::ortho_mvp(state.screen_w, state.screen_h);
        let model = Self::mat4_mul(Self::mat4_translation(px, py), Self::mat4_scale(w, h));
        let mvp = Self::mat4_mul(ortho, model);

        let uniforms = Uniforms { mvp, color, uv_base: [0.0, 0.0, 0.0, 0.0], uv_scale: [1.0, 1.0, 0.0, 0.0] };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_base_dual_grid(&mut self, state: &GameState) {
        let tile_world = state.map.tile_size;
        let width = state.map.base.first().map(|r| r.len()).unwrap_or(0);
        let height = state.map.base.len();
        if width == 0 || height == 0 { return; }

        let tile_px: f32 = 24.0; // source tile size in pixels inside the atlas

        // Placeholder 4x4 mapping: mask -> (u, v)
        // let mut uv_table: [(u32, u32); 16] = [(0, 0); 16];
        // for m in 0..16u32 { uv_table[m as usize] = (m % 4, m / 4); }
        // Real uv table
        let uv_table: [(u32, u32); 16] = [
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

        let tex_w = self.tilemap_w;
        let tex_h = self.tilemap_h;

        // Helper closure to read base value with bounds check
        let mut base_at = |x: i32, y: i32| -> u8 {
            if x < 0 || y < 0 { return 0; }
            let ux = x as usize; let uy = y as usize;
            if uy >= height || ux >= width { return 0; }
            state.map.base[uy][ux]
        };

        // Apply half-tile offset: 0.5 left (negative X), 0.5 down (positive Y)
        let offset_x = 0.5 * tile_world;
        let offset_y = 0.5 * tile_world;

        for y in 0..(height.saturating_sub(1)) {
            for x in 0..(width.saturating_sub(1)) {
                let tl = base_at(x as i32, y as i32) != 0;
                let tr = base_at(x as i32 + 1, y as i32) != 0;
                let bl = base_at(x as i32, y as i32 + 1) != 0;
                let br = base_at(x as i32 + 1, y as i32 + 1) != 0;

                let mut mask: u32 = 0;
                if tl { mask |= 1; }
                if tr { mask |= 2; }
                if bl { mask |= 4; }
                if br { mask |= 8; }

                if mask == 0 { continue; }

                let (u, v) = uv_table[mask as usize];
                let uv_base_px = [u as f32 * tile_px, v as f32 * tile_px];
                let uv_base = [uv_base_px[0] / tex_w, uv_base_px[1] / tex_h];
                let uv_scale = [tile_px / tex_w, tile_px / tex_h];

                let px = x as f32 * tile_world + offset_x;
                let py = y as f32 * tile_world + offset_y;

                self.draw_tile_textured(state, px, py, tile_world, tile_world, [1.0, 1.0, 1.0, 1.0], uv_base, uv_scale);
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


