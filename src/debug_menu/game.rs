use super::common::{DebugMenu, GameStateDebugMenu};
use crate::state::game_state::Game;
use egui::Ui;

use crate::camera::MouseCoords;

impl GameStateDebugMenu for Game {
    fn mouse_button_event(&mut self, _coords: MouseCoords, _stage: &mut DebugMenu) {}
    fn mouse_drawing(&mut self, _coords: MouseCoords, _debug_menu: &DebugMenu) {}

    fn render_ui(&mut self, ui: &mut Ui, stage: &mut DebugMenu) {
        if ui.add(egui::Button::new("Regenerate map")).clicked() {
            *self = Game::new();
        }

        ui.add(egui::Checkbox::new(
            &mut stage.zoom_show_full,
            "Zoom to room",
        ));
        ui.add(egui::Checkbox::new(&mut stage.show_dark, "Show dark"));
    }
}
