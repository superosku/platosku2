//! End-to-end tests for interactions between the player, enemies, items and rooms.

use crate::common::*;
use platosku2::sim::{Key, Sim};
use platosku2::sound_handler::Sound;
use platosku2::state::game_state::{COIN_STREAK_TICKS, Game, MapSetup};
use platosku2::state::item::{ItemType, SPAWNED_COIN_PICKUP_DELAY};
use platosku2::state::map_like::Room;

fn slime_health(sim: &Sim) -> u32 {
    assert_eq!(sim.game.enemies.len(), 1);
    sim.game.enemies[0].get_health().current
}

fn touching_a_slime_hurts_and_gives_immunity_frames() {
    let mut sim = sim_in(&["##########", "#........#", "#S....s..#", "##########"]);
    assert_eq!(sim.player().health.current, 4);

    sim.press(Key::Right);
    let hit = sim.run_until(180, |g| g.player.health.current < 4);
    assert!(hit.is_some(), "the slime never hurt the player");
    assert_eq!(
        sim.player().health.current,
        3,
        "an idle slime does 1 damage"
    );
    assert!(!sim.player().can_be_hit());
    assert!(sim.sounds().contains(&Sound::PlayerHit));

    // Still touching the slime, but immune for a while.
    sim.release(Key::Right).run(30);
    assert_eq!(sim.player().health.current, 3);
}

fn stomping_a_slime_damages_it_and_bounces_the_player() {
    let mut sim = sim_in(&[
        "#######", //
        "#..S..#", //
        "#.....#", //
        "#.....#", //
        "#.....#", //
        "#..s..#", //
        "#######", //
    ]);
    assert_eq!(slime_health(&sim), 2);

    let stomped = sim.run_until(120, |g| g.enemies[0].get_health().current < 2);
    assert!(stomped.is_some(), "the player never stomped the slime");
    assert!(sim.player().bb.vy < 0.0, "the player should bounce up");
    assert_eq!(sim.player().health.current, 4, "stomping must not hurt");
    let sounds = sim.sounds();
    assert!(sounds.contains(&Sound::EnemyHit) && sounds.contains(&Sound::Clink));
}

fn stomping_a_slime_twice_kills_it() {
    let mut sim = sim_in(&[
        "#######", //
        "#..S..#", //
        "#.....#", //
        "#.....#", //
        "#.....#", //
        "#..s..#", //
        "#######", //
    ]);
    // Keep bouncing on the slime; it has 2 health and 30 immunity frames.
    let killed = sim.run_until(600, |g| g.enemies.is_empty());
    assert!(killed.is_some(), "the slime should die after two stomps");
}

const BOX_ROOM: &[&str] = &[
    "##########", //
    "#........#", //
    "#S.......#", //
    "##########", //
];

const BOX_ROOM_TALL: &[&str] = &[
    "##########", //
    "#........#", //
    "#........#", //
    "#........#", //
    "#S.......#", //
    "##########", //
];

fn the_player_starts_with_a_box_and_can_throw_it() {
    let mut sim = sim_in(BOX_ROOM);
    sim.run(30);
    assert_eq!(
        sim.player().item().map(|i| i.item_type()),
        Some(ItemType::Box)
    );

    sim.clear_sounds().tap(Key::Swing);
    assert!(sim.player().item().is_none());
    assert_eq!(sim.game.items.len(), 1);
    assert!(
        sim.game.items[0].bb().vx > 0.0,
        "thrown forwards (facing right)"
    );
    assert_eq!(sim.sounds(), vec![Sound::Throw]);

    sim.run(120);
    let item_bb = *sim.game.items[0].bb();
    assert!(
        item_bb.x > sim.player().bb.x + 1.0,
        "box landed ahead of the player"
    );
}

fn dropping_and_picking_up_the_box() {
    let mut sim = sim_in(BOX_ROOM);
    sim.run(30);
    sim.press(Key::Down)
        .tap(Key::Swing)
        .release(Key::Down)
        .run(30);
    assert!(sim.player().item().is_none());
    assert_eq!(sim.game.items.len(), 1);
    assert_eq!(sim.game.items[0].bb().vx, 0.0, "dropped, not thrown");

    sim.press(Key::Down).tap(Key::Swing).release(Key::Down);
    assert_eq!(
        sim.player().item().map(|i| i.item_type()),
        Some(ItemType::Box)
    );
    assert!(sim.game.items.is_empty());
}

