//! What the game and the editor draw, on top of the map tiles.

use super::{Renderer, TILE_SIZE};
use crate::camera::Camera;
use crate::state::BaseTile;
use crate::state::GameState;
use crate::state::game_state::{Editor, Game};
use crate::state::map_like::DoorDir;

pub trait DrawableGameState: GameState {
    /// World sprites drawn after the tiles and before the player
    /// (doors, items, enemies).
    fn draw_world(&self, renderer: &mut Renderer);
    /// World sprites drawn after the player (enemy health bars).
    fn draw_world_top(&self, renderer: &mut Renderer);
    /// Which tiles the dark overlay covers this frame, or `None` for no overlay.
    fn dark_tiles(&self) -> Option<Box<dyn Fn(i32, i32) -> bool + '_>>;
    fn update_and_draw_minimap(&mut self, renderer: &mut Renderer, camera: &Camera, draw_big: bool);
}

impl DrawableGameState for Game {
    fn draw_world(&self, renderer: &mut Renderer) {
        for door in &self.map.doors {
            renderer.draw_from_texture_atlas(
                "door",
                door.get_atlas_index(),
                false,
                door.x as f32,
                door.y as f32,
                1.0,
                1.0,
                1.0,
            );
        }
        for item in &self.items {
            item.draw(renderer);
        }
        for enemy in &self.enemies {
            enemy.draw(renderer);
        }
    }

    fn draw_world_top(&self, renderer: &mut Renderer) {
        for enemy in &self.enemies {
            let health = enemy.get_health().ratio();
            if health < 1.0 && enemy.should_render_health_bar() {
                let bb = enemy.bb();
                renderer.draw_health_bar_above(bb.x, bb.y, bb.w, health);
            }
        }
    }

    fn dark_tiles(&self) -> Option<Box<dyn Fn(i32, i32) -> bool + '_>> {
        let (cur_room, prev_room, ratio, door_pos) = self.get_rooms_for_display();

        if ratio == 1.0 || ratio == 0.0 {
            // Everything outside the current room is dark.
            return Some(Box::new(move |x, y| {
                !matches!(
                    cur_room.and_then(|room| room.get_relative(x, y)),
                    Some((base, _)) if base != BaseTile::NotPartOfRoom
                )
            }));
        }

        let (Some(room1), Some(room2)) = (cur_room, prev_room) else {
            return None;
        };
        // Room transition: the dark spreads from the door over the previous
        // room and lifts from the new one.
        Some(Box::new(move |x, y| {
            let dist_to_door =
                ((x - door_pos.0) * (x - door_pos.0) + (y - door_pos.1) * (y - door_pos.1)).isqrt();
            let room_max_dist = room1.h.max(room1.w);
            let distance_ratio = dist_to_door as f32 / room_max_dist as f32;

            if let Some((base, _overlay)) = room1.get_relative(x, y) {
                if (1.0 - distance_ratio) < ratio {
                    return true;
                }
                if base != BaseTile::NotPartOfRoom {
                    return false;
                }
            }
            if let Some((base, _overlay)) = room2.get_relative(x, y) {
                if distance_ratio / 2.0 + 0.5 > ratio {
                    return true;
                }
                if base != BaseTile::NotPartOfRoom {
                    return false;
                }
            }
            true
        }))
    }

    fn update_and_draw_minimap(
        &mut self,
        renderer: &mut Renderer,
        camera: &Camera,
        draw_big: bool,
    ) {
        if let Some((minimap, map, cur_room_index, visited_rooms)) = self.minimap_parts() {
            minimap.update_and_draw_minimap(
                renderer,
                camera,
                map,
                cur_room_index,
                visited_rooms,
                draw_big,
            )
        }
    }
}

impl DrawableGameState for Editor {
    fn draw_world(&self, renderer: &mut Renderer) {
        let room_pos = self.room.get_pos();
        for door in self.room.get_doors() {
            let x = room_pos.0 + door.x as i32;
            let y = room_pos.1 + door.y as i32;
            let frame = match door.dir {
                DoorDir::Up => 3,
                DoorDir::Right => 4,
                DoorDir::Down => 5,
                DoorDir::Left => 6,
            };
            renderer
                .draw_from_texture_atlas("tiles", frame, false, x as f32, y as f32, 1.0, 1.0, 1.0);
        }

        // Enemy spawn points
        for template in &self.room.object_templates {
            let bb = template.get_bb();
            renderer.draw_from_texture_atlas(
                template.get_texture_index(),
                0,
                false,
                bb.x - 1.0 / TILE_SIZE,
                bb.y - 1.0 / TILE_SIZE,
                bb.w + 2.0 / TILE_SIZE,
                bb.h + 2.0 / TILE_SIZE,
                1.0,
            );
        }
    }

    fn draw_world_top(&self, _renderer: &mut Renderer) {}

    fn dark_tiles(&self) -> Option<Box<dyn Fn(i32, i32) -> bool + '_>> {
        None
    }

    fn update_and_draw_minimap(
        &mut self,
        _renderer: &mut Renderer,
        _camera: &Camera,
        _draw_big: bool,
    ) {
    }
}
