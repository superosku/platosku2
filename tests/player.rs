//! End-to-end tests for player movement: walking, jumping, coyote time,
//! ladders, one-way platforms and ledge hanging.

mod common;
use common::*;
use platosku2::sim::{Key, Sim};
use platosku2::sound_handler::Sound;
use platosku2::state::player::PlayerState;

const FLAT: &[&str] = &[
    "####################",
    "#..................#",
    "#..................#",
    "#..................#",
    "#..................#",
    "#S.................#",
    "####################",
];

/// A settled player standing on the floor of `FLAT`.
fn standing_in_flat_room() -> Sim {
    let mut sim = sim_in(FLAT);
    sim.run(30);
    assert!(sim.player().is_on_ground());
    sim
}

/// Jumps while holding the jump key for `hold_ticks` and returns the height
/// reached, in tiles.
fn jump_height(sim: &mut Sim, hold_ticks: u32) -> f32 {
    let start = sim.player().bb.y;
    let mut min_y = start;
    sim.press(Key::Jump)
        .run_observing(hold_ticks.max(1), |g| min_y = min_y.min(g.player.bb.y))
        .release(Key::Jump)
        .run_observing(120, |g| min_y = min_y.min(g.player.bb.y));
    start - min_y
}

#[test]
fn player_settles_on_floor() {
    let sim = standing_in_flat_room();
    assert!((feet(&sim) - 6.0).abs() < 0.001, "feet at {}", feet(&sim));
    assert_eq!(sim.player().bb.vy, 0.0);
}

#[test]
fn walking_accelerates_to_max_speed_and_stops_with_friction() {
    let mut sim = standing_in_flat_room();
    let x0 = sim.player().bb.x;

    sim.press(Key::Right).run(3);
    assert_eq!(sim.player().bb.vx, 0.06, "max speed after 3 ticks");

    sim.run(57);
    let walked = sim.player().bb.x - x0;
    assert!((3.4..3.7).contains(&walked), "walked {walked} tiles in 1 s");

    sim.release(Key::Right).run(20);
    assert_eq!(sim.player().bb.vx, 0.0, "friction should stop the player");
}

#[test]
fn walking_into_a_wall_stops_at_the_wall() {
    let mut sim = standing_in_flat_room();
    sim.press(Key::Left).run(60);
    // The left wall is the column x = 0..1.
    let x = sim.player().bb.x;
    assert!((1.0..1.01).contains(&x), "x = {x}");
}

#[test]
fn tapping_jump_gives_a_low_jump_and_holding_a_high_one() {
    let mut sim = standing_in_flat_room();
    let tap = jump_height(&mut sim, 1);
    let held = jump_height(&mut sim, 30);

    assert!((0.9..1.2).contains(&tap), "tap jump height {tap}");
    assert!((2.1..2.4).contains(&held), "held jump height {held}");
}

#[test]
fn jump_plays_sound_and_lands_again() {
    let mut sim = standing_in_flat_room();
    sim.clear_sounds();
    sim.tap(Key::Jump);
    assert_eq!(sim.sounds(), vec![Sound::Jump]);
    assert!(sim.player().bb.vy < 0.0, "should be moving up");

    let landed = sim.run_until(120, |g| g.player.is_on_ground());
    assert!(landed.is_some(), "player should land");
}

#[test]
#[ignore = "known bug: holding jump keeps re-jumping silently after landing (max_jump_frames is reset on the ground)"]
fn holding_jump_does_not_jump_again_after_landing() {
    let mut sim = standing_in_flat_room();
    sim.press(Key::Jump).run(120);
    assert!(sim.player().is_on_ground());
    sim.clear_sounds().run(60);
    assert!(
        sim.player().is_on_ground(),
        "holding jump must not bunny hop"
    );
    assert!(sim.sounds().is_empty());
}

const LEDGE: &[&str] = &[
    "##############",
    "#............#",
    "#............#",
    "#S...........#",
    "#####........#",
    "#............#",
    "#............#",
    "##############",
];

/// Walks right off the ledge and returns the sim on the first tick the player is
/// in the air.
fn just_walked_off_ledge() -> Sim {
    let mut sim = sim_in(LEDGE);
    sim.run(30).press(Key::Right);
    let ticks = sim.run_until(200, |g| !g.player.is_on_ground());
    assert!(ticks.is_some(), "player never left the ledge");
    sim.release(Key::Right);
    sim
}

#[test]
fn coyote_time_allows_jumping_just_after_leaving_a_ledge() {
    let mut sim = just_walked_off_ledge();
    sim.run(2).tap(Key::Jump);
    assert!(sim.player().bb.vy < 0.0, "late jump should still work");
    assert!(sim.sounds().contains(&Sound::Jump));
}

#[test]
fn no_jump_long_after_leaving_a_ledge() {
    let mut sim = just_walked_off_ledge();
    sim.run(8).clear_sounds().tap(Key::Jump);
    assert!(sim.player().bb.vy > 0.0, "should still be falling");
    assert!(!sim.sounds().contains(&Sound::Jump));
}

