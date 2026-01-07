use miniquad::*;

mod camera;
mod physics;
mod state;
use crate::state::{GameState, InputState};
mod debug_menu;
mod render;

use crate::camera::Camera;
use crate::debug_menu::GameStateDebugMenu;
use crate::render::{DrawableGameState, Renderer};
use crate::state::game_map::Room;
use crate::state::game_state::{Editor, Game};
use egui_miniquad as egui_mq;

#[derive(Debug, Eq, PartialEq)]
enum TileSelection {
    NotPartOf,
    Clear,
    Stone,
    Wood,
    Ladder,
}

#[derive(Debug, Eq, PartialEq)]
enum EnemySelection {
    Remove,
    Bat,
    Slime,
}

#[derive(Debug, Eq, PartialEq)]
enum DoorSelection {
    Left,
    Right,
    Up,
    Down,
    Remove,
}

enum EditorSelection {
    Tiles { selection: TileSelection },
    Enemies { selection: EnemySelection },
    PlayerPos,
    Doors { selection: DoorSelection },
}

struct DebugMenu {
    editor_selection: EditorSelection,
    all_rooms: Vec<(String, Room)>,
    current_editor_room_index: u32,
    is_game: bool,
    zoom_show_full: bool,
    show_dark: bool,
}

impl DebugMenu {
    pub fn new() -> DebugMenu {
        let all_rooms = Room::load_rooms_from_folder();

        DebugMenu {
            editor_selection: EditorSelection::Tiles {
                selection: TileSelection::Clear,
            },
            all_rooms,
            current_editor_room_index: 0,
            is_game: true,
            zoom_show_full: true,
            show_dark: false,
        }
    }
}

trait FullGameState: GameState + DrawableGameState + GameStateDebugMenu {}
impl<T: GameState + DrawableGameState + GameStateDebugMenu> FullGameState for T {}

struct Stage {
    egui_mq: egui_mq::EguiMq,

    input: InputState,
    state: Box<dyn FullGameState>,
    renderer: Renderer,
    camera: Camera,

    last_time: f64,
    last_time_ups: f64,
    updates: u32,
    frames: u32,
    accumulator: f64,
    time_spent_drawing: f64,
    time_spent_updating: f64,

    mouse_pressed: bool,

    debug_menu: DebugMenu,
}

impl Stage {
    fn new(width: i32, height: i32) -> Stage {
        let mut renderer = Renderer::new();
        let state = Box::new(Game::new());

        Stage {
            egui_mq: egui_mq::EguiMq::new(&mut *renderer.ctx),
            state,
            renderer,
            last_time: date::now(),
            updates: 0,
            frames: 0,
            accumulator: 0.0,
            last_time_ups: date::now(),
            time_spent_drawing: 0.0,
            time_spent_updating: 0.0,
            mouse_pressed: false,
            debug_menu: DebugMenu::new(),
            input: InputState::default(),
            camera: Camera::new(0.0, 0.0, 2.0, width as f32, height as f32),
        }
    }

    fn handle_editor_tile_drawing(&mut self, x: f32, y: f32) {
        let coords = self.camera.screen_to_tile(x, y);
        self.state.mouse_drawing(coords, &self.debug_menu);
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        let update_start = date::now();
        let mut frame_time = update_start - self.last_time;
        self.last_time = update_start;

        if frame_time > 1.0 / 10.0 {
            frame_time = 1.0 / 10.0;
        }

        self.accumulator += frame_time;

        let dt = 1.0 / 60.0;

        while self.accumulator >= dt {
            self.state.update(&self.input); // HERE is the actual game call
            self.state
                .update_camera(&mut self.camera, self.debug_menu.zoom_show_full); // HERE is the actual game call
            self.input.jump = false;
            self.updates += 1;
            self.accumulator -= dt;
        }

        let elapsed = update_start - self.last_time_ups;
        let update_total = date::now() - update_start;
        self.time_spent_updating += update_total;

        if elapsed >= 1.0 {
            let fps = self.frames as f64 / elapsed;
            let ups = self.updates as f64 / elapsed;
            let ratio_of_time_updating = self.time_spent_updating / elapsed;
            let ratio_of_time_drawing = self.time_spent_drawing / elapsed;
            self.time_spent_updating = 0.0;
            self.time_spent_drawing = 0.0;
            println!(
                "FPS: {:.2}, UPS: {:.2}, updat: {:.4} draw: {:.4}",
                fps, ups, ratio_of_time_updating, ratio_of_time_drawing
            );
            self.frames = 0;
            self.updates = 0;
            self.last_time_ups = update_start;
        }
    }

