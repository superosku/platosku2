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
use crate::state::game_state::{Game, GameBuilder};
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

/// Everything needed to replay a simulation exactly: the game setup and the
/// input of every tick. The game is deterministic, so replaying gives the same
/// run, frame for frame.
#[derive(Clone)]
pub struct Recording {
    pub setup: GameBuilder,
    /// The input for each tick, in order.
    pub inputs: Vec<InputState>,
    /// The player's position at the end, to check that a replay matched.
    pub final_player_pos: (f32, f32),
}

thread_local! {
    static RECORDING_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FINISHED_RECORDINGS: std::cell::RefCell<Vec<Recording>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Turns recording on or off for sims created on this thread from now on. A
/// recorded sim keeps its setup and inputs, and hands them over when it is
/// dropped (also when a test panics), see [`take_recordings`].
pub fn set_recording(enabled: bool) {
    RECORDING_ENABLED.with(|r| r.set(enabled));
}

/// Returns (and clears) the recordings of sims dropped on this thread.
pub fn take_recordings() -> Vec<Recording> {
    FINISHED_RECORDINGS.with(|r| std::mem::take(&mut *r.borrow_mut()))
}

pub struct Sim {
    pub game: Game,
    input: InputState,
    sound: RecordingSound,
    tick: u64,
    recording: Option<Recording>,
}

impl Sim {
    /// A sim for an already built game. It can't be recorded, because the setup
    /// is unknown; prefer [`Sim::from_builder`].
    pub fn new(game: Game) -> Sim {
        Sim {
            game,
            input: InputState::default(),
            sound: RecordingSound::default(),
            tick: 0,
            recording: None,
        }
    }

    /// A sim for the game described by `setup`. Recorded if recording is
    /// enabled on this thread (see [`set_recording`]).
    pub fn from_builder(setup: GameBuilder) -> Sim {
        let mut sim = Sim::new(setup.clone().build());
        if RECORDING_ENABLED.with(|r| r.get()) {
            sim.recording = Some(Recording {
                setup,
                inputs: Vec::new(),
                final_player_pos: (0.0, 0.0),
            });
        }
        sim
    }

    /// Sets the whole input state for the next tick (used for replays).
    pub fn set_input(&mut self, input: InputState) -> &mut Self {
        self.input = input;
        self
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
        if let Some(recording) = &mut self.recording {
            recording.inputs.push(self.input.clone());
        }
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

    /// Notes of the `play_note` calls for `sound` so far, in order.
    pub fn notes(&self, sound: Sound) -> Vec<u32> {
        self.sound
            .requests()
            .iter()
            .filter(|r| r.sound == sound)
            .filter_map(|r| r.note)
            .collect()
    }

    pub fn clear_sounds(&mut self) -> &mut Self {
        self.sound.clear();
        self
    }
}

impl Drop for Sim {
    fn drop(&mut self) {
        if let Some(mut recording) = self.recording.take() {
            recording.final_player_pos = (self.game.player.bb.x, self.game.player.bb.y);
            FINISHED_RECORDINGS.with(|r| r.borrow_mut().push(recording));
        }
    }
}

impl Recording {
    /// Replays the recording in a new sim; `on_tick` is called after every tick.
    /// Returns the sim at the end.
    pub fn replay(&self, mut on_tick: impl FnMut(&Sim)) -> Sim {
        let mut sim = Sim::new(self.setup.clone().build());
        for input in &self.inputs {
            sim.set_input(input.clone()).step();
            on_tick(&sim);
        }
        sim
    }
}
