use super::common::{
    DebugMenu, DoorSelection, EditorSelection, EnemySelection, GameStateDebugMenu, TileSelection,
};
use crate::physics::EPS;
use crate::state::game_state::{Editor, GameState};
use crate::state::map_like::{DoorDir, MapLike, ObjectTemplate, ObjectTemplateType, Room};
use crate::state::{BaseTile, OverlayTile};
use egui::Ui;
use std::fs;
use std::path::Path;

use crate::camera::MouseCoords;

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
                TileSelection::StartDoor => {
                    self.map_mut().set_base(coords.0, coords.1, BaseTile::Empty);
                    self.map_mut()
                        .set_overlay(coords.0, coords.1, OverlayTile::StartDoor);
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
            EditorSelection::Enemies {
                snap_top,
                snap_bottom,
                selection,
            } => {
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
                    EnemySelection::Burrower => ObjectTemplateType::Burrower,
                };

                let template = ObjectTemplate::new(coords.x, coords.y, template_type.clone());
                let bb = template.get_bb();

                match (snap_top, snap_bottom) {
                    (false, false) => {
                        self.room.add_object_template(template);
                    }
                    (_, true) => {
                        let x1 = bb.x as i32;
                        let x2 = (bb.x + bb.w) as i32;
                        let mut y = bb.y.floor() as i32;

                        while !self.room.is_solid_at_tile(x1, y + 1)
                            && !self.room.is_solid_at_tile(x2, y + 1)
                        {
                            y += 1;
                        }

                        let new_template = ObjectTemplate::new(
                            coords.x,
                            y as f32 + (1.0 - bb.h) - EPS,
                            template_type,
                        );

                        self.room.add_object_template(new_template);
                    }
                    (true, _) => {
                        let x1 = bb.x as i32;
                        let x2 = (bb.x + bb.w) as i32;
                        let mut y = bb.y.floor() as i32;

                        while !self.room.is_solid_at_tile(x1, y - 1)
                            && !self.room.is_solid_at_tile(x2, y - 1)
                        {
                            y -= 1;
                        }

                        let new_template =
                            ObjectTemplate::new(coords.x, y as f32 + EPS, template_type);

                        self.room.add_object_template(new_template);
                    }
                }
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
                snap_bottom: false,
                snap_top: false,
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

        match &mut stage.editor_selection {
            EditorSelection::Tiles { selection } => {
                ui.add(egui::Label::new("Tile:"));

                ui.horizontal_wrapped(|ui| {
                    for (candidate, image_source) in [
                        (
                            TileSelection::NotPartOf,
                            egui::include_image!("../../assets/ui_sprites/not_part_of.png"),
                        ),
                        (
                            TileSelection::Clear,
                            egui::include_image!("../../assets/ui_sprites/clear.png"),
                        ),
                        (
                            TileSelection::Stone,
                            egui::include_image!("../../assets/ui_sprites/stone.png"),
                        ),
                        (
                            TileSelection::Wood,
                            egui::include_image!("../../assets/ui_sprites/wood.png"),
                        ),
                        (
                            TileSelection::Ladder,
                            egui::include_image!("../../assets/ui_sprites/ladder.png"),
                        ),
                        (
                            TileSelection::Platform,
                            egui::include_image!("../../assets/ui_sprites/platform.png"),
                        ),
                        (
                            TileSelection::StartDoor,
                            egui::include_image!("../../assets/ui_sprites/start_door.png"),
                        ),
                    ] {
                        let image = egui::Image::new(image_source)
                            .fit_to_exact_size(egui::Vec2::new(20.0, 20.0));
                        if ui
                            .add(egui::Button::image(image).selected(*selection == candidate))
                            .clicked()
                        {
                            new_selection = Some(EditorSelection::Tiles {
                                selection: candidate,
                            });
                        }
                    }
                });
            }
            EditorSelection::Enemies {
                snap_top,
                snap_bottom,
                selection,
            } => {
                ui.add(egui::Label::new("Enemy:"));

                ui.horizontal_wrapped(|ui| {
                    for (candidate, image_source) in [
                        (
                            EnemySelection::Remove,
                            egui::include_image!("../../assets/ui_sprites/remove.png"),
                        ),
                        (
                            EnemySelection::Bat,
                            egui::include_image!("../../assets/ui_sprites/bat.png"),
                        ),
                        (
                            EnemySelection::Slime,
                            egui::include_image!("../../assets/ui_sprites/slime.png"),
                        ),
                        (
                            EnemySelection::Worm,
                            egui::include_image!("../../assets/ui_sprites/worm.png"),
                        ),
                        (
                            EnemySelection::Burrower,
                            egui::include_image!("../../assets/ui_sprites/burrower.png"),
                        ),
                    ] {
                        let image = egui::Image::new(image_source)
                            .fit_to_exact_size(egui::Vec2::new(20.0, 20.0));
                        if ui
                            .add(egui::Button::image(image).selected(*selection == candidate))
                            .clicked()
                        {
                            new_selection = Some(EditorSelection::Enemies {
                                selection: candidate,
                                snap_bottom: *snap_bottom,
                                snap_top: *snap_top,
                            });
                        }
                    }
                });

                ui.add(egui::Checkbox::new(snap_bottom, "Snap bottom"));
                ui.add(egui::Checkbox::new(snap_top, "Snap top"));
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
                    DoorSelection::LevelStart,
                    DoorSelection::LevelEnd,
                    DoorSelection::Remove,
                ] {
                    if ui
                        .add(egui::RadioButton::new(
                            *selection == door_type,
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

        ui.add(egui::Label::new("Sort levels by:"));
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new("Name")).clicked() {
                stage.all_rooms.sort_by(|a, b| a.0.cmp(&b.0));
            }
            if ui.add(egui::Button::new("Enemies")).clicked() {
                stage
                    .all_rooms
                    .sort_by(|a, b| a.1.has_enemies().cmp(&b.1.has_enemies()));
            }
            if ui.add(egui::Button::new("InDoor")).clicked() {
                stage
                    .all_rooms
                    .sort_by(|a, b| a.1.has_start_door().cmp(&b.1.has_start_door()));
            }
            if ui.add(egui::Button::new("Active")).clicked() {
                stage
                    .all_rooms
                    .sort_by(|a, b| a.1.disabled.cmp(&b.1.disabled));
            }
        });

        ui.add(egui::Label::new("Levels:"));

        let mut remove_current = false;
        let mut reload_rooms = false;

        // TODO: Set the min size of checkboxes and others (decreases padding)
        // TODO: Find a better way to do this
        // Maybe like this?
        // ui.scope(|ui| {
        //     ui.spacing_mut().interact_size = egui::vec2(16.0, 16.0);
        // OR
        //     let spacing = &mut ui.style_mut().spacing;
        //     spacing.button_padding = egui::vec2(0.0, 0.0);
        ui.spacing_mut().interact_size.x = 16.0;
        ui.spacing_mut().interact_size.y = 16.0;

        egui::Grid::new("some_unique_id").show(ui, |ui| {
            for (room_index, (file_name, room)) in stage.all_rooms.iter().enumerate() {
                let is_current_room = room_index as u32 == stage.current_editor_room_index;

                // Enabled/Disabled checkbox
                let mut is_checked = !room.disabled;
                ui.add_enabled_ui(!is_current_room, |ui| {
                    ui.add_sized([16.0, 16.0], egui::Checkbox::without_text(&mut is_checked));
                });
                // ui.add_enabled(
                //     !is_current_room,
                //     egui::Checkbox::without_text(&mut is_checked)
                // );
                if is_checked == room.disabled {
                    println!("Clicked a checkbox");

                    self.room.disabled = !room.disabled;

                    let path = Path::new("rooms").join(file_name);
                    self.room.save_json(path);
                    reload_rooms = true;
                }

                // Change room link
                ui.horizontal(|ui| {
                    if ui.add(egui::Link::new(file_name.clone())).clicked() {
                        println!("Clicked a link");
                        self.room = Room::clone(room);
                        stage.current_editor_room_index = room_index as u32;
                        self.player_mut().bb.x = self.room.get_center().0;
                        self.player_mut().bb.y = self.room.get_center().1;
                    }
                });

                // Room information (has enemies, has starting room etc.)
                if room.has_start_door() {
                    ui.add(egui::Label::new("\u{1F6AA}"));
                }
                if room.has_enemies() {
                    ui.add(egui::Label::new("\u{1F432}"));
                }

                if is_current_room {
                    // Save / Delete buttons
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Save")).clicked() {
                            println!("Button clicked!");

                            self.room.resize_shrink();

                            let path = Path::new("rooms").join(file_name);
                            self.room.save_json(path);
                            reload_rooms = true;
                        }
                        if ui.add(egui::Button::new("Del")).clicked() && stage.all_rooms.len() > 2 {
                            remove_current = true;
                        }
                    });

                    // Room information
                    ui.end_row();
                    ui.add(egui::Label::new(""));
                    ui.add(egui::Label::new(format!(
                        "Size: ({} {})",
                        room.get_pos().0 - room.w as i32,
                        room.get_pos().1 - room.h as i32
                    )));
                }
                ui.end_row();
            }
        });
        if remove_current {
            let file_name_to_remove = stage.all_rooms[stage.current_editor_room_index as usize]
                .0
                .clone();

            stage
                .all_rooms
                .remove(stage.current_editor_room_index as usize);
            self.room = stage.all_rooms[0].1.clone();
            stage.current_editor_room_index = 0;

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