    fn draw(&mut self) {
        // Game
        let draw_start = date::now();

        self.renderer
            .draw(self.state.as_ref(), &self.camera, self.debug_menu.show_dark);
        self.frames += 1;
        let draw_total = date::now() - draw_start;
        self.time_spent_drawing += draw_total;

        // GUI
        self.egui_mq
            .run(&mut *self.renderer.ctx, |_mq_ctx, egui_ctx| {
                egui::Window::new("Debug").show(egui_ctx, |ui| {
                    let previous_selection = self.debug_menu.is_game;
                    egui::ComboBox::from_id_salt("Select one!")
                        .selected_text(if self.debug_menu.is_game {
                            "Game"
                        } else {
                            "Editor"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.debug_menu.is_game, false, "Editor");
                            ui.selectable_value(&mut self.debug_menu.is_game, true, "Game");
                        });
                    if previous_selection != self.debug_menu.is_game {
                        if self.debug_menu.is_game {
                            self.state = Box::new(Game::new());
                        } else {
                            let mut editor = Editor::new();
                            editor.room = self.debug_menu.all_rooms[0].1.clone();
                            self.state = Box::new(editor);
                            self.debug_menu.current_editor_room_index = 0;
                            self.debug_menu.editor_selection = EditorSelection::Tiles {
                                selection: TileSelection::Stone,
                            }
                        }
                    }

                    self.state.render_ui(ui, &mut self.debug_menu)
                });
            });

        self.egui_mq.draw(&mut *self.renderer.ctx);

        self.renderer.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.camera.on_resize(width, height);
        self.renderer.resize(width, height);
    }

    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::Left => self.input.left = true,
            KeyCode::Right => self.input.right = true,
            KeyCode::Up => self.input.up = true,
            KeyCode::X => self.input.swing = true,
            KeyCode::Z => self.input.jump = true,
            KeyCode::Down => self.input.down = true,
            _ => {}
        }
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        match keycode {
            KeyCode::Left => self.input.left = false,
            KeyCode::Right => self.input.right = false,
            KeyCode::Up => self.input.up = false,
            KeyCode::X => self.input.swing = false,
            KeyCode::Z => self.input.jump = false,
            KeyCode::Down => self.input.down = false,
            _ => {}
        }
        self.egui_mq.key_up_event(keycode, keymods);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.camera.zoom_scroll(dy);
        self.egui_mq.mouse_wheel_event(dx, dy);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.egui_mq.mouse_motion_event(x, y);

        if self.egui_mq.egui_ctx().wants_pointer_input() {
            return;
        }

        if self.mouse_pressed {
            self.handle_editor_tile_drawing(x, y);
        }
    }

    fn mouse_button_down_event(&mut self, mb: MouseButton, x: f32, y: f32) {
        self.mouse_pressed = true;

        self.egui_mq.mouse_button_down_event(mb, x, y);

        if self.egui_mq.egui_ctx().wants_pointer_input() {
            return;
        }

        let coords = self.camera.screen_to_tile(x, y);

        self.handle_editor_tile_drawing(x, y);

        self.state.mouse_button_event(coords, &mut self.debug_menu)
    }

    fn mouse_button_up_event(&mut self, mb: MouseButton, x: f32, y: f32) {
        self.mouse_pressed = false;

        self.egui_mq.mouse_button_up_event(mb, x, y);
    }

    fn char_event(&mut self, character: char, _keymods: KeyMods, _repeat: bool) {
        self.egui_mq.char_event(character);
    }
}

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
