use miniquad::*;

struct Stage {
    ctx: Box<Context>,
    pipeline: Pipeline,
    bindings: Bindings,
    screen_width: f32,
    screen_height: f32,

    tile_size: f32,
    base_grid: Vec<Vec<u8>>,    // base terrain layer
    overlay_grid: Vec<Vec<u8>>, // overlay/decorations layer
}

impl Stage {
    fn new(width: i32, height: i32) -> Stage {
        // Simple unit quad at origin (0..1, 0..1)
        #[repr(C)]
        struct Vertex { pos: [f32; 2] }
        let vertices: [Vertex; 4] = [
            Vertex { pos: [0.0, 0.0] },
            Vertex { pos: [1.0, 0.0] },
            Vertex { pos: [1.0, 1.0] },
            Vertex { pos: [0.0, 1.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let mut ctx = window::new_rendering_backend();

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

        // Small demo tilemaps (dual-grid): base terrain and overlay
        let base_grid = vec![
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        ];

        let mut overlay_grid = vec![vec![0u8; 16]; 8];
        // simple decorations in overlay
        overlay_grid[2][3] = 2;
        overlay_grid[2][4] = 2;
        overlay_grid[2][5] = 2;
        overlay_grid[4][8] = 3;

        Stage {
            ctx,
            pipeline,
            bindings,
            screen_width: width as f32,
            screen_height: height as f32,
            tile_size: 32.0,
            base_grid,
            overlay_grid,
        }
    }

    fn ortho_mvp(&self) -> [f32; 16] {
        // Build an orthographic projection with origin at top-left, y down
        let l = 0.0;
        let r = self.screen_width;
        let t = 0.0;
        let b = self.screen_height;
        let n = -1.0;
        let f = 1.0;
        let sx = 2.0 / (r - l);
        let sy = 2.0 / (t - b); // negative to flip Y downwards
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
        // Column-major multiplication: out = a * b
        // Indexing: m[col*4 + row]
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

    fn draw_tile(&mut self, x: i32, y: i32, color: [f32; 4]) {
        let px = x as f32 * self.tile_size;
        let py = y as f32 * self.tile_size;

        let ortho = self.ortho_mvp();
        let model = Stage::mat4_mul(Stage::mat4_translation(px, py), Stage::mat4_scale(self.tile_size, self.tile_size));
        let mvp = Stage::mat4_mul(ortho, model);

        let uniforms = Uniforms { mvp, color };
        self.ctx.apply_uniforms(UniformsSource::table(&uniforms));
        self.ctx.draw(0, 6, 1);
    }

    fn base_color(tile: u8) -> [f32; 4] {
        match tile {
            1 => [0.35, 0.25, 0.15, 1.0], // walls/ground - brown
            2 => [0.40, 0.40, 0.45, 1.0], // stone
            _ => [0.0, 0.0, 0.0, 0.0],    // empty
        }
    }

    fn overlay_color(tile: u8) -> [f32; 4] {
        match tile {
            1 => [0.20, 0.60, 1.0, 0.8], // water-like
            2 => [1.0, 0.85, 0.2, 0.9],  // coin-like
            3 => [1.0, 0.2, 0.2, 0.9],   // hazard
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    }
}

#[repr(C)]
struct Uniforms {
    mvp: [f32; 16],
    color: [f32; 4],
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let clear = PassAction::Clear { color: Some((0.08, 0.09, 0.10, 1.0)), depth: Some(1.0), stencil: Some(0) };
        self.ctx.begin_default_pass(clear);
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);

        // draw base grid
        for y in 0..self.base_grid.len() {
            for x in 0..self.base_grid[y].len() {
                let tile = self.base_grid[y][x];
                if tile == 0 { continue; }
                let color = Self::base_color(tile);
                self.draw_tile(x as i32, y as i32, color);
            }
        }

        // draw overlay grid on top
        for y in 0..self.overlay_grid.len() {
            for x in 0..self.overlay_grid[y].len() {
                let tile = self.overlay_grid[y][x];
                if tile == 0 { continue; }
                let color = Self::overlay_color(tile);
                self.draw_tile(x as i32, y as i32, color);
            }
        }

        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
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

fn main() {
    miniquad::start(
        conf::Conf {
            window_title: String::from("Miniquad Dual-Grid Tilemap"),
            high_dpi: false,
            window_width: 800,
            window_height: 600,
            ..Default::default()
        },
        || {
            let (w, h) = window::screen_size();
            Box::new(Stage::new(w as i32, h as i32))
        },
    );
}
