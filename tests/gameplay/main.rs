//! Gameplay tests: the whole game simulated tick by tick with scripted input
//! (see `platosku2::sim::Sim`).
//!
//! This test target has its own `main` (`harness = false` in Cargo.toml) so that
//! it can open a window on the main thread for watching the tests:
//!
//!     cargo test --test gameplay                          # headless, like in CI
//!     cargo test --test gameplay -- ledge                 # only tests matching "ledge"
//!     cargo test --test gameplay -- --watch               # run, then replay in a window (4x speed)
//!     cargo test --test gameplay -- --watch slime --speed 1
//!
//! Watching works by recording every `Sim` a test creates (setup + input of each
//! tick) and replaying it afterwards; the game is deterministic, so the replay
//! is exactly what the test saw. Tests must therefore create their sims with
//! `Sim::from_builder` and change the game only through input.
//!
//! To add a test: write `fn my_test()` in a module and add it to that module's
//! `tests()` list. An unlisted test function is unused and fails clippy.

mod common;
mod interactions;
mod player;

use libtest_mimic::{Arguments, Failed, Trial};
use platosku2::sim;
use platosku2::watch::{WatchItem, watch};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

/// `tests![a, b]` makes a list of `("a", a as fn())`.
#[macro_export]
macro_rules! tests {
    ($($name:ident),* $(,)?) => {
        vec![$((stringify!($name), $name as fn())),*]
    };
}

/// A module's tests: `(name, test function)`.
type TestList = Vec<(&'static str, fn())>;

fn all_tests() -> Vec<(String, fn())> {
    let modules: [(&str, TestList); 2] = [
        ("player", player::tests()),
        ("interactions", interactions::tests()),
    ];
    modules
        .into_iter()
        .flat_map(|(module, tests)| {
            tests
                .into_iter()
                .map(move |(name, f)| (format!("{module}::{name}"), f))
        })
        .collect()
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "panic".to_string()
    }
}

/// Replays each recording and checks that it ends where the test ended.
fn check_replays(recordings: &[sim::Recording]) -> Option<String> {
    for (i, recording) in recordings.iter().enumerate() {
        let replay = recording.replay(|_| {});
        let p = &replay.game.player.bb;
        if (p.x, p.y) != recording.final_player_pos {
            return Some(format!(
                "replay of sim #{i} ended at ({}, {}) but the test ended at {:?}: \
                 the game is not deterministic, or the test changed the game directly \
                 instead of through input",
                p.x, p.y, recording.final_player_pos
            ));
        }
    }
    None
}

/// Our own options, removed before the rest goes to libtest-mimic.
struct WatchOptions {
    watch: bool,
    speed: f32,
}

fn take_watch_options(args: &mut Vec<String>) -> WatchOptions {
    let mut options = WatchOptions {
        watch: false,
        speed: 4.0,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--watch" => {
                options.watch = true;
                args.remove(i);
            }
            "--speed" => {
                args.remove(i);
                let value = (i < args.len()).then(|| args.remove(i));
                options.speed = value
                    .as_deref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| panic!("--speed needs a number, e.g. --speed 4"));
            }
            _ => i += 1,
        }
    }
    options
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let options = take_watch_options(&mut args);
    let arguments = Arguments::from_iter(args);

    // (test index, watch item), filled in by the tests that ran.
    let watched: Arc<Mutex<Vec<(usize, WatchItem)>>> = Arc::default();

    let trials = all_tests()
        .into_iter()
        .enumerate()
        .map(|(index, (name, test))| {
            let watched = watched.clone();
            let watching = options.watch;
            Trial::test(name.clone(), move || {
                // Always record: every recording is replayed right away to check
                // that replays (and so --watch) show exactly what the test did.
                sim::set_recording(true);
                let result = catch_unwind(AssertUnwindSafe(test));
                // Sims are dropped by now (also on panic), so their recordings are in.
                let recordings = sim::take_recordings();
                sim::set_recording(false);

                let mut failure = result.as_ref().err().map(|p| panic_message(p.as_ref()));
                if failure.is_none() {
                    failure = check_replays(&recordings);
                }
                if watching {
                    watched.lock().unwrap().push((
                        index,
                        WatchItem {
                            name,
                            failure: failure.clone(),
                            recordings,
                        },
                    ));
                }
                match failure {
                    None => Ok(()),
                    Some(msg) => Err(Failed::from(msg)),
                }
            })
        })
        .collect();

    let conclusion = libtest_mimic::run(&arguments, trials);

    if options.watch {
        let mut items = std::mem::take(&mut *watched.lock().unwrap());
        items.sort_by_key(|(index, _)| *index);
        watch(
            items.into_iter().map(|(_, item)| item).collect(),
            options.speed,
        );
    }

    conclusion.exit();
}
