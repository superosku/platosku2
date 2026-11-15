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

The game prints FPS and UPS to stdout every second, and the map generator logs a lot (`Iterating for adding a room ...`). That output is normal.

## Architecture

### Main loop (`src/main.rs`)
- `Stage` implements miniquad's `EventHandler`. It owns `state: Box<dyn FullGameState>` (`Game` or `Editor`), `InputState`, `Renderer`, `Camera`, `SoundHandler` and `DebugMenu`.
- **Fixed timestep:** `update()` accumulates real time and runs `state.update(...)` plus `state.update_camera(...)` at exactly 1/60 s per tick. Frame time is capped at 0.1 s. Rendering is uncapped and separate (`draw()`).
- `InputState` is a plain struct with booleans. Key events set `left/right/up/down/*_held`. The edge-triggered `jump_pressed` and `swing_pressed` are set on key-down and **cleared after each tick**.
- Keys: arrows move, **Z** jumps, **X** swings, throws or picks up (with Down: drop, throw down or pick up), **M** shows the big map.
- The egui "Debug" window switches between Game and Editor. Switching to Game calls `Game::new()`, which regenerates the map.

### State (`src/state/`)
- `GameState` trait (`game_state.rs`): `update`, `update_camera`, accessors for player and map, and `update_and_draw_minimap`. `FullGameState` = `GameState + DrawableGameState (render.rs) + GameStateDebugMenu (debug_menu/)`.
- `Game`: player, `GameMap`, `items: Vec<Item>`, `enemies: Vec<Box<dyn Enemy>>`, current and previous room index (for camera and room-transition "dark" effect), `Minimap`. `Game::update` does all the interactions: door push-out, player update, items (physics, pickup, swing hits, projectile hits on enemies), enemies (only those in the current room are updated), stomp and contact damage, room change detection, doors.
- `Editor`: a single `Room` plus a player for testing the room.
- **Result-enum pattern:** entities don't mutate the world directly. They return `Vec<...Result>` (`PlayerUpdateResult`, `EnemyUpdateResult`, `ItemInteractionResult`) and `Game::update` applies them. Follow this pattern for new interactions.
- `player.rs`: `PlayerState` state machine (`Normal`, `Swinging`, `Hanging`, `OnLadder`, `Dead`). It includes coyote time (`safe_edge_frames`), variable jump hold (`max_jump_frames`), ledge hang, ladders and drop-through platforms (down+jump). `Health` + `immunity_frames`.
- `enemies/`: the `Enemy` trait (`common.rs`) and `Bat`, `Slime`, `Worm`, `Burrower`. Each has its own state enum and an `AnimationHandler`. To add an enemy: implement `Enemy`, add a variant to `ObjectTemplateType` and `ObjectTemplate::as_object/get_texture_index` (`map_like/common.rs`), add it to the editor's `EnemySelection`, and add a sprite plus a UI icon.
- `item.rs`: `Item` + `ItemType` (Coin, stones, Box, GreenProjectile). Boxes break into coins when swung. GreenProjectile is the bat's attack.
- `animation_handler.rs`: generic `AnimationHandler<T: AnimationConfig>`. A state maps to (start frame, end frame, ticks per frame, loop, reverse) in the atlas.
- `common.rs`: `BoundingBox` (position, size **and velocity** `vx/vy`), `Pos`, `Dir`, `Health`.
- `coin.rs` is **dead code**. It isn't declared in `state/mod.rs`, so it isn't compiled.

