//! Simple text scripts that set up a game and play scripted input.
//!
//! Used by the headless script runner (`cargo run --example run_script`), by
//! tests, and later by the screenshot capture mode.
//!
//! # Format
//!
//! ```text
//! // Comments start with //. Blank lines are ignored.
//! seed 42                          // RNG seed (default 0)
//! items off                        // random items on the map: on (default) | off
//! map random                       // random map from rooms/ (default)
//! map room rooms/room_0008.json    // a single room loaded from a file
//! map ascii                        // a single room as ASCII art (see Room::from_ascii)
//! #######
//! #S....#
//! #######
//! end
//! player 3 5                       // player start position (default: the start door)
//!
//! 30 right          // hold exactly these keys for 30 ticks (1 tick = 1/60 s)
//! 1 right jump      // keys: left right up down jump swing
//! 15 right jump     // jump/swing register a press only when they become held
//! 60                // no keys
//! snap after_jump   // a named marker (runner prints state / capture takes a screenshot)
//! ```

use crate::sim::{Key, Sim};
use crate::state::game_state::{Game, MapSetup};
use crate::state::map_like::Room;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Hold exactly `keys` for `ticks` ticks.
    Hold { ticks: u32, keys: Vec<Key> },
    /// A named marker between ticks.
    Snap(String),
}

#[derive(Debug, Clone)]
pub enum ScriptMap {
    Random,
    RoomFile(String),
    Ascii(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Script {
    pub seed: u64,
    pub random_items: bool,
    pub map: ScriptMap,
    pub player_pos: Option<(f32, f32)>,
    pub steps: Vec<Step>,
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut script = Script {
            seed: 0,
            random_items: true,
            map: ScriptMap::Random,
            player_pos: None,
            steps: Vec::new(),
        };

        let mut lines = text.lines().enumerate();
        while let Some((index, raw_line)) = lines.next() {
            let line_no = index + 1;
            let err = |msg: String| format!("line {line_no}: {msg}");

            let line = match raw_line.find("//") {
                Some(i) => &raw_line[..i],
                None => raw_line,
            }
            .trim();
            if line.is_empty() {
                continue;
            }

            let mut words = line.split_whitespace();
            let command = words.next().unwrap();
            let args: Vec<&str> = words.collect();

            match command {
                "seed" => {
                    let [seed] = args[..] else {
                        return Err(err("usage: seed <number>".into()));
                    };
                    script.seed = seed
                        .parse()
                        .map_err(|_| err(format!("bad seed {seed:?}")))?;
                }
                "items" => {
                    script.random_items = match args[..] {
                        ["on"] => true,
                        ["off"] => false,
                        _ => return Err(err("usage: items on|off".into())),
                    };
                }
                "player" => {
                    let [x, y] = args[..] else {
                        return Err(err("usage: player <x> <y>".into()));
                    };
                    let parse = |v: &str| {
                        v.parse::<f32>()
                            .map_err(|_| err(format!("bad number {v:?}")))
                    };
                    script.player_pos = Some((parse(x)?, parse(y)?));
                }
                "map" => match args[..] {
                    ["random"] => script.map = ScriptMap::Random,
                    ["room", path] => script.map = ScriptMap::RoomFile(path.to_string()),
                    ["ascii"] => {
                        // Read raw lines (no comment stripping: '#' and '/' are tiles) until "end".
                        let mut rows = Vec::new();
                        loop {
                            match lines.next() {
                                Some((_, row)) if row.trim() == "end" => break,
                                Some((_, row)) => rows.push(row.to_string()),
                                None => return Err(err("map ascii without a matching end".into())),
                            }
                        }
                        script.map = ScriptMap::Ascii(rows);
                    }
                    _ => return Err(err("usage: map random | map room <file> | map ascii".into())),
                },
                "snap" => {
                    let [name] = args[..] else {
                        return Err(err("usage: snap <name>".into()));
                    };
                    script.steps.push(Step::Snap(name.to_string()));
                }
                ticks if ticks.chars().all(|c| c.is_ascii_digit()) => {
                    let ticks: u32 = ticks
                        .parse()
                        .map_err(|_| err(format!("bad tick count {ticks:?}")))?;
                    let keys = args
                        .iter()
                        .map(|k| parse_key(k).ok_or_else(|| err(format!("unknown key {k:?}"))))
                        .collect::<Result<Vec<Key>, String>>()?;
                    script.steps.push(Step::Hold { ticks, keys });
                }
                other => return Err(err(format!("unknown command {other:?}"))),
            }
        }

        Ok(script)
    }

    pub fn load(path: &str) -> Result<Script, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Script::parse(&text).map_err(|e| format!("{path}: {e}"))
    }

