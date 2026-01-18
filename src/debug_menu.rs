use crate::camera::MouseCoords;
use crate::state::game_map::{DoorDir, ObjectTemplateType, Room};
use crate::state::game_state::{Editor, Game};
use crate::state::{BaseTile, GameState, OverlayTile};
use crate::{DebugMenu, DoorSelection, EditorSelection, EnemySelection, TileSelection};
use egui::Ui;
use std::fs;
use std::path::Path;

pub trait GameStateDebugMenu: GameState {
    fn mouse_button_event(&mut self, coords: MouseCoords, stage: &mut DebugMenu);
    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu);
    fn mouse_drawing(&mut self, coords: MouseCoords, debug_menu: &DebugMenu);
}

impl GameStateDebugMenu for Game {
    fn mouse_button_event(&mut self, _coords: MouseCoords, _stage: &mut DebugMenu) {}
    fn mouse_drawing(&mut self, _coords: MouseCoords, _debug_menu: &DebugMenu) {}

    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu) {
        if ui.add(egui::Button::new("Regenerate map")).clicked() {
            // TODO: This is hack to regenerate the state. Need to figure out somethign better...
            let new_state = Game::new();
            self.map = new_state.map;
            self.enemies = new_state.enemies;
            self.player = new_state.player;
        }

        ui.add(egui::Checkbox::new(
            &mut stage.zoom_show_full,
            "Zoom to room",
        ));
        ui.add(egui::Checkbox::new(&mut stage.show_dark, "Show dark"));
    }
}

impl GameStateDebugMenu for Editor {
    fn mouse_drawing(&mut self, coords: MouseCoords, debug_menu: &DebugMenu) {
        let coords = coords.as_i();
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

                    let tile_to_set = match self.map().get_at(coords.0, coords.1) {
                        (_, OverlayTile::Platform) => OverlayTile::LadderPlatform,
                        _ => OverlayTile::Ladder,
                    };

                    self.map_mut().set_overlay(coords.0, coords.1, tile_to_set);
                }
                TileSelection::Platform => {
                    self.map_mut().set_base(coords.0, coords.1, BaseTile::Empty);

                    let tile_to_set = match self.map().get_at(coords.0, coords.1) {
                        (_, OverlayTile::Ladder) => OverlayTile::LadderPlatform,
                        _ => OverlayTile::Platform,
                    };

                    self.map_mut().set_overlay(coords.0, coords.1, tile_to_set);
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

    fn mouse_button_event(&mut self, coords: MouseCoords, stage: &mut DebugMenu) {
        match &stage.editor_selection {
            EditorSelection::PlayerPos => {
                self.player_mut().bb.x = coords.x;
                self.player_mut().bb.y = coords.y;
            }
            EditorSelection::Enemies { selection } => {
                let template_type = match selection {
                    EnemySelection::Remove => {
                        self.room.object_templates.retain(|template| {
                            let bb = template.get_bb();
                            !(coords.x > bb.x
                                && coords.x < bb.x + bb.w
                                && coords.y > bb.y
                                && coords.y < bb.y + bb.h)
                        });
                        return;
                    }
                    EnemySelection::Bat => ObjectTemplateType::Bat,
                    EnemySelection::Slime => ObjectTemplateType::Slime,
                    EnemySelection::Worm => ObjectTemplateType::Worm,
                };
                self.room
                    .add_object_template(coords.x, coords.y, template_type);
            }
            EditorSelection::Doors { selection } => {
                let coords = coords.as_i();
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

                for candidate in [
                    TileSelection::NotPartOf,
                    TileSelection::Clear,
                    TileSelection::Wood,
                    TileSelection::Ladder,
                    TileSelection::Platform,
                    TileSelection::Stone,
                ] {
                    if ui
                        .add(egui::RadioButton::new(
                            *selection == candidate,
                            format!("{:?}", candidate),
                        ))
                        .clicked()
                    {
                        new_selection = Some(EditorSelection::Tiles {
                            selection: candidate,
                        });
                    }
                }
            }
            EditorSelection::Enemies { selection } => {
                ui.add(egui::Label::new("Enemy:"));

                for candidate in [
                    EnemySelection::Remove,
                    EnemySelection::Bat,
                    EnemySelection::Slime,
                    EnemySelection::Worm,
                ] {
                    if ui
                        .add(egui::RadioButton::new(
                            *selection == candidate,
                            format!("{:?}", candidate),
                        ))
                        .clicked()
                    {
                        new_selection = Some(EditorSelection::Enemies {
                            selection: candidate,
                        });
                    }
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

            stage.current_editor_room_index = stage.all_rooms.len() as u32;
            self.room = new_room.clone();

            self.player_mut().bb.x = self.room.get_center().0;
            self.player_mut().bb.y = self.room.get_center().1;

            reload_rooms = true;
        }

        if reload_rooms {
            stage.all_rooms = Room::load_rooms_from_folder();
        }
    }
}
