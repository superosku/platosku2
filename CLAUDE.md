# Platosku 2

A 2D platformer prototype written in Rust (edition 2024). It is a hobby project, somewhere between tinkering for fun and a small finished game. The game is built on **miniquad** (window, GL and input), with **egui** via egui-miniquad for the debug and editor UI and **quad-snd** for audio.

## Commands

```sh
cargo run                                   # run the game (800x600 window); must run from repo root (assets are loaded by relative path)
cargo test --all-targets                    # tests
cargo fmt --all -- --check                  # CI checks formatting
cargo clippy --all-targets -- -D warnings   # CI fails on ANY clippy warning
```

CI (`.github/workflows/`) runs check, fmt, clippy (with `-D warnings`) and test on Ubuntu. Before you finish a change, run fmt, clippy and test the same way CI does.

The game prints `Game seed: N` at startup, FPS and UPS every second, and a lot of map generator output (`Iterating for adding a room ...`). That output is normal. Use `Game::new_with_seed(N)` to reproduce a run.

Put temporary files (logs, screenshots, scripts) in `target/claude/`, which is gitignored.

## Project layout
- `src/main.rs`: the game binary (window, input, debug UI). Everything else is in the library (`src/lib.rs`), so tests and tools can use it.
- `src/state/`: game logic (player, enemies, items, map, rooms). Must stay deterministic and free of rendering (see Determinism).
- `src/physics.rs`, `src/render/` (renderer, see Rendering), `src/minimap.rs`, `src/camera.rs`, `src/sound_handler.rs`, `src/atlas_info.rs`, `src/debug_menu/` (egui editor and debug UI).
- `src/sim.rs` (headless simulation for tests), `src/script.rs` (input scripts), `src/watch.rs` (replay window for gameplay tests).
- `tests/`: gameplay, content and script tests (see Testing). `examples/`: `run_script`, `capture`, `sprite_sheet` (see Tools).
- `rooms/*.json`: room content (made with the in-game editor). `scripts/*.txt`: example input scripts. `assets/`: sprites, atlas, sounds.

## Architecture

### Main loop (`src/main.rs`)
- `Stage` implements miniquad's `EventHandler`. It owns `state: Box<dyn FullGameState>` (`Game` or `Editor`), `InputState`, `Renderer`, `Camera`, `SoundHandler` and `DebugMenu`.
- **Fixed timestep:** `update()` accumulates real time and runs `state.update(...)` plus `state.update_camera(...)` at exactly 1/60 s per tick. Frame time is capped at 0.1 s. Rendering is uncapped and separate (`draw()`).
- `InputState` is a plain struct with booleans. Key events set `left/right/up/down/*_held`. The edge-triggered `jump_pressed` and `swing_pressed` are set on key-down and **cleared after each tick**.
- Keys: arrows move, **Z** jumps, **X** swings, throws or picks up (with Down: drop, throw down or pick up), **M** shows the big map.
- The egui "Debug" window switches between Game and Editor. Switching to Game calls `Game::new()`, which regenerates the map.

