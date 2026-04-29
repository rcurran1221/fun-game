# Agent Instructions — fun-game (bevy_game)

## Repository

- **Local path:** `/home/robc/games/bevy_game`
- **Remote:** `git@github.com:rcurran1221/fun-game.git`
- **Main branch:** `main`

## Project

An OSRS-meets-Tarkov extraction fighting game built in Rust with Bevy 0.15.  
Single file: `src/main.rs` (~2000 lines).

## Environment

- Platform: WSL2 (Ubuntu on Windows)
- Software rendering required — set via `.cargo/config.toml`:
  ```
  LIBGL_ALWAYS_SOFTWARE = "1"
  ```
- No audio device available in WSL2 (harmless warning at runtime)

## Build & Run

```bash
cargo build          # ~8s incremental, ~5min cold
cargo run
```

## Controls

| Key | Action |
|-----|--------|
| WASD / Arrows | Move |
| LMB / Space | Attack nearest enemy |
| E | Mine nearest rock (must be within ~2m) |
| Walk into green zone | Begin extraction (3.5s) |
| R | Restart after death/extraction |
| ESC | Quit |

## Bevy 0.15 API Notes

- `EventWriter` uses `.send()`, not `.write()`
- Mesh components: `Mesh3d(...)`, `MeshMaterial3d(...)`
- UI: `Node`, `BackgroundColor`, `Text::new()`, `TextFont`, `TextColor`
- `AmbientLight` is a `Resource` — insert via `commands.insert_resource(...)`
- `Camera::viewport_to_world` returns `Option<Ray3d>` — use `.ok()?`
- Entity hierarchy: `commands.entity(child).set_parent(parent)` — NOT `add_child`
- Multiple mutable `Text` queries in the same system require `ParamSet`
- `Visibility::default()` = `Visibility::Inherited` — add to non-mesh pivot entities
- Do NOT take both `Res<T>` and `ResMut<T>` in the same system — use only `ResMut<T>`

## Architecture

All game code lives in `src/main.rs`. Key sections:

| Section | Description |
|---------|-------------|
| `setup` / `spawn_world` | One-time world geometry, rocks, extraction zones |
| `spawn_player` / `spawn_enemies` | Humanoid character construction |
| `build_humanoid` | Shared helper — spawns head/torso/arms/legs as child entities |
| `player_movement` | WASD movement, bounds clamping, cancels mining on move |
| `player_combat_mine` | LMB/Space attack + E mining + mining tick |
| `ai_update` | Enemy patrol → chase → attack state machine |
| `apply_damage` | Processes `DamageEvent`, triggers red screen flash |
| `check_deaths` | Player death → `GamePhase::Dead`; enemy death → despawn |
| `extraction_update` | Tracks time-on-zone, triggers `GamePhase::Extracted` |
| `animate_characters` | Idle / Walk / Mining animations for all humanoids |
| `update_hud` | Refreshes all UI text and bars each frame |
| `reset_game` | Despawns all `GameEntity` entities and rebuilds scene |
| `spawn_hud` | Creates all UI nodes (called on setup and reset) |

## Key Resources

| Resource | Purpose |
|----------|---------|
| `GamePhase` | `Playing` / `Dead` / `Extracted` |
| `PlayerAction` | `Free` / `Mining { target, progress, total, ore }` / `Extracting { progress }` |
| `Inventory` | Ore counts + total value |
| `PlayerStats` | Mining XP + level |
| `ShouldReset` | Flag checked by `reset_game` each frame |

## Key Components

- `Player` — marks the player entity
- `Enemy { ai, patrol, hp_fill }` — enemy state + patrol waypoints
- `Health { cur, max }` — on player and enemies
- `Rock { ore, depleted, respawn_timer, full_mat, depl_mat }` — on ore rocks
- `ExtractionZone` — marks extraction platform entities
- `GameEntity` — tag on every spawned entity; all are despawned on reset
- `PlayerLimbs` — holds entity IDs for each limb (on player and enemy roots)
- `AnimState` — `Idle` / `Walking` / `Mining` (drives `animate_characters`)
