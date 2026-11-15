//! Checks on the game content in `rooms/` and on map generation. These catch a
//! broken room saved from the editor before it shows up as a weird bug in play.

use platosku2::state::GameRng;
use platosku2::state::map_like::{GameMap, MapLike, Room};
use rand::SeedableRng;

fn all_rooms() -> Vec<(String, Room)> {
    Room::load_rooms_from_folder()
}

fn room_candidates() -> Vec<Room> {
    all_rooms().into_iter().map(|(_, room)| room).collect()
}

#[test]
fn all_rooms_load_and_have_doors() {
    let rooms = all_rooms();
    assert!(!rooms.is_empty(), "no rooms found in rooms/");
    for (name, room) in &rooms {
        assert!(!room.get_doors().is_empty(), "{name} has no doors");
        assert!(room.w > 0 && room.h > 0, "{name} is empty");
    }
}

#[test]
fn some_room_has_a_start_door() {
    let start_rooms: Vec<String> = all_rooms()
        .into_iter()
        .filter(|(_, room)| room.get_start_pos().is_some())
        .map(|(name, _)| name)
        .collect();
    assert!(!start_rooms.is_empty(), "no room has a start door");
}

#[test]
fn doors_face_out_of_the_room() {
    use platosku2::state::BaseTile;
    use platosku2::state::map_like::DoorDir;
    for (name, room) in all_rooms() {
        let (rx, ry) = room.get_pos();
        for door in room.get_doors() {
            let (dx, dy) = match door.dir {
                DoorDir::Left => (-1, 0),
                DoorDir::Right => (1, 0),
                DoorDir::Up => (0, -1),
                DoorDir::Down => (0, 1),
            };
            let (x, y) = (rx + door.x as i32, ry + door.y as i32);
            let inside = (rx + door.x as i32 - dx, ry + door.y as i32 - dy);
            let outside = (x + dx, y + dy);

            let outside_tile = room.get_relative(outside.0, outside.1).map(|t| t.0);
            assert!(
                matches!(outside_tile, None | Some(BaseTile::NotPartOfRoom)),
                "{name}: door {door:?} does not lead out of the room (beyond it: {outside_tile:?})"
            );
            let inside_tile = room.get_relative(inside.0, inside.1).map(|t| t.0);
            assert!(
                matches!(inside_tile, Some(t) if t != BaseTile::NotPartOfRoom),
                "{name}: door {door:?} has no room tile behind it"
            );
        }
    }
}

const SEEDS: u64 = 200;

#[test]
fn map_generation_works_for_many_seeds() {
    let candidates = room_candidates();
    for seed in 0..SEEDS {
        let mut rng = GameRng::seed_from_u64(seed);
        let map = GameMap::new_random(&candidates, &mut rng);

        assert_eq!(map.rooms.len(), 10, "seed {seed}: expected 10 rooms");
        assert_eq!(
            map.doors.len(),
            map.rooms.len() - 1,
            "seed {seed}: every added room should come with one door"
        );

        // The player must start in open space.
        let (sx, sy) = map.player_start_pos();
        assert!(
            !map.is_solid_at_tile(sx as i32, sy as i32),
            "seed {seed}: player starts inside a solid tile"
        );

        // Every door must connect two rooms: open tiles on both sides.
        for door in &map.doors {
            let (a, b) = if door.goes_up_down {
                ((door.x, door.y - 1), (door.x, door.y + 1))
            } else {
                ((door.x - 1, door.y), (door.x + 1, door.y))
            };
            assert!(
                !map.is_solid_at_tile(door.x, door.y),
                "seed {seed}: door at ({}, {}) is solid",
                door.x,
                door.y
            );
            assert!(
                map.get_room_at_i(a.0, a.1).is_some() && map.get_room_at_i(b.0, b.1).is_some(),
                "seed {seed}: door at ({}, {}) does not connect two rooms",
                door.x,
                door.y
            );
        }
    }
}

#[test]
fn generated_maps_have_no_stacked_duplicate_rooms() {
    let candidates = room_candidates();
    for seed in 0..SEEDS {
        let mut rng = GameRng::seed_from_u64(seed);
        let map = GameMap::new_random(&candidates, &mut rng);
        let positions: Vec<((i32, i32), u32, u32)> =
            map.rooms.iter().map(|r| (r.get_pos(), r.w, r.h)).collect();
        for (i, a) in positions.iter().enumerate() {
            for b in &positions[i + 1..] {
                assert_ne!(
                    a, b,
                    "seed {seed}: two identical rooms at the same position"
                );
            }
        }
    }
}