### State (`src/state/`)
- `GameState` trait (`game_state.rs`): `update(input, &dyn SoundPlayer)`, `update_camera`, accessors for player and map. It has no rendering dependencies. `FullGameState` = `GameState + DrawableGameState (render/game.rs, including the minimap) + GameStateDebugMenu (debug_menu/)`.
- **`UpdateCtx`** (`update_ctx.rs`): `{ sound: &dyn SoundPlayer, rng: &mut GameRng }`, passed to every entity update (player, enemies, item interactions). Add new cross-cutting services here instead of new parameters.
- **Constructing games:** `Game::new()` (random seed, printed), `Game::new_with_seed(seed)`, or `Game::builder().seed(..).single_room(room).player_pos(x, y).random_items(false).build()`. `MapSetup` = `RandomFromFolder` | `Random(Vec<Room>)` | `SingleRoom(..)`.
- `Game`: player, `GameMap`, `items: Vec<Item>`, `enemies: Vec<Box<dyn Enemy>>`, current and previous room index (for camera and room-transition "dark" effect), `visited_rooms`, `Minimap` (presentation state only, no GPU resources), and the `GameRng`. `Game::update` does all the interactions: door push-out, player update, items (physics, pickup, swing hits, projectile hits on enemies), enemies (only those in the current room are updated), stomp and contact damage, room change detection, doors.
- `Editor`: a single `Room` plus a player for testing the room.
- **Result-enum pattern:** entities don't mutate the world directly. They return `Vec<...Result>` (`PlayerUpdateResult`, `EnemyUpdateResult`, `ItemInteractionResult`) and `Game::update` applies them. Follow this pattern for new interactions.
- `player.rs`: `PlayerState` state machine (`Normal`, `Swinging`, `Hanging`, `OnLadder`, `Dead`). It includes coyote time (`safe_edge_frames`), variable jump hold (`max_jump_frames`), ledge hang, ladders and drop-through platforms (down+jump). `Health` + `immunity_frames`.
- `enemies/`: the `Enemy` trait (`common.rs`) and `Bat`, `Slime`, `Worm`, `Burrower`. Each has its own state enum and an `AnimationHandler`. To add an enemy: implement `Enemy`, add a variant to `ObjectTemplateType` and `ObjectTemplate::as_object/get_texture_index` (`map_like/common.rs`), add it to the editor's `EnemySelection`, and add a sprite plus a UI icon.
- `item.rs`: `Item` + `ItemType` (Coin, stones, Box, GreenProjectile). Boxes break into coins when swung; the coins pop up and can't be collected for `SPAWNED_COIN_PICKUP_DELAY` (20) ticks, so they're collected one by one. GreenProjectile is the burrower's attack (it spits one towards the player).
- `animation_handler.rs`: generic `AnimationHandler<T: AnimationConfig>`. A state maps to (start frame, end frame, ticks per frame, loop, reverse) in the atlas.
- `common.rs`: `BoundingBox` (position, size **and velocity** `vx/vy`), `Pos`, `Dir`, `Health`.

### Map (`src/state/map_like/`)
- `MapLike` trait: tile queries (`get_at`, `is_solid_at_tile`, `is_ladder_at`, `is_platform_at`, `overlaps_solid`, …). It is implemented by `Room` and `GameMap`.
- Tiles: `BaseTile` (NotPartOfRoom, Empty, Stone, Wood; everything except Empty is solid) + `OverlayTile` (Ladder, Platform, LadderPlatform, StartDoor).
- `Room` (`room.rs`): serde JSON in `rooms/room_XXXX.json`. It holds base and overlay tiles, its position, `doors: Vec<RoomDoor>`, `object_templates` (enemy spawns) and `disabled`. `Room::load_rooms_from_folder()` reads `rooms/`. The editor saves here too. **`disabled` is currently only used by the editor; the map generator ignores it.**
- `Room::from_ascii(x, y, &[rows])` builds a room from ASCII art (see its doc comment for the characters). Use it in tests.
- `GameMap::new_random(&rooms, rng)` (`game_map.rs`): starts from the **first** room that has a StartDoor, then randomly attaches rooms door to door (max 1000 attempts, stops at 10 rooms), rejecting overlaps. It then flattens everything into one tile grid (`rebuild_tile_cache`). `GameMap::from_single_room(room)` makes a one-room map. `MapDoor` is a door between rooms, with animation and temporary closing. Doors are currently **always open** (see the `TODO: TEMP` in `Game::update`).
- Coordinates: world units are **tiles** (1.0 = one tile = 16 px, `state::common::TILE_SIZE`). Positions are f32, tile indices i32, and y grows downward.

### Physics (`src/physics.rs`)
- `integrate_kinematic(map, bb, gravity)`: per-axis sweep (X then Y) with bisection against `overlaps_solid`. Returns a new bb plus `on_bottom/top/left/right`. GRAVITY = 0.007 tiles/tick², terminal velocity 0.9.
- `check_and_snap_platforms` (one-way platforms) and `check_and_snap_hang` (ledge grab).