const LADDER: &[&str] = &[
    "#######", "#..H..#", "#..H..#", "#..H..#", "#.SH..#", "#######",
];

/// Walks right until the player's center is over the ladder column (x = 3).
fn at_ladder_bottom() -> Sim {
    let mut sim = sim_in(LADDER);
    sim.run(30).press(Key::Right);
    let reached = sim.run_until(60, |g| {
        let center = g.player.bb.x + g.player.bb.w * 0.5;
        (3.2..3.8).contains(&center)
    });
    assert!(reached.is_some(), "never reached the ladder");
    sim.release(Key::Right).run(10);
    sim
}

#[test]
fn climbing_a_ladder_up_and_down() {
    let mut sim = at_ladder_bottom();
    let start_y = sim.player().bb.y;

    sim.press(Key::Up).run(1);
    assert!(matches!(sim.player().state(), PlayerState::OnLadder));
    // Climbing snaps the player to the middle of the ladder.
    let center = sim.player().bb.x + sim.player().bb.w * 0.5;
    assert!((center - 3.5).abs() < 0.001, "center {center}");

    sim.run(120).release(Key::Up);
    let top_y = sim.player().bb.y;
    assert!(
        start_y - top_y > 2.5,
        "climbed only {} tiles",
        start_y - top_y
    );
    // Ladder climbing moves the player without collision checks, so the head can
    // end up at most one climb step (max_speed = 0.06 tiles, under one pixel)
    // inside the ceiling tile.
    assert!(top_y >= 1.0 - 0.06, "climbed into the ceiling: y = {top_y}");

    sim.press(Key::Down);
    let off = sim.run_until(200, |g| !matches!(g.player.state(), PlayerState::OnLadder));
    assert!(off.is_some(), "should leave the ladder at the bottom");
    assert!(
        (feet(&sim) - 5.0).abs() < 0.01,
        "back on the floor, feet {}",
        feet(&sim)
    );
}

#[test]
fn jumping_off_a_ladder() {
    let mut sim = at_ladder_bottom();
    sim.press(Key::Up).run(30).release(Key::Up);
    assert!(matches!(sim.player().state(), PlayerState::OnLadder));
    sim.tap(Key::Jump);
    assert!(matches!(sim.player().state(), PlayerState::Normal));
}

const PLATFORM: &[&str] = &[
    "#######", "#.....#", "#..S..#", "#-----#", "#.....#", "#######",
];

#[test]
fn standing_on_a_one_way_platform() {
    let mut sim = sim_in(PLATFORM);
    sim.run(60);
    assert!(sim.player().is_on_ground());
    assert!((feet(&sim) - 3.0).abs() < 0.01, "feet {}", feet(&sim));
}

#[test]
fn jumping_up_through_a_platform_from_below() {
    let mut sim = sim_in(PLATFORM);
    sim.run(30)
        .press(Key::Down)
        .tap(Key::Jump)
        .release(Key::Down);
    sim.run(60);
    assert!((feet(&sim) - 5.0).abs() < 0.01, "dropped to the floor");

    // A full jump from the floor passes through the platform and lands on it.
    sim.press(Key::Jump).run(15).release(Key::Jump).run(90);
    assert!(sim.player().is_on_ground());
    assert!(
        (feet(&sim) - 3.0).abs() < 0.01,
        "landed on the platform, feet {}",
        feet(&sim)
    );
}

#[test]
fn down_and_jump_drops_through_a_platform() {
    let mut sim = sim_in(PLATFORM);
    sim.run(30).clear_sounds();
    sim.press(Key::Down)
        .tap(Key::Jump)
        .release(Key::Down)
        .run(60);
    assert!((feet(&sim) - 5.0).abs() < 0.01, "feet {}", feet(&sim));
    assert!(
        !sim.sounds().contains(&Sound::Jump),
        "dropping is not a jump"
    );
}

const LEDGE_WALL: &[&str] = &[
    "#########",
    "#.......#",
    "#.......#",
    "#.....###",
    "#.....###",
    "#S....###",
    "#########",
];

#[test]
fn grabbing_a_ledge_and_jumping_up_from_it() {
    let mut sim = sim_in(LEDGE_WALL);
    sim.run(30).press(Key::Right);
    let at_wall = sim.run_until(120, |g| g.player.bb.x + g.player.bb.w > 5.99);
    assert!(at_wall.is_some(), "never reached the wall");
    // Jump up along the wall while holding right.
    sim.press(Key::Jump).run(15).release(Key::Jump);
    let hung = sim.run_until(120, |g| {
        matches!(g.player.state(), PlayerState::Hanging { .. })
    });
    assert!(hung.is_some(), "never grabbed the ledge");
    // Hanging aligns the player's top with the top of the ledge tile (applied on
    // the tick after the grab).
    sim.run(1);
    assert_eq!(sim.player().bb.y, 3.0);

    sim.release(Key::Right).run(30);
    assert!(matches!(sim.player().state(), PlayerState::Hanging { .. }));

    sim.clear_sounds().tap(Key::Jump);
    assert!(matches!(sim.player().state(), PlayerState::Normal));
    assert!(sim.sounds().contains(&Sound::Jump));
}
