//! Plays an input script with real rendering and saves a PNG screenshot at every
//! `snap` marker (and optionally every N ticks).
//!
//!     cargo run --example capture -- scripts/jump_demo.txt [--out DIR] [--every N]
//!
//! The game runs one tick per rendered frame (no wall-clock time), so the same
//! script always gives the same screenshots. A window opens briefly; the game is
//! rendered into an offscreen 800x600 texture. The egui debug window is not
//! included. Default output: target/claude/shots/<script name>/.

use miniquad::*;
use platosku2::camera::Camera;
use platosku2::render::Renderer;
use platosku2::script::{Script, ScriptEvent, ScriptPlayer};
use platosku2::sim::Sim;
use platosku2::state::GameState;
use std::path::{Path, PathBuf};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
/// Ticks simulated and rendered per window frame. Higher is faster.
const TICKS_PER_FRAME: u32 = 8;

struct Args {
    script_path: String,
    out_dir: PathBuf,
    clean_out_dir: bool,
    every: Option<u64>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args().skip(1);
    let mut script_path = None;
    let mut out_dir = None;
    let mut every = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out_dir = Some(PathBuf::from(args.next().ok_or("--out needs a directory")?)),
            "--every" => {
                let n = args.next().ok_or("--every needs a number")?;
                let n: u64 = n.parse().map_err(|_| format!("bad --every value {n:?}"))?;
                if n == 0 {
                    return Err("--every must be at least 1".into());
                }
                every = Some(n);
            }
            _ if script_path.is_none() && !arg.starts_with("--") => script_path = Some(arg),
            _ => return Err(format!("unexpected argument {arg:?}")),
        }
    }
    let script_path = script_path.ok_or("missing script path")?;
    let clean_out_dir = out_dir.is_none();
    let out_dir = out_dir.unwrap_or_else(|| {
        let stem = Path::new(&script_path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "script".into());
        PathBuf::from("target/claude/shots").join(stem)
    });
    Ok(Args {
        script_path,
        out_dir,
        clean_out_dir,
        every,
    })
}

struct Capture {
    sim: Sim,
    player: ScriptPlayer,
    renderer: Renderer,
    camera: Camera,
    target: TextureId,
    out_dir: PathBuf,
    every: Option<u64>,
    shot_count: u32,
    done: bool,
    camera_settled: bool,
}

impl Capture {
    fn render(&mut self) {
        // show_dark and the small minimap, like the default game view.
        self.renderer
            .draw(&mut self.sim.game, &self.camera, true, false);
    }

    fn save(&mut self, name: &str) {
        let mut pixels = self
            .renderer
            .read_capture_pixels(self.target, WIDTH, HEIGHT);
        // The window ignores alpha, but blending can leave it below 255 in the texture.
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        let path = self.out_dir.join(format!(
            "{:03}_{}_t{}.png",
            self.shot_count,
            name,
            self.sim.tick()
        ));
        image::save_buffer(&path, &pixels, WIDTH, HEIGHT, image::ColorType::Rgba8)
            .unwrap_or_else(|e| panic!("saving {}: {e}", path.display()));
        println!("{}", path.display());
        self.shot_count += 1;
    }
}

impl EventHandler for Capture {
    fn update(&mut self) {}

    fn draw(&mut self) {
        for _ in 0..TICKS_PER_FRAME {
            if self.done {
                break;
            }
            match self.player.advance(&mut self.sim) {
                ScriptEvent::Ticked => {
                    // Same order as the game loop: update, then camera, then draw.
                    if !self.camera_settled {
                        // The camera eases towards the current room, which is only
                        // known after the first tick. Settle it right away so the
                        // screenshots don't show the camera flying in. This does
                        // not touch the game state.
                        for _ in 0..300 {
                            self.sim.game.update_camera(&mut self.camera, false);
                        }
                        self.camera_settled = true;
                    }
                    self.sim.game.update_camera(&mut self.camera, false);
                    self.render();
                    if let Some(every) = self.every
                        && self.sim.tick().is_multiple_of(every)
                    {
                        self.save("every");
                    }
                }
                ScriptEvent::Snap(name) => {
                    self.render();
                    self.save(&name);
                }
                ScriptEvent::Done => {
                    self.done = true;
                    window::order_quit();
                }
            }
        }

        // Keep the window itself black; everything interesting is offscreen.
        self.renderer
            .ctx
            .begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.ctx.end_render_pass();
        self.renderer.ctx.commit_frame();
    }
}

fn main() {
    let args = parse_args().unwrap_or_else(|e| {
        eprintln!(
            "{e}\nusage: cargo run --example capture -- <script.txt> [--out DIR] [--every N]"
        );
        std::process::exit(2);
    });
    let script = Script::load(&args.script_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    if args.clean_out_dir && args.out_dir.exists() {
        for entry in std::fs::read_dir(&args.out_dir).unwrap().flatten() {
            if entry.path().extension().is_some_and(|e| e == "png") {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    std::fs::create_dir_all(&args.out_dir).unwrap();

    miniquad::start(
        conf::Conf {
            window_title: String::from("platosku2 capture"),
            high_dpi: false,
            window_width: WIDTH as i32,
            window_height: HEIGHT as i32,
            ..Default::default()
        },
        move || {
            let game = script.build_game().unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let mut renderer = Renderer::new();
            let (pass, target) = renderer.new_capture_target(WIDTH, HEIGHT);
            renderer.set_render_target(Some(pass));

            let sim = Sim::new(game);
            let camera = Camera::new(0.0, 0.0, 2.0, WIDTH as f32, HEIGHT as f32);

            Box::new(Capture {
                player: ScriptPlayer::new(&script),
                sim,
                renderer,
                camera,
                target,
                out_dir: args.out_dir,
                every: args.every,
                shot_count: 0,
                done: false,
                camera_settled: false,
            })
        },
    );
}
