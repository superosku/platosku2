//! Shared helpers for the e2e tests.
#![allow(dead_code)] // Not every test file uses every helper.

use platosku2::sim::Sim;
use platosku2::state::game_state::Game;
use platosku2::state::map_like::Room;

/// A simulation of a single room drawn as ASCII art (see `Room::from_ascii`),
/// with no random items. The player starts at the `S` tile.
pub fn sim_in(rows: &[&str]) -> Sim {
    sim_in_seeded(rows, 0)
}

pub fn sim_in_seeded(rows: &[&str], seed: u64) -> Sim {
    let room = Room::from_ascii(0, 0, rows);
    Sim::new(
        Game::builder()
            .seed(seed)
            .single_room(room)
            .random_items(false)
            .build(),
    )
}

/// The player's feet (bottom of the bounding box).
pub fn feet(sim: &Sim) -> f32 {
    sim.player().bb.y + sim.player().bb.h
}
