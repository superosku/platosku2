use crate::state::game_map::{DoorDir, GameMap, Room};
use crate::state::game_state::{Editor, Game};
use crate::state::{BaseTile, GameState, OverlayTile};
use crate::{DebugMenu, DoorSelection, EditorSelection, EnemySelection, TileSelection};
use egui::Ui;
use std::fs;
use std::path::Path;

pub trait GameStateDebugMenu: GameState {
    fn mouse_button_event(&mut self, coords: (i32, i32), stage: &mut DebugMenu);
    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu);
    fn mouse_drawing(&mut self, coords: (i32, i32), debug_menu: &DebugMenu);
}

impl GameStateDebugMenu for Game {
    fn mouse_button_event(&mut self, _coords: (i32, i32), _stage: &mut DebugMenu) {}
    fn mouse_drawing(&mut self, _coords: (i32, i32), _debug_menu: &DebugMenu) {}

    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu) {
        if ui.add(egui::Button::new("Regenerate map")).clicked() {
            self.map = GameMap::new_random();
            let player_pos = self.map.player_start_pos();
            self.player.bb.x = player_pos.0;
            self.player.bb.y = player_pos.1;
        }

        if ui
            .add(egui::RadioButton::new(
                !stage.zoom_show_full,
                "Zoom to room",
            ))
            .clicked()
        {
            println!("Setting zoom to room");
            stage.zoom_show_full = false
        }

        if ui
            .add(egui::RadioButton::new(
                stage.zoom_show_full,
                "Zoom show all",
            ))
            .clicked()
        {
            println!("Setting zomo show all");
            stage.zoom_show_full = true
        }
    }
}

impl GameStateDebugMenu for Editor {
    fn mouse_drawing(&mut self, coords: (i32, i32), debug_menu: &DebugMenu) {
        if let EditorSelection::Tiles { selection } = &debug_menu.editor_selection {
            match &selection {
                TileSelection::NotPartOf => {
                    self.map_mut()
                        .set_base(coords.0, coords.1, BaseTile::NotPartOfRoom);
                    self.map_mut()
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
                TileSelection::Clear => {
                    self.map_mut().set_base(coords.0, coords.1, BaseTile::Empty);
                    self.map_mut()
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
                TileSelection::Ladder => {
                    self.map_mut().set_base(coords.0, coords.1, BaseTile::Empty);
                    self.map_mut()
                        .set_overlay(coords.0, coords.1, OverlayTile::Ladder);
                }
                TileSelection::Stone => {
                    self.map_mut().set_base(coords.0, coords.1, BaseTile::Stone);
                    self.map_mut()
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
                TileSelection::Wood => {
                    self.map_mut().set_base(coords.0, coords.1, BaseTile::Wood);
                    self.map_mut()
                        .set_overlay(coords.0, coords.1, OverlayTile::None);
                }
            }
        }
    }

    fn mouse_button_event(&mut self, coords: (i32, i32), stage: &mut DebugMenu) {
        match &stage.editor_selection {
            EditorSelection::PlayerPos => {
                self.player_mut().bb.x = coords.0 as f32;
                self.player_mut().bb.y = coords.1 as f32;
            }
            EditorSelection::Enemies { .. } => {}
            EditorSelection::Doors { selection } => {
                for (sel, direction) in [
                    (DoorSelection::Up, DoorDir::Up),
                    (DoorSelection::Down, DoorDir::Down),
                    (DoorSelection::Right, DoorDir::Right),
                    (DoorSelection::Left, DoorDir::Left),
                ] {
                    if *selection == sel {
                        self.room.set_door(coords.0, coords.1, direction);
                    }
                }
                if *selection == DoorSelection::Remove {
                    self.room.remove_door(coords.0, coords.1);
                }
            }
            EditorSelection::Tiles { .. } => {} // This one is handled in the drawing function
        }
    }

    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu) {
        ui.add(egui::Label::new("Tool:"));

        if ui
            .add(egui::RadioButton::new(
                matches!(stage.editor_selection, EditorSelection::Tiles { .. }),
                "Tiles",
            ))
            .clicked()
        {
            stage.editor_selection = EditorSelection::Tiles {
                selection: TileSelection::Clear,
            };
        }
        if ui
            .add(egui::RadioButton::new(
                matches!(stage.editor_selection, EditorSelection::Enemies { .. }),
                "Enemies",
            ))
            .clicked()
        {
            stage.editor_selection = EditorSelection::Enemies {
                selection: EnemySelection::Bat,
            };
        }
        if ui
            .add(egui::RadioButton::new(
                matches!(stage.editor_selection, EditorSelection::PlayerPos),
                "Player position",
            ))
            .clicked()
        {
            stage.editor_selection = EditorSelection::PlayerPos;
        }
        if ui
            .add(egui::RadioButton::new(
                matches!(stage.editor_selection, EditorSelection::Doors { .. }),
                "Doors",
            ))
            .clicked()
        {
            stage.editor_selection = EditorSelection::Doors {
                selection: DoorSelection::Left,
            };
        }

        let mut new_selection: Option<EditorSelection> = None;

        match &stage.editor_selection {
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
            stage.editor_selection = selection;
        }

        ui.add(egui::Label::new("Levels:"));

        let mut remove_current = false;
        let mut reload_rooms = false;
        for (room_index, (file_name, room)) in stage.all_rooms.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.add(egui::Link::new(file_name)).clicked() {
                    println!("Clicked a link");

                    self.room = room.clone();
                    stage.current_editor_room_index = room_index as u32;
                    self.player_mut().bb.x = self.room.get_center().0;
                    self.player_mut().bb.y = self.room.get_center().1;
                }
                if room_index as u32 == stage.current_editor_room_index {
                    if ui.add(egui::Button::new("Save")).clicked() {
                        self.room.resize_shrink();
                        let path = Path::new("rooms").join(file_name);
                        self.room.save_json(path);
                        println!("Button clicked!");
                        reload_rooms = true;
                    }
                    if ui.add(egui::Button::new("Del")).clicked() && stage.all_rooms.len() > 2 {
                        remove_current = true;
                    }
                }
            });
        }
        if remove_current {
            let file_name_to_remove = stage.all_rooms[stage.current_editor_room_index as usize]
                .0
                .clone();

            stage
                .all_rooms
                .remove(stage.current_editor_room_index as usize);
            self.room = stage.all_rooms[0].1.clone();
            stage.current_editor_room_index = 0;

            // Remove file_name_to_remove file name (in a folder called rooms)
            let path = Path::new("rooms").join(&file_name_to_remove);

            if let Err(err) = fs::remove_file(&path) {
                eprintln!("Failed to remove room file '{}': {}", path.display(), err);
            }
        }

        if ui.add(egui::Button::new("New room")).clicked() {
            let new_room = Room::new_boxed(0, 0, 5, 5);
            new_room.save_json(Room::next_available_file_name());
            stage.current_editor_room_index = stage.all_rooms.len() as u32 - 1;

            self.room = stage.all_rooms[stage.all_rooms.len() - 1].1.clone();

            self.player_mut().bb.x = self.room.get_center().0;
            self.player_mut().bb.y = self.room.get_center().1;
            reload_rooms = true;
        }

        if reload_rooms {
            stage.all_rooms = Room::load_rooms_from_folder();
        }
    }
}