fn swinging_at_a_box_breaks_it_into_coins_that_can_be_collected() {
    let mut sim = sim_in(BOX_ROOM);
    sim.run(30);
    // Drop the box at our feet, then swing at it.
    sim.press(Key::Down)
        .tap(Key::Swing)
        .release(Key::Down)
        .run(30);
    sim.clear_sounds().tap(Key::Swing).run(30);

    assert!(
        !sim.game
            .items
            .iter()
            .any(|i| i.item_type() == ItemType::Box),
        "the box should be broken"
    );
    let sounds = sim.sounds();
    assert!(sounds.contains(&Sound::Swing));
    assert!(
        sounds.contains(&Sound::Clink),
        "breaking the box makes a clink"
    );

    // Coins pop out of the box, which is where the player stands, so they are
    // collected when they fall back down or after a short walk.
    sim.press(Key::Left)
        .run(30)
        .press(Key::Right)
        .release(Key::Left)
        .run(30);
    let coins_left = sim
        .game
        .items
        .iter()
        .filter(|i| i.item_type() == ItemType::Coin)
        .count();
    let collected = sim
        .sounds()
        .iter()
        .filter(|s| **s == Sound::CollectCoin)
        .count();
    assert!(collected >= 1, "should have collected at least one coin");
    assert!(
        (3..=5).contains(&(collected + coins_left)),
        "a box drops 3-5 coins"
    );
}

/// Breaks the player's box, stands still for `ticks` ticks and returns the
/// coins collected as (ticks after the break, note of the coin sound).
fn break_box_and_record_collections(sim: &mut Sim, ticks: u32) -> Vec<(u32, u32)> {
    sim.run(30);
    sim.press(Key::Down)
        .tap(Key::Swing)
        .release(Key::Down)
        .run(30);
    sim.clear_sounds().tap(Key::Swing);
    assert_eq!(sim.sounds(), vec![Sound::Swing, Sound::Clink], "box broke");
    sim.clear_sounds();

    let mut collected = Vec::new();
    for t in 1..=ticks {
        sim.run(1);
        let notes = sim.notes(Sound::CollectCoin);
        for note in &notes[collected.len()..] {
            collected.push((t, *note));
        }
    }
    collected
}

fn coins_from_a_broken_box_pop_up_and_are_collected_one_at_a_time() {
    for seed in 0..8 {
        let mut sim = sim_in_seeded(BOX_ROOM, seed);
        let collected: Vec<u32> = break_box_and_record_collections(&mut sim, 120)
            .iter()
            .map(|(tick, _)| *tick)
            .collect();
        assert!(!collected.is_empty(), "seed {seed}: no coins collected");

        // Nothing is collected while the coins can't be picked up yet.
        assert!(
            collected[0] >= SPAWNED_COIN_PICKUP_DELAY,
            "seed {seed}: coin collected too early, at tick {}",
            collected[0]
        );
        // Each coin gets its own tick, so the sounds don't pile up.
        assert!(
            collected.windows(2).all(|w| w[0] != w[1]),
            "seed {seed}: several coins collected on one tick: {collected:?}"
        );
    }
}

fn coins_collected_quickly_play_rising_notes() {
    let mut longest = 0;
    for seed in 0..8 {
        let mut sim = sim_in_seeded(BOX_ROOM, seed);
        let collected = break_box_and_record_collections(&mut sim, 120);
        assert_eq!(collected[0].1, 0, "seed {seed}: a streak starts at note 0");
        for pair in collected.windows(2) {
            let ((prev_tick, prev_note), (tick, note)) = (pair[0], pair[1]);
            let expected = if tick - prev_tick <= COIN_STREAK_TICKS {
                prev_note + 1
            } else {
                0
            };
            assert_eq!(note, expected, "seed {seed}: {collected:?}");
        }
        longest = longest.max(collected.len());
    }
    assert!(longest >= 3, "some box should give a streak of 3 coins");
}

fn the_coin_streak_starts_over_after_a_pause() {
    // With this seed, some coins are collected right away (a streak) and one
    // lands out of reach.
    let mut sim = sim_in_seeded(BOX_ROOM, 0);
    let collected = break_box_and_record_collections(&mut sim, 120);
    let streak: Vec<u32> = collected.iter().map(|c| c.1).collect();
    assert!(streak.len() >= 2, "a streak first: {streak:?}");
    assert_eq!(sim.game.items.len(), 1, "one coin left");

    // Walk to the last coin, long after the streak ended.
    let key = if sim.game.items[0].bb().x > sim.player().bb.x {
        Key::Right
    } else {
        Key::Left
    };
    sim.press(key);
    sim.run_until(120, |g| g.items.is_empty())
        .expect("should reach the last coin");
    let mut expected = streak;
    expected.push(0);
    assert_eq!(sim.notes(Sound::CollectCoin), expected);
}

