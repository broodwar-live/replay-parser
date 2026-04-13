# API Reference

## JavaScript API (via WASM)

### `parseReplay(data: Uint8Array) -> Object`

Parse a `.rep` file. Returns:

```js
{
  header: {
    engine: "BroodWar",
    frame_count: 40856,
    map_name: "Fighting Spirit",
    map_width: 128,
    map_height: 128,
    game_speed: "Fastest",
    game_type: "Melee",
    players: [
      { name: "Flash", race: "Terran", player_id: 0, color: 111, ... },
      { name: "Jaedong", race: "Zerg", player_id: 1, color: 117, ... }
    ]
  },
  commands: [ { frame: 100, player_id: 0, command: { Select: { unit_tags: [0, 1] } } }, ... ],
  build_order: [ { frame: 500, player_id: 0, action: { Train: 7 }, real_seconds: 21.0 }, ... ],
  player_apm: [ { player_id: 0, apm: 312.5, eapm: 280.1 }, ... ],
  timeline: [ { frame: 0, players: [...] }, ... ],
  map_data: Uint8Array  // raw CHK bytes
}
```

### `GameMap`

```js
const map = new GameMap(chkBytes, cv5Bytes, vf4Bytes);

// Properties
map.width         // u16 — tile width (e.g., 128)
map.height        // u16 — tile height
map.widthPx       // u32 — pixel width (e.g., 4096)
map.heightPx      // u32 — pixel height
map.tileset       // string — "Badlands", "Jungle", etc.

// Queries
map.isWalkable(mx, my)     // bool — mini-tile (8px grid)
map.isWalkablePx(px, py)   // bool — pixel coords
map.groundHeight(mx, my)   // u8 — 0=Low, 1=Mid, 2=High, 3=VeryHigh

// Bulk data for rendering
map.walkabilityGrid()  // Uint8Array — 1=walkable, 0=blocked, (width*4 x height*4)
map.heightGrid()       // Uint8Array — 0-3 per mini-tile
```

### `GameSim`

```js
const sim = new GameSim(chkBytes, cv5, vf4, unitsDat, flingyDat, weaponsDat, repBytes);
// weaponsDat can be empty Uint8Array (combat disabled)

// Stepping
sim.step()                // advance 1 frame, apply commands at this frame
sim.stepTo(targetFrame)   // skip ahead

// Properties
sim.currentFrame   // u32
sim.unitCount      // usize

// Unit data — flat Int32Array, 6 values per unit
sim.unitData()
// [x, y, unitType, owner, hp, maxHp, x, y, unitType, owner, hp, maxHp, ...]

// Player resources — flat Int32Array, 4 values per player x 8 players
sim.playerData()
// [minerals, gas, supplyUsed, supplyMax, ...]  (player 0, then player 1, ...)

// Fog of war — Uint8Array, one byte per tile
sim.visibilityGrid(player)
// 0=fog (unexplored), 1=explored (not visible), 2=visible
// Dimensions: map.width x map.height tiles
```

## Rust API

### replay-core

```rust
use replay_core::{parse, Replay};

let replay: Replay = parse(&bytes)?;

// Header
replay.header.map_name           // String
replay.header.frame_count        // u32
replay.header.duration_secs()    // f64
replay.header.players            // Vec<Player>

// Commands
replay.commands                  // Vec<GameCommand>
replay.commands[0].frame         // u32
replay.commands[0].player_id     // u8
replay.commands[0].command       // Command enum

// Analytics
replay.build_order               // Vec<BuildOrderEntry>
replay.player_apm                // Vec<PlayerApm>
replay.apm_over_time(60.0, 10.0) // Vec<ApmSample>

// Map data
replay.map_data                  // Vec<u8> — raw CHK bytes
```

### bw-engine

```rust
use bw_engine::*;

// Map
let map = Map::from_chk(&chk, &cv5, &vf4)?;
map.is_walkable(mx, my)         // bool — mini-tile
map.ground_height(mx, my)       // Option<GroundHeight>

// Game data
let data = GameData::from_dat_full(&units_dat, &flingy_dat, &weapons_dat)?;

// Simulation
let mut game = Game::new(map, data);
game.load_initial_units(&chk_units)?;
game.create_melee_starting_units(&start_locs, &races);
game.set_player_resources(0, 50, 0);

game.apply_command(0, &EngineCommand::Select(vec![0, 1]));
game.apply_command(0, &EngineCommand::Move { x: 200, y: 100 });
game.step();

// Queries
for unit in game.units() {
    println!("{}: ({}, {})", unit.unit_type, unit.pixel_x, unit.pixel_y);
}
game.player_state(0)             // Option<&PlayerState>
game.visibility_grid(0)          // Vec<u8>
```