### Rendering (`src/render/`, `src/minimap.rs`, `src/camera.rs`)
- **Game state describes, the renderer draws.** Entities fill a `DrawList` (`state/draw.rs`: `Sprite`, rectangles, rotated rectangles, health bars; coordinates in tiles) in their `draw(&self, out: &mut DrawList)` methods. Nothing in `state/` imports `render`. Use `Sprite::around(name, frame, &bb)` for the usual sprite with a 1 px border around a bounding box.
- `DrawableGameState` (`render/game.rs`): `draw_world` (doors, items, enemies; before the player), `draw_world_top` (after the player: enemy health bars), `dark_tiles` (which tiles the dark overlay covers) and `update_and_draw_minimap`.
- `Renderer::draw` (`render/mod.rs`) fixes the frame order: tiles, then the world `DrawList` (overlay tiles, `draw_world`, player, `draw_world_top`), then the dark overlay, then the HUD (player health bar, minimap). Everything in one stage is drawn **in submission order**, so later items are on top.
- `QuadBatch` (`render/batch.rs`) collects quads with per-vertex color and splits them into texture segments (one draw call per texture change). Colored rectangles use a 1x1 white texture. Indices are `u32`. `GpuMesh` grows (and frees) its buffers as needed; each buffer is uploaded at most once per frame.
- Two shaders (`render/shaders.rs`): **sprite** (texture times vertex color; also rectangles and the HUD) and **tile** (magenta pixels in `tilemap16.png` show a repeating 64x64 region of `tile_backgrounds.png`, anchored to the world). The atlas has no magenta pixels.
- Tiles use a **dual-grid** autotiling system (`render/tiles.rs`: `TileMesh::push_layer`, `DUAL_GRID_UV_TABLE`). All layers (`TileLayer`: background, not-part-of-room, stone, wood) go into one mesh, which is **cached**: it covers the visible area plus `CACHE_MARGIN` cells and is rebuilt only when the camera leaves that area or `MapLike::tile_revision()` changes. **Any new code that changes tiles must renew the revision** (`new_tile_revision()`; `Room` does it in its mutating methods, `GameMap` in `rebuild_tile_cache`). The revision comes from a process-wide counter and never affects game logic.
- The "dark" overlay hides rooms other than the current one and animates during room transitions (`DrawableGameState::dark_tiles`). It is rebuilt every frame.
- `Camera` owns the view math (`view`, `view_proj`, `screen_proj`, `visible_world_px`, `screen_to_tile`, using `glam`). The camera center is pixel-snapped. It lerps smoothly towards a target (`slowly_follow`); the zoom is a free value (pixel-perfect output is a README TODO).
- `Minimap`: built as a CPU pixel image and uploaded only when the room changes (`Renderer::set_minimap_image`, drawn with `draw_minimap_image`). It shows visited rooms and supports zooming.

### Sound (`src/sound_handler.rs`)
- `SoundPlayer` trait: `play(sound)` and `play_note(sound, note)` (the `note`-th note of a rising major scale, up to one octave; used for streaks). Implementations: `SoundHandler` (real audio), `NullSound` and `RecordingSound` (for tests; `played()` returns the requested sounds in order, `requests()` includes the notes, `Sim::notes(sound)` lists them).
- The `define_sounds!` macro maps the `Sound` enum to file names in `assets/sounds/src/<name>.wav`. At startup, `SoundHandler` decodes them (`hound`) and **resamples** each into 7 pitch variants in memory (a higher pitch is also shorter, like tape speed). `play()` picks a random variant at a random volume. Per-sound pitch and volume ranges, and which sounds get scale notes, are in `Sound::variation`. A file that fails to load prints `Sound not loaded: ...` and that sound is silent.
- **Same-tick stagger:** if the same sound is requested again in a tick, the extra ones are delayed 4 ticks each (`Stagger`, in order). `Stage` calls `SoundHandler::end_tick()` after every tick.
- **Coin streak:** coins don't play their own sound; `Game` counts a `CoinStreak` and calls `play_note(CollectCoin, n)`. A coin within `COIN_STREAK_TICKS` (30) of the previous one plays the next note; otherwise it starts again at 0.
- Sound files are third-party, used with permission and must not be redistributed (`assets/sounds/README.md`).

### Assets
- Sprites are edited as `assets/sprites/*.aseprite` and packed into `assets/atlas.png` + `assets/atlas.json` with the aseprite CLI (command in README.md). `AtlasInfo` keys frames by (name, frame index), parsed from `"<name> <n>.aseprite"`. A missing sprite logs "Sprite not found" and draws a 1x1 rect.
- `ui_sprite_creator.py` (PIL) cuts editor icons from the atlas into `assets/ui_sprites/`.
- aseprite and ffmpeg are not necessarily installed on the dev machine. Check before relying on them.

## Determinism (important)
Game logic must give identical results for the same seed and inputs, on every platform (macOS on ARM locally, Linux on x86 in CI).
- **Randomness:** only through `ctx.rng` / a `&mut GameRng` (ChaCha8). Never `rand::rng()` in game logic. The audio layer (`sound_handler.rs`: variant and volume choice, resampling math) is the exception, because it doesn't affect state.
- **Float math:** `+ - * /` and `sqrt` are fine. `sin`, `cos`, `atan2`, `powf`, `exp` and similar are platform-dependent and **banned by `clippy.toml`** (`disallowed-methods`). Use `libm::sinf` and friends, or avoid them (see `random_unit_vector` in `state/common.rs`). `render/` and `sound_handler.rs` are exempt.
- **No iterating `HashMap`/`HashSet`** in game logic; the order is random per run. Use `Vec` or `BTreeMap`/`BTreeSet`. Lookups and inserts are fine.
- **No wall-clock time** in game logic. Count ticks.