fn coins_from_a_broken_box_pop_up_about_half_a_tile() {
    for seed in 0..8 {
        let mut sim = sim_in_seeded(BOX_ROOM_TALL, seed);
        sim.run(30);
        sim.press(Key::Down)
            .tap(Key::Swing)
            .release(Key::Down)
            .run(30);
        let floor = sim.game.items[0].bb().y + sim.game.items[0].bb().h;
        sim.tap(Key::Swing);

        // The highest any coin's bottom gets above the floor.
        let mut peak = 0.0f32;
        sim.run_observing(40, |g| {
            for item in &g.items {
                peak = peak.max(floor - (item.bb().y + item.bb().h));
            }
        });
        // Measured 0.40-0.56 tiles over these seeds.
        assert!(
            (0.3..=0.7).contains(&peak),
            "seed {seed}: coins should pop up about half a tile, peaked at {peak}"
        );
    }
}

/// Two rooms connected by a door: A (with the start) on the left, B on the right.
fn two_room_game(room_b: &[&str]) -> Sim {
    let a = Room::from_ascii(
        0,
        0,
        &[
            "########", //
            "#......#", //
            "#S.....>", //
            "########", //
        ],
    );
    let b = Room::from_ascii(0, 0, room_b);
    let sim = Sim::from_builder(
        Game::builder()
            .map(MapSetup::Random(vec![a, b]))
            .random_items(false),
    );
    assert_eq!(sim.game.map.rooms.len(), 2, "the two rooms should connect");
    sim
}

fn walking_through_a_door_changes_the_room() {
    let mut sim = two_room_game(&[
        "########", //
        "#......#", //
        "<......#", //
        "########", //
    ]);
    sim.run(1);
    assert_eq!(sim.game.current_room_index(), Some(0));

    sim.press(Key::Right);
    let changed = sim.run_until(300, |g| g.current_room_index() == Some(1));
    assert!(changed.is_some(), "never reached the second room");
    // Room B starts at the shared door column x = 7. Keep walking into it.
    sim.run(60);
    assert!(sim.player().bb.x > 9.0, "x = {}", sim.player().bb.x);
    assert_eq!(sim.game.current_room_index(), Some(1));
    assert!(sim.game.visited_rooms.contains(&0) && sim.game.visited_rooms.contains(&1));
}

fn enemies_in_other_rooms_are_frozen() {
    let mut sim = two_room_game(&[
        "########", //
        "#..b...#", //
        "<......#", //
        "########", //
    ]);
    let bat_start = *sim.game.enemies[0].bb();
    sim.run(120);
    let bat_now = *sim.game.enemies[0].bb();
    assert_eq!((bat_start.x, bat_start.y), (bat_now.x, bat_now.y));

    // Once the player enters the bat's room, it starts flying.
    sim.press(Key::Right);
    sim.run_until(300, |g| g.current_room_index() == Some(1))
        .expect("never reached the second room");
    sim.release(Key::Right).run(30);
    let bat_later = *sim.game.enemies[0].bb();
    assert_ne!((bat_start.x, bat_start.y), (bat_later.x, bat_later.y));
}

/// The tests in this file, registered with the harness in main.rs.
pub fn tests() -> Vec<(&'static str, fn())> {
    crate::tests![
        touching_a_slime_hurts_and_gives_immunity_frames,
        stomping_a_slime_damages_it_and_bounces_the_player,
        stomping_a_slime_twice_kills_it,
        the_player_starts_with_a_box_and_can_throw_it,
        dropping_and_picking_up_the_box,
        swinging_at_a_box_breaks_it_into_coins_that_can_be_collected,
        coins_from_a_broken_box_pop_up_and_are_collected_one_at_a_time,
        coins_from_a_broken_box_pop_up_about_half_a_tile,
        coins_collected_quickly_play_rising_notes,
        the_coin_streak_starts_over_after_a_pause,
        walking_through_a_door_changes_the_room,
        enemies_in_other_rooms_are_frozen,
    ]
}
