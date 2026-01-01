use miniquad::*;
use state::OverlayTile;
use std::fs;
use std::path::Path;

mod camera;
mod physics;
mod state;
use crate::state::game_map::{DoorDir, MapLike};
use crate::state::{BaseTile, Bat, Coin, Enemy, GameMap, GameState, InputState, Player};
mod render;
use crate::render::Renderer;
use crate::state::enemies::Slime;
use crate::state::game_map::Room;
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

struct UiConfig {
    editor_selection: EditorSelection,
}

impl UiConfig {
    pub fn new() -> UiConfig {
        UiConfig {
            editor_selection: EditorSelection::Tiles {
                selection: TileSelection::Clear,
            },
        }
    }
}

struct Stage {
    egui_mq: egui_mq::EguiMq,

    state: GameState,
    renderer: Renderer,

    last_time: f64,
    last_time_ups: f64,
    updates: u32,
    frames: u32,
    accumulator: f64,
    time_spent_drawing: f64,
    time_spent_updating: f64,

    ui_config: UiConfig,

    mouse_pressed: bool,
    all_rooms: Vec<(String, Room)>,
    current_editor_room_index: u32,
}

impl Stage {
    fn new(width: i32, height: i32) -> Stage {
        // Simple unit quad at origin (0..1, 0..1)
        let mut renderer = Renderer::new();

        let all_rooms = Room::load_rooms_from_folder();
        let map = all_rooms[0].1.clone();

        // Start player near the top-left open area
        let player = Player::new(2.0, 2.0);

        let mut state = GameState {
            screen_w: width as f32,
            screen_h: height as f32,
            player,
            map,
            input: InputState::default(),
            coins: vec![
                Coin::new(4.0, 1.0),
                Coin::new(6.0, 1.5),
                Coin::new(10.0, 1.0),
            ],
            enemies: vec![
                Box::new(Bat::new(8.0, 2.0)) as Box<dyn Enemy>,
                Box::new(Bat::new(12.0, 2.0)) as Box<dyn Enemy>,
                Box::new(Bat::new(4.0, 2.5)) as Box<dyn Enemy>,
                Box::new(Slime::new(5.0, 5.5)) as Box<dyn Enemy>,
                Box::new(Slime::new(9.0, 4.0)) as Box<dyn Enemy>,
                Box::new(Slime::new(10.0, 4.0)) as Box<dyn Enemy>,
            ],
            camera: camera::Camera::new(0.0, 0.0, 2.0),
        };

        // Initialize camera to player center
        let pcx = state.player.bb.x + state.player.bb.w * 0.5;
        let pcy = state.player.bb.y + state.player.bb.h * 0.5;
        state.camera.follow(pcx, pcy);

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
            ui_config: UiConfig::new(),
            mouse_pressed: false,
            all_rooms,
            current_editor_room_index: 0,
        }
    }

    fn handle_editor_tile_drawing(&mut self, x: f32, y: f32) {
        let coords =
            self.state
                .camera
                .screen_to_tile(x, y, self.state.screen_w, self.state.screen_h);
        println!("Mouse coords: {:?}", coords);

        match &self.ui_config.editor_selection {
            EditorSelection::Tiles { selection } => match &selection {
                TileSelection::NotPartOf => {
                    self.state
                        .map
                        .set_base(coords.0, coords.1, BaseTile::NotPartOfRoom);
                    self.state
                        .map
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
                TileSelection::Clear => {
                    self.state.map.set_base(coords.0, coords.1, BaseTile::Empty);
                    self.state
                        .map
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
                TileSelection::Ladder => {
                    self.state.map.set_base(coords.0, coords.1, BaseTile::Empty);
                    self.state
                        .map
                        .set_overlay(coords.0, coords.1, OverlayTile::Ladder);
                }
                TileSelection::Stone => {
                    self.state.map.set_base(coords.0, coords.1, BaseTile::Stone);
                    self.state
                        .map
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
                TileSelection::Wood => {
                    self.state.map.set_base(coords.0, coords.1, BaseTile::Wood);
                    self.state
                        .map
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
            },
            _ => {}
        }
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
            self.state.update(); // HERE is the actual game call
            self.state.input.jump = false;
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
                "FPS: {:.2}, UPS: {:.2}, updat: {:.2} draw: {:.2}",
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

        self.renderer.draw(&self.state);
        self.frames += 1;
        let draw_total = date::now() - draw_start;
        self.time_spent_drawing += draw_total;

        // GUI
        self.egui_mq
            .run(&mut *self.renderer.ctx, |_mq_ctx, egui_ctx| {
                egui::Window::new("Level editor").show(egui_ctx, |ui| {
                    // egui::widgets::global_theme_preference_buttons(ui);
                    // ui.checkbox(&mut true, "Show egui demo windows");

                    ui.add(egui::Label::new("Tool:"));

                    if ui
                        .add(egui::RadioButton::new(
                            matches!(
                                self.ui_config.editor_selection,
                                EditorSelection::Tiles { .. }
                            ),
                            "Tiles",
                        ))
                        .clicked()
                    {
                        self.ui_config.editor_selection = EditorSelection::Tiles {
                            selection: TileSelection::Clear,
                        };
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            matches!(
                                self.ui_config.editor_selection,
                                EditorSelection::Enemies { .. }
                            ),
                            "Enemies",
                        ))
                        .clicked()
                    {
                        self.ui_config.editor_selection = EditorSelection::Enemies {
                            selection: EnemySelection::Bat,
                        };
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            matches!(self.ui_config.editor_selection, EditorSelection::PlayerPos),
                            "Player position",
                        ))
                        .clicked()
                    {
                        self.ui_config.editor_selection = EditorSelection::PlayerPos;
                    }
                    if ui
                        .add(egui::RadioButton::new(
                            matches!(
                                self.ui_config.editor_selection,
                                EditorSelection::Doors { .. }
                            ),
                            "Doors",
                        ))
                        .clicked()
                    {
                        self.ui_config.editor_selection = EditorSelection::Doors {
                            selection: DoorSelection::Left,
                        };
                    }

                    let mut new_selection: Option<EditorSelection> = None;

                    match &self.ui_config.editor_selection {
                        EditorSelection::Tiles { selection } => {
                            ui.add(egui::Label::new("Tile:"));
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, TileSelection::NotPartOf),
                                    "NotPartOf",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Tiles {
                                    selection: TileSelection::NotPartOf,
                                });
                            }
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, TileSelection::Clear),
                                    "Clear",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Tiles {
                                    selection: TileSelection::Clear,
                                });
                            }
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, TileSelection::Wood),
                                    "Wood",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Tiles {
                                    selection: TileSelection::Wood,
                                });
                            }
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, TileSelection::Ladder),
                                    "Ladder",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Tiles {
                                    selection: TileSelection::Ladder,
                                });
                            }
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, TileSelection::Stone),
                                    "Stone",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Tiles {
                                    selection: TileSelection::Stone,
                                });
                            }
                        }
                        EditorSelection::Enemies { selection } => {
                            ui.add(egui::Label::new("Enemy:"));
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, EnemySelection::Bat),
                                    "Bat",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Enemies {
                                    selection: EnemySelection::Bat,
                                });
                            }
                            if ui
                                .add(egui::RadioButton::new(
                                    matches!(selection, EnemySelection::Slime),
                                    "Slime",
                                ))
                                .clicked()
                            {
                                new_selection = Some(EditorSelection::Enemies {
                                    selection: EnemySelection::Slime,
                                });
                            }
                        }
                        EditorSelection::PlayerPos => {
                            ui.add(egui::Label::new("Click to set player pos"));
                        }
                        EditorSelection::Doors { selection } => {
                            ui.add(egui::Label::new("Door:"));

                            for door_type in [
                                DoorSelection::Right,
                                DoorSelection::Left,
                                DoorSelection::Up,
                                DoorSelection::Down,
                                DoorSelection::Remove,
                            ] {
                                if ui
                                    .add(egui::RadioButton::new(
                                        *selection == door_type,
                                        // matches!(selection, door_type),
                                        format!("{:?}", door_type),
                                    ))
                                    .clicked()
                                {
                                    new_selection = Some(EditorSelection::Doors {
                                        selection: door_type,
                                    });
                                }
                            }
                        }
                    }

                    if let Some(selection) = new_selection {
                        self.ui_config.editor_selection = selection;
                    }

                    ui.add(egui::Label::new("Levels:"));

                    let mut remove_current = false;
                    let mut reload_rooms = false;
                    for (room_index, (file_name, room)) in self.all_rooms.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui.add(egui::Link::new(file_name)).clicked() {
                                println!("Clicked a link");
                                self.state.map = room.clone();
                                self.current_editor_room_index = room_index as u32;
                                self.state.player.bb.x = self.state.map.get_center().0;
                                self.state.player.bb.y = self.state.map.get_center().1;
                            }
                            if room_index as u32 == self.current_editor_room_index {
                                if ui.add(egui::Button::new("Save")).clicked() {
                                    self.state.map.resize_shrink();
                                    let path = Path::new("rooms").join(&file_name);
                                    self.state.map.save_json(path);
                                    println!("Button clicked!");
                                    reload_rooms = true;
                                }
                                if ui.add(egui::Button::new("Del")).clicked() {
                                    if self.all_rooms.len() > 2 {
                                        remove_current = true;
                                    }
                                }
                            }
                        });
                    }
                    if remove_current {
                        let file_name_to_remove = self.all_rooms
                            [self.current_editor_room_index as usize]
                            .0
                            .clone();

                        self.all_rooms
                            .remove(self.current_editor_room_index as usize);
                        self.state.map = self.all_rooms[0].1.clone();
                        self.current_editor_room_index = 0;

                        // Remove file_name_to_remove file name (in a folder called rooms)
                        let path = Path::new("rooms").join(&file_name_to_remove);

                        if let Err(err) = fs::remove_file(&path) {
                            eprintln!("Failed to remove room file '{}': {}", path.display(), err);
                        }
                    }

                    if ui.add(egui::Button::new("New room")).clicked() {
                        let new_room = Room::new_boxed(0, 0, 5, 5);
                        new_room.save_json(Room::next_available_file_name());
                        self.current_editor_room_index = self.all_rooms.len() as u32 - 1;
                        self.state.map = self.all_rooms[self.all_rooms.len() - 1].1.clone();
                        self.state.player.bb.x = self.state.map.get_center().0;
                        self.state.player.bb.y = self.state.map.get_center().1;
                        reload_rooms = true;
                    }

                    if reload_rooms {
                        self.all_rooms = Room::load_rooms_from_folder();
                    }
                });
            });

        self.egui_mq.draw(&mut *self.renderer.ctx);

        self.renderer.ctx.commit_frame();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.state.on_resize(width, height);
        self.renderer.resize(width, height);
    }

    fn key_down_event(&mut self, keycode: KeyCode, keymods: KeyMods, _repeat: bool) {
        match keycode {
            KeyCode::Left => self.state.input.left = true,
            KeyCode::Right => self.state.input.right = true,
            KeyCode::Up => self.state.input.up = true,
            KeyCode::X => self.state.input.swing = true,
            KeyCode::Z => self.state.input.jump = true,
            KeyCode::Down => self.state.input.down = true,
            _ => {}
        }
        self.egui_mq.key_down_event(keycode, keymods);
    }

    fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        match keycode {
            KeyCode::Left => self.state.input.left = false,
            KeyCode::Right => self.state.input.right = false,
            KeyCode::Up => self.state.input.up = false,
            KeyCode::X => self.state.input.swing = false,
            KeyCode::Z => self.state.input.jump = false,
            KeyCode::Down => self.state.input.down = false,
            _ => {}
        }
        self.egui_mq.key_up_event(keycode, keymods);
    }

    fn mouse_wheel_event(&mut self, dx: f32, dy: f32) {
        self.state.camera.zoom_scroll(dy);
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

        let coords =
            self.state
                .camera
                .screen_to_tile(x, y, self.state.screen_w, self.state.screen_h);
        println!("Mouse coords: {:?}", coords);

        self.handle_editor_tile_drawing(x, y);

        match &self.ui_config.editor_selection {
            EditorSelection::PlayerPos => {
                self.state.player.bb.x = coords.0 as f32;
                self.state.player.bb.y = coords.1 as f32;
            }
            EditorSelection::Enemies { selection } => {}
            EditorSelection::PlayerPos => {
                self.state.player.bb.x = coords.0 as f32;
                self.state.player.bb.y = coords.1 as f32;
            }
            EditorSelection::Doors { selection } => {
                for (sel, direction) in [
                    (DoorSelection::Up, DoorDir::Up),
                    (DoorSelection::Down, DoorDir::Down),
                    (DoorSelection::Right, DoorDir::Right),
                    (DoorSelection::Left, DoorDir::Left),
                ] {
                    if *selection == sel {
                        self.state.map.set_door(coords.0, coords.1, direction);
                    }
                }
                if *selection == DoorSelection::Remove {
                    self.state.map.remove_door(coords.0, coords.1);
                }
            }
            EditorSelection::Tiles { .. } => {} // This one is handled in the drawing function
        }
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