## Testing
Four kinds of tests, all headless and run in CI by `cargo test --all-targets`:

- **Unit tests** live next to the code (`#[cfg(test)] mod tests`).
- **Gameplay tests** (`tests/gameplay/`): the whole game simulated tick by tick with scripted input through `Sim` (`src/sim.rs`).
  ```rust
  let mut sim = sim_in(&["#####", "#S..#", "#####"]);   // tests/gameplay/common.rs
  sim.press(Key::Right).run(30);            // 1 tick = 1/60 s
  sim.tap(Key::Jump);                       // press for exactly one tick
  sim.run_until(120, |g| g.player.is_on_ground());
  assert!(sim.sounds().contains(&Sound::Jump));
  ```
  - `player.rs`: movement (walk, jump heights, coyote time, ladders, platforms, ledge hang). `interactions.rs`: enemies, items, doors and rooms.
  - **Custom harness** (`harness = false`, `tests/gameplay/main.rs`, using `libtest-mimic`). Tests are plain `fn`s with no `#[test]`; **register each new test in its module's `tests()` list** (an unlisted one is dead code and fails clippy). New modules go in `all_tests()` in `main.rs`.
  - Create sims with `Sim::from_builder(..)` (as `sim_in` does) and change the game **only through input**. Every sim is recorded and replayed after the test; if the replay ends elsewhere, the test fails. That keeps the game deterministic and makes `--watch` exact.
  - `#[ignore]` doesn't apply here. For a known, not-yet-fixed gameplay bug, leave the test out of `tests()` with a `// known bug: ...` comment and `#[allow(dead_code)]`.
  - Filter by name: `cargo test --test gameplay -- ledge`.
  - **Watch them:** `cargo test --test gameplay -- --watch [filter] [--speed N]` runs the tests, then replays them in a window (default 4x; Space pause, `.` step, `+`/`-` speed, R restart, N/P next/previous, Esc quit). Failed tests stop at the failure and show the message. This is for the user; it needs a display.
  - Test output isn't captured by this harness, so map-generation `println!`s show up in the output.
- **Content tests** (`tests/content.rs`): every room in `rooms/` is valid, and map generation works for 200 seeds. Run them after editing rooms.
- **Script tests** (`tests/scripts.rs`): every script in `scripts/` parses and runs.

Tips:
- Measure before asserting: write a throwaway probe test that prints values, then pick thresholds from real numbers. Delete the probe afterwards.
- For unit tests, a known, not-yet-fixed bug gets `#[ignore = "known bug: ..."]` (run them with `cargo test -- --ignored`).
- The game logic runs headless. Only `Renderer::new()` needs a window.

## Tools

### Input scripts (`src/script.rs`, `scripts/*.txt`)
Text files that set up a game (`seed`, `items on|off`, `map random | room <file> | ascii … end`, `player x y`) and play input: each line `<ticks> [keys…]` holds exactly those keys (`left right up down jump swing`). `snap <name>` marks a point. The format is documented at the top of `src/script.rs`.
```sh
cargo run --example run_script -- scripts/jump_demo.txt   # headless; prints player state at each snap
```
Use it to check a scenario quickly or reproduce a bug. In code: `Script::load(path)?.build_game()?`, then `sim.run_script(&script, |sim, snap_name| ...)`.

### Visual checks (screenshots and sprites)
You can look at PNGs with the Read tool. Use these to check rendering, UI and sprite changes:
```sh
cargo run --example capture -- scripts/jump_demo.txt            # PNG at every snap -> target/claude/shots/jump_demo/
cargo run --example capture -- scripts/jump_demo.txt --every 10 # plus every 10 ticks (animations)
cargo run --example sprite_sheet -- list                        # sprite names and frame counts
cargo run --example sprite_sheet -- character 0-7               # enlarged frames -> target/claude/sprites/character.png
```
- Capture plays the script with **one tick per rendered frame** (no wall-clock time), so the same script always gives byte-identical screenshots. It renders into an 800x600 offscreen texture (`Renderer::set_render_target`); a window opens briefly. The egui debug window is not included.
- It relies on miniquad's OpenGL backend (the default on macOS); `texture_read_pixels` is unimplemented in miniquad's Metal backend. It needs a display, so it can't run in CI.
- To check a scene, write a small script (ASCII map + inputs + `snap`) in `target/claude/`, capture it and look at the PNGs.
- **Rendering refactors:** captures are byte-identical for the same script, so compare before and after pixel by pixel. Build the old code from `git archive <commit> | tar -x -C target/claude/old`, run `capture` there with `CARGO_TARGET_DIR=../oldtarget` and `--out ../before`, run it again on the new code, and diff the PNGs. Use `--every N` on a random-map walk through doors to cover room transitions (the dark). Seed 6, `300 right` then `600 left`, crosses two doors.
- Sprite edits: edit `assets/sprites/*.aseprite`, re-export the atlas with the aseprite CLI (README.md; `aseprite` is on PATH), then use `sprite_sheet` and a capture to review.

