//! Headless simulation harness for end-to-end tests.
//!
//! A [`Sim`] wraps a [`Game`] and drives it tick by tick with scripted input, the
//! same way the real game loop in `main.rs` does, but without a window or audio.
//!
//! ```no_run
//! use platosku2::sim::{Key, Sim};
//! use platosku2::state::game_state::Game;
//! use platosku2::state::map_like::Room;
//!
//! let room = Room::from_ascii(0, 0, &[
//!     "#######",
//!     "#S....#",
//!     "#######",
//! ]);
//! let mut sim = Sim::new(Game::builder().single_room(room).random_items(false).build());
//! sim.press(Key::Right).run(30);
//! assert!(sim.player().bb.x > 1.5);
//! ```

use crate::sound_handler::{RecordingSound, Sound};
use crate::state::game_state::Game;
use crate::state::player::Player;
use crate::state::{GameState, InputState};

/// The game's input buttons (see `main.rs` for the keyboard mapping).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    /// Z on the keyboard.
    Jump,
    /// X on the keyboard.
    Swing,
}

pub struct Sim {
    pub game: Game,
    input: InputState,
    sound: RecordingSound,
    tick: u64,
}

impl Sim {
    pub fn new(game: Game) -> Sim {
        Sim {
            game,
            input: InputState::default(),
            sound: RecordingSound::default(),
            tick: 0,
        }
    }

    /// Key down. Jump and Swing also register a "pressed" edge for the next tick,
    /// unless the key is already held (like keyboard auto-repeat in the game).
    pub fn press(&mut self, key: Key) -> &mut Self {
        let input = &mut self.input;
        match key {
            Key::Left => input.left = true,
            Key::Right => input.right = true,
            Key::Up => input.up = true,
            Key::Down => input.down = true,
            Key::Jump => {
                if !input.jump_held {
                    input.jump_pressed = true;
                }
                input.jump_held = true;
            }
            Key::Swing => {
                if !input.swing_held {
                    input.swing_pressed = true;
                }
                input.swing_held = true;
            }
        }
        self
    }

    /// Key up.
    pub fn release(&mut self, key: Key) -> &mut Self {
        let input = &mut self.input;
        match key {
            Key::Left => input.left = false,
            Key::Right => input.right = false,
            Key::Up => input.up = false,
            Key::Down => input.down = false,
            Key::Jump => input.jump_held = false,
            Key::Swing => input.swing_held = false,
        }
        self
    }

    /// Releases every key.
    pub fn release_all(&mut self) -> &mut Self {
        self.input = InputState::default();
        self
    }

    /// Presses the key for exactly one tick, then releases it.
    pub fn tap(&mut self, key: Key) -> &mut Self {
        self.press(key).run(1).release(key)
    }

    /// Runs `ticks` game updates (1 tick = 1/60 s) with the current input.
    pub fn run(&mut self, ticks: u32) -> &mut Self {
        for _ in 0..ticks {
            self.step();
        }
        self
    }

    /// Runs until `condition` is true, for at most `max_ticks` ticks. Returns the
    /// number of ticks it took, or `None` if the condition never became true.
    /// The condition is checked before each tick, so `Some(0)` means it was
    /// already true.
    pub fn run_until(&mut self, max_ticks: u32, condition: impl Fn(&Game) -> bool) -> Option<u32> {
        for ticks in 0..=max_ticks {
            if condition(&self.game) {
                return Some(ticks);
            }
            if ticks < max_ticks {
                self.step();
            }
        }
        None
    }

    /// Runs `ticks` updates and calls `observe` after each one. Useful for
    /// measuring things like the highest point of a jump.
    pub fn run_observing(&mut self, ticks: u32, mut observe: impl FnMut(&Game)) -> &mut Self {
        for _ in 0..ticks {
            self.step();
            observe(&self.game);
        }
        self
    }

    fn step(&mut self) {
        self.game.update(&self.input, &self.sound);
        // Same as the game loop in main.rs: "pressed" edges last a single tick.
        self.input.jump_pressed = false;
        self.input.swing_pressed = false;
        self.tick += 1;
    }

    pub fn player(&self) -> &Player {
        &self.game.player
    }

    /// Total number of ticks simulated so far.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// All sounds requested so far, in order.
    pub fn sounds(&self) -> Vec<Sound> {
        self.sound.played()
    }

    pub fn clear_sounds(&mut self) -> &mut Self {
        self.sound.clear();
        self
    }
}