### Map (`src/state/map_like/`)
- `MapLike` trait: tile queries (`get_at`, `is_solid_at_tile`, `is_ladder_at`, `is_platform_at`, `overlaps_solid`, …). It is implemented by `Room` and `GameMap`.
- Tiles: `BaseTile` (NotPartOfRoom, Empty, Stone, Wood; everything except Empty is solid) + `OverlayTile` (Ladder, Platform, LadderPlatform, StartDoor).
- `Room` (`room.rs`): serde JSON in `rooms/room_XXXX.json`. It holds base and overlay tiles, its position, `doors: Vec<RoomDoor>`, `object_templates` (enemy spawns) and `disabled`. `Room::load_rooms_from_folder()` reads `rooms/`. The editor saves here too.
- `GameMap::new_random()` (`game_map.rs`): starts from the **first** room that has a StartDoor, then randomly attaches rooms door to door (max 1000 attempts, stops at 10 rooms), rejecting overlaps. It then flattens everything into one tile grid. `MapDoor` is a door between rooms, with animation and temporary closing. Doors are currently **always open** (see the `TODO: TEMP` in `Game::update`).
- Coordinates: world units are **tiles** (1.0 = one tile = 16 px, `render::TILE_SIZE`). Positions are f32, tile indices i32, and y grows downward.

### Physics (`src/physics.rs`)
- `integrate_kinematic(map, bb, gravity)`: per-axis sweep (X then Y) with bisection against `overlaps_solid`. Returns a new bb plus `on_bottom/top/left/right`. GRAVITY = 0.007 tiles/tick², terminal velocity 0.9.
- `check_and_snap_platforms` (one-way platforms) and `check_and_snap_hang` (ledge grab).

### Rendering (`src/render.rs`, `src/minimap.rs`, `src/camera.rs`)
- `Renderer` owns the miniquad `Context`, 3 pipelines (sprites, batched tiles, HUD) and textures. Sprites are queued into one atlas batch (`draw_from_texture_atlas`) and flushed once per frame.
- Tiles use a **dual-grid** autotiling system (`draw_base_dual_grid`, `DUAL_GRID_UV_TABLE`) with layer indices into `tile_backgrounds.png`.
- The "dark" overlay hides rooms other than the current one and animates during room transitions (`DrawableGameState::draw_extra_last`).
- `Camera` lerps smoothly towards a target (`slowly_follow`). `screen_to_tile` must match the renderer's view transform.
- `Minimap`: built as a CPU pixel image and uploaded to a texture only when the room changes. It shows visited rooms and supports zooming.
- Entity `draw` methods live on the entities themselves and take `&mut Renderer`.

### Sound (`src/sound_handler.rs`)
- The `define_sounds!` macro maps the `Sound` enum to file names. `play()` picks a random **variant** from `assets/sounds/dest/<name>__vNN.wav`.
- `assets/sounds/dest/` is gitignored and generated by `make_sound_variants.sh` (needs `ffmpeg` with rubberband). **If it's missing, the game is silent.** No error is shown, because non-variant files are looked up without the `.wav` extension, so they never load.
- Sound files are third-party, used with permission and must not be redistributed (`assets/sounds/README.md`).

### Assets
- Sprites are edited as `assets/sprites/*.aseprite` and packed into `assets/atlas.png` + `assets/atlas.json` with the aseprite CLI (command in README.md). `AtlasInfo` keys frames by (name, frame index), parsed from `"<name> <n>.aseprite"`. A missing sprite logs "Sprite not found" and draws a 1x1 rect.
- `ui_sprite_creator.py` (PIL) cuts editor icons from the atlas into `assets/ui_sprites/`.
- aseprite and ffmpeg are not necessarily installed on the dev machine. Check before relying on them.

## Testability notes (current state)
- There are only 4 unit tests (room resize, in `room.rs`).
- Good for testing: fixed timestep, plain-data `InputState`, pure `physics.rs` functions and `MapLike` (a `Room` can be built in code with `Room::new_boxed`/`new_empty`).
- Obstacles to headless and deterministic tests:
  - `SoundHandler` is passed into `update()` everywhere and creates a real `AudioContext`.
  - Randomness uses global `rand::rng()` in map generation, items, bat, slime and burrower. It isn't seeded.
  - `Game::new()` reads `rooms/` from disk and always generates a random map.
  - `Game` owns `Minimap`, and the minimap draw is part of the `GameState` trait. `Renderer::new()` needs a window.
  - `Enemy::update` takes a concrete `&GameMap`, not `&dyn MapLike`.

## Conventions
- Small, focused changes. Keep the game behaving the same unless the change is meant to alter it.
- Don't commit unless asked.
- The README's TODO list is partly out of date (Worm and Burrower already exist).
