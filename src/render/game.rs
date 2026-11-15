//! What the game and the editor draw, on top of the map tiles.

use super::Renderer;
use crate::camera::Camera;
use crate::state::BaseTile;
use crate::state::GameState;
use crate::state::draw::{DrawList, Sprite};
use crate::state::game_state::{Editor, Game};
use crate::state::map_like::DoorDir;

pub trait DrawableGameState: GameState {
    /// World sprites drawn after the tiles and before the player
    /// (doors, items, enemies).
    fn draw_world(&self, out: &mut DrawList);
    /// World sprites drawn after the player (enemy health bars).
    fn draw_world_top(&self, out: &mut DrawList);
    /// Which tiles the dark overlay covers this frame, or `None` for no overlay.
    fn dark_tiles(&self) -> Option<Box<dyn Fn(i32, i32) -> bool + '_>>;
    fn update_and_draw_minimap(&mut self, renderer: &mut Renderer, camera: &Camera, draw_big: bool);
}

impl DrawableGameState for Game {
    fn draw_world(&self, out: &mut DrawList) {
        for door in &self.map.doors {
            let (x, y) = (door.x as f32, door.y as f32);
            out.sprite(Sprite::new("door", door.get_atlas_index(), x, y, 1.0, 1.0));
        }
        for item in &self.items {
            item.draw(out);
        }
        for enemy in &self.enemies {
            enemy.draw(out);
        }
    }

    fn draw_world_top(&self, out: &mut DrawList) {
        for enemy in &self.enemies {
            let health = enemy.get_health().ratio();
            if health < 1.0 && enemy.should_render_health_bar() {
                out.health_bar_above(enemy.bb(), health);
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
    fn draw_world(&self, out: &mut DrawList) {
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
            out.sprite(Sprite::new("tiles", frame, x as f32, y as f32, 1.0, 1.0));
        }

        // Enemy spawn points
        for template in &self.room.object_templates {
            let bb = template.get_bb();
            out.sprite(Sprite::around(template.get_texture_index(), 0, &bb));
        }
    }

    fn draw_world_top(&self, _out: &mut DrawList) {}

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
