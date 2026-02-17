use crate::state::GameState;
use crate::state::map_like::Room;
use egui::Ui;

use crate::camera::MouseCoords;

#[derive(Debug, Eq, PartialEq)]
pub enum TileSelection {
    NotPartOf,
    Clear,
    Stone,
    Wood,
    Ladder,
    Platform,
    StartDoor,
}

#[derive(Debug, Eq, PartialEq)]
pub enum EnemySelection {
    Remove,
    Bat,
    Slime,
    Worm,
    Burrower,
}

#[derive(Debug, Eq, PartialEq)]
pub enum DoorSelection {
    Left,
    Right,
    Up,
    Down,
    LevelStart,
    LevelEnd,
    Remove,
}

pub enum EditorSelection {
    Tiles {
        selection: TileSelection,
    },
    Enemies {
        snap_bottom: bool,
        snap_top: bool,
        selection: EnemySelection,
    },
    PlayerPos,
    Doors {
        selection: DoorSelection,
    },
}

pub struct DebugMenu {
    pub editor_selection: EditorSelection,
    pub all_rooms: Vec<(String, Room)>,
    pub current_editor_room_index: u32,
    pub is_game: bool,
    pub zoom_show_full: bool,
    pub show_dark: bool,
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
            show_dark: true,
        }
    }
}

pub trait GameStateDebugMenu: GameState {
    fn mouse_button_event(&mut self, coords: MouseCoords, stage: &mut DebugMenu);
    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu);
    fn mouse_drawing(&mut self, coords: MouseCoords, debug_menu: &DebugMenu);
}
