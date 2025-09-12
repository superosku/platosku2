use miniquad::*;

use crate::state::GameState;

#[repr(C)]
struct Uniforms {
    mvp: [f32; 16],
    color: [f32; 4],
}

#[repr(C)]
struct Vertex { pos: [f32; 2] }

pub struct Renderer {
    ctx: Box<Context>,
    pipeline: Pipeline,
    bindings: Bindings,
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut ctx = window::new_rendering_backend();

        // unit quad
        let vertices: [Vertex; 4] = [
            Vertex { pos: [0.0, 0.0] },
            Vertex { pos: [1.0, 0.0] },
            Vertex { pos: [1.0, 1.0] },
            Vertex { pos: [0.0, 1.0] },
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

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX_SHADER, fragment: FRAGMENT_SHADER },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("color", UniformType::Float4),
                        ],
                    },
                },
            )
            .expect("failed to compile shader");

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2)],
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

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![],
        };

        Renderer { ctx, pipeline, bindings }
    }

    pub fn resize(&mut self, _w: f32, _h: f32) {
        // Nothing to do yet
    }

    pub fn draw(&mut self, state: &GameState) {
        let clear = PassAction::Clear { color: Some((0.08, 0.09, 0.10, 1.0)), depth: Some(1.0), stencil: Some(0) };
        self.ctx.begin_default_pass(clear);
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);

        // draw base grid
        for y in 0..state.map.base.len() {
            for x in 0..state.map.base[y].len() {
                let tile = state.map.base[y][x];
                if tile == 0 { continue; }
                let color = Self::base_color(tile);
                self.draw_tile(state, x as i32, y as i32, color);
            }
        }

        // draw overlay grid on top
        for y in 0..state.map.overlay.len() {
            for x in 0..state.map.overlay[y].len() {
                let tile = state.map.overlay[y][x];
                if tile == 0 { continue; }
                let color = Self::overlay_color(tile);
                self.draw_tile(state, x as i32, y as i32, color);
            }
        }

        // draw player on top
        let px = state.player.x;
        let py = state.player.y;
        let ps = state.player.size;
        self.draw_rect(state, px, py, ps, ps, [0.20, 1.0, 0.40, 1.0]);

        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn draw_tile(&mut self, state: &GameState, x: i32, y: i32, color: [f32; 4]) {
        let tile_size = state.map.tile_size;
        let px = x as f32 * tile_size;
        let py = y as f32 * tile_size;

        let ortho = Self::ortho_mvp(state.screen_w, state.screen_h);
        let model = Self::mat4_mul(Self::mat4_translation(px, py), Self::mat4_scale(tile_size, tile_size));
        let mvp = Self::mat4_mul(ortho, model);

        let uniforms = Uniforms { mvp, color };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn draw_rect(&mut self, state: &GameState, px: f32, py: f32, w: f32, h: f32, color: [f32; 4]) {
        let ortho = Self::ortho_mvp(state.screen_w, state.screen_h);
        let model = Self::mat4_mul(Self::mat4_translation(px, py), Self::mat4_scale(w, h));
        let mvp = Self::mat4_mul(ortho, model);

        let uniforms = Uniforms { mvp, color };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
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
uniform mat4 mvp;
uniform vec4 color;
varying vec4 v_color;
void main() {
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
    v_color = color;
}
"#;

const FRAGMENT_SHADER: &str = r#"#version 100
precision mediump float;
varying vec4 v_color;
void main() {
    gl_FragColor = v_color;
}
"#;


