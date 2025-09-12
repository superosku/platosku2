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

    // Player state
    player_x: f32,
    player_y: f32,
    player_size: f32,
    player_speed: f32,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
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
            // Start player near the top-left open area
            player_x: 32.0 * 2.0,
            player_y: 32.0 * 2.0,
            player_size: 24.0,
            player_speed: 3.0,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
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

    fn draw_rect(&mut self, px: f32, py: f32, w: f32, h: f32, color: [f32; 4]) {
        let ortho = self.ortho_mvp();
        let model = Stage::mat4_mul(Stage::mat4_translation(px, py), Stage::mat4_scale(w, h));
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
    fn update(&mut self) {
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        if self.move_left { dx -= self.player_speed; }
        if self.move_right { dx += self.player_speed; }
        if self.move_up { dy -= self.player_speed; }
        if self.move_down { dy += self.player_speed; }

        self.player_x = (self.player_x + dx).clamp(0.0, (self.screen_width - self.player_size).max(0.0));
        self.player_y = (self.player_y + dy).clamp(0.0, (self.screen_height - self.player_size).max(0.0));
    }

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

        // draw player on top
        let px = self.player_x;
        let py = self.player_y;
        let ps = self.player_size;
        self.draw_rect(px, py, ps, ps, [0.20, 1.0, 0.40, 1.0]);

        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::A | KeyCode::Left => self.move_left = true,
            KeyCode::D | KeyCode::Right => self.move_right = true,
            KeyCode::W | KeyCode::Up => self.move_up = true,
            KeyCode::S | KeyCode::Down => self.move_down = true,
            _ => {}
        }
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        match keycode {
            KeyCode::A | KeyCode::Left => self.move_left = false,
            KeyCode::D | KeyCode::Right => self.move_right = false,
            KeyCode::W | KeyCode::Up => self.move_up = false,
            KeyCode::S | KeyCode::Down => self.move_down = false,
            _ => {}
        }
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