    /// Creates the game described by the script's header.
    pub fn build_game(&self) -> Result<Game, String> {
        let map = match &self.map {
            ScriptMap::Random => MapSetup::RandomFromFolder,
            ScriptMap::RoomFile(path) => {
                let room = Room::load_json(path).map_err(|e| format!("{path}: {e}"))?;
                MapSetup::SingleRoom(Box::new(room))
            }
            ScriptMap::Ascii(rows) => {
                let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
                MapSetup::SingleRoom(Box::new(Room::from_ascii(0, 0, &rows)))
            }
        };
        let mut builder = Game::builder()
            .seed(self.seed)
            .map(map)
            .random_items(self.random_items);
        if let Some((x, y)) = self.player_pos {
            builder = builder.player_pos(x, y);
        }
        Ok(builder.build())
    }

    /// Total number of ticks the script runs.
    pub fn total_ticks(&self) -> u64 {
        self.steps
            .iter()
            .map(|s| match s {
                Step::Hold { ticks, .. } => *ticks as u64,
                Step::Snap(_) => 0,
            })
            .sum()
    }
}

fn parse_key(word: &str) -> Option<Key> {
    Some(match word {
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "jump" => Key::Jump,
        "swing" => Key::Swing,
        _ => return None,
    })
}

const ALL_KEYS: [Key; 6] = [
    Key::Left,
    Key::Right,
    Key::Up,
    Key::Down,
    Key::Jump,
    Key::Swing,
];

impl Sim {
    /// Makes exactly `keys` held: presses newly held keys and releases the rest.
    pub fn set_keys(&mut self, keys: &[Key]) -> &mut Self {
        for key in ALL_KEYS {
            if keys.contains(&key) {
                self.press(key);
            } else {
                self.release(key);
            }
        }
        self
    }

    /// Plays the script's steps. `on_snap` is called at every `snap` marker.
    pub fn run_script(&mut self, script: &Script, mut on_snap: impl FnMut(&Sim, &str)) {
        for step in &script.steps {
            match step {
                Step::Hold { ticks, keys } => {
                    self.set_keys(keys).run(*ticks);
                }
                Step::Snap(name) => on_snap(self, name),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_steps_and_ascii_map() {
        let script = Script::parse(
            "// a comment\n\
             seed 7\n\
             items off\n\
             player 2 1.5\n\
             map ascii\n\
             #####\n\
             #S..#\n\
             #####\n\
             end\n\
             \n\
             30 right   // walk\n\
             1 right jump\n\
             snap peak\n\
             10\n",
        )
        .unwrap();
        assert_eq!(script.seed, 7);
        assert!(!script.random_items);
        assert_eq!(script.player_pos, Some((2.0, 1.5)));
        assert!(
            matches!(&script.map, ScriptMap::Ascii(rows) if rows.len() == 3 && rows[1] == "#S..#")
        );
        assert_eq!(
            script.steps,
            vec![
                Step::Hold {
                    ticks: 30,
                    keys: vec![Key::Right]
                },
                Step::Hold {
                    ticks: 1,
                    keys: vec![Key::Right, Key::Jump]
                },
                Step::Snap("peak".into()),
                Step::Hold {
                    ticks: 10,
                    keys: vec![]
                },
            ]
        );
        assert_eq!(script.total_ticks(), 41);
    }

    #[test]
    fn reports_errors_with_line_numbers() {
        let err = Script::parse("seed 1\n10 rigth\n").unwrap_err();
        assert!(err.contains("line 2") && err.contains("rigth"), "{err}");
        assert!(
            Script::parse("map ascii\n###\n")
                .unwrap_err()
                .contains("end")
        );
        assert!(
            Script::parse("fly 10")
                .unwrap_err()
                .contains("unknown command")
        );
    }

    #[test]
    fn held_jump_registers_one_press() {
        let script = Script::parse(
            "items off\nmap ascii\n#######\n#.....#\n#.....#\n#S....#\n#######\nend\n\
             30\n20 jump\n60 jump\n",
        )
        .unwrap();
        let mut sim = Sim::new(script.build_game().unwrap());
        sim.run_script(&script, |_, _| {});
        let jumps = sim
            .sounds()
            .iter()
            .filter(|s| **s == crate::sound_handler::Sound::Jump)
            .count();
        assert_eq!(jumps, 1);
    }
}
