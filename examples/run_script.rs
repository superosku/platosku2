//! Runs an input script headlessly and prints the game state at every `snap`
//! marker and at the end.
//!
//!     cargo run --example run_script -- scripts/jump_demo.txt
//!
//! See `src/script.rs` for the script format.

use platosku2::script::Script;
use platosku2::sim::Sim;

fn print_state(sim: &Sim, label: &str, sounds_seen: &mut usize) {
    let p = sim.player();
    let sounds = sim.sounds();
    let new_sounds = &sounds[*sounds_seen..];
    *sounds_seen = sounds.len();
    println!(
        "[{label}] tick {}: player x {:.3} y {:.3} vx {:.3} vy {:.3} {:?}{} health {}/{} room {:?} | enemies {} items {} | sounds {:?}",
        sim.tick(),
        p.bb.x,
        p.bb.y,
        p.bb.vx,
        p.bb.vy,
        p.state(),
        if p.is_on_ground() { " on_ground" } else { "" },
        p.health.current,
        p.health.max,
        sim.game.current_room_index(),
        sim.game.enemies.len(),
        sim.game.items.len(),
        new_sounds,
    );
}

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run --example run_script -- <script.txt>");
        std::process::exit(2);
    };
    let script = Script::load(&path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    let game = script.build_game().unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let mut sim = Sim::new(game);
    let mut sounds_seen = 0;
    print_state(&sim, "init", &mut sounds_seen);
    sim.run_script(&script, |sim, name| {
        print_state(sim, name, &mut sounds_seen)
    });
    print_state(&sim, "end", &mut sounds_seen);
}