## How to develop a feature
1. **Agree on the plan first.** The user likes to discuss and agree before work starts: describe the approach, open questions and trade-offs, then wait for a go-ahead. Ask when a change would alter existing game behaviour.
2. **Understand the current behaviour.** Read the code. Measure with a throwaway probe test (prints values, deleted afterwards) or `run_script`, rather than guessing numbers.
3. **Write or extend tests.** Where tests go:
   | Change | Test |
   |---|---|
   | Player movement, physics, controls | gameplay test in `tests/gameplay/player.rs` |
   | Enemies, items, combat, doors, rooms | gameplay test in `tests/gameplay/interactions.rs` (or a new module) |
   | New room content, map generation | content tests (`tests/content.rs`) cover it; extend them for new rules |
   | Pure logic (parsing, geometry, helpers) | unit test next to the code |
   | Rendering, UI, sprites, animations | no automated test; check visually with `capture` / `sprite_sheet` and look at the PNGs |
   - Test the edges, not just the happy path: the last tick something works and the first tick it doesn't, both directions (left/right), and the negative case (what must *not* happen).
   - Use `Room::from_ascii` scenes that isolate the behaviour. Pick thresholds from measured values, with a small tolerance.
4. **For a bug fix,** first write a test that reproduces the bug and confirm it **fails** on the old code, then fix it and confirm it passes.
5. **Implement.** Keep game logic deterministic (see Determinism) and follow the existing patterns (result enums, `UpdateCtx`, the "add an enemy" checklist).
6. **Check:** `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`. For anything visible, capture screenshots and look at them. Smoke-run the game when touching the main loop or rendering (`cargo run`, check the log for panics).
7. **Update docs:** the README TODO list, and this file when architecture, tools or conventions change. Record new quirks or design decisions under "Known issues and design decisions".
8. **Commit** (see Conventions). The user can watch the new gameplay tests with `cargo test --test gameplay -- --watch <filter>`.

## Known issues and design decisions
Found while writing tests; keep this list current.
- **Ladder and ledge jumps always rise ~1 tile**; holding jump doesn't make them higher (only ground jumps get the extra rise frames). Intended.
- **Pressing jump while holding up on a ladder is ignored** (the jump-off needs up released). Not pinned by a test.
- **Ladder climbing ignores collisions**, so the head can end up to one climb step (0.06 tiles, under a pixel) inside the ceiling. Harmless; the test allows it.
- **Room `disabled` flag** is ignored by the map generator (the user wants to keep it that way for now).
- **Doors are always open** (`TODO: TEMP` in `Game::update`); the "closed until enemies are dead" logic is disabled.
- **Collected coins aren't counted** (`ItemInteractionResult::IncreaseScore` does nothing). **Dying** has an animation but no game over or restart.
- **Character sprite frame 21** isn't used by any animation.
- The game's very first frame is drawn before the first tick, so it's all dark. One frame at startup; left as is.
- `coin.rs` is dead code (not in `state/mod.rs`).
- **Free camera zoom, not pixel-perfect:** the zoom fits the room and animates, so art pixels are rarely an integer scale and can shimmer. Kept on purpose for now; the fix (low-res target, integer upscale) is a README TODO.
- **`tile_revision` is a process-wide counter**, not game state. It only invalidates the renderer's tile cache; game logic must never read it.
- **The editor isn't covered by `capture`** (it only plays scripts in a `Game`), so editor drawing (doors, enemy spawn icons, live tile edits) has to be checked by hand.

## Conventions
- Small, focused changes. Keep the game behaving the same unless the change is meant to alter it.
- Commit each finished step as its own small commit once the checks above pass. **Push only when the user asks.** The remote uses SSH.
- Keep the README TODO list up to date when features land.
