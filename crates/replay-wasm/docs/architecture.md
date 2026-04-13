# replay-wasm Architecture

## Overview

`replay-wasm` provides WebAssembly bindings for `replay-core` and `bw-engine`, exposing replay parsing and game simulation to JavaScript via `wasm-bindgen`.

## Exports

### `parseReplay(data: Uint8Array) -> Object`

Parses a `.rep` file and returns the full replay as a JS object via serde serialization. All replay-core types (Header, Player, Command, BuildOrderEntry, etc.) are serialized to their JS equivalents.

### `GameMap`

Queryable map from CHK + tileset data.

```js
const map = new GameMap(chkData, cv5Data, vf4Data);
map.width              // tile dimensions
map.heightPx           // pixel dimensions
map.tileset            // "Jungle", "Badlands", etc.
map.isWalkable(mx, my) // mini-tile walkability
map.walkabilityGrid()  // flat Uint8Array (width*4 x height*4)
map.heightGrid()       // flat Uint8Array (0-3 per mini-tile)
```

### `GameSim`

Full game simulation that processes replay commands frame by frame.

```js
const sim = new GameSim(chkData, cv5, vf4, unitsDat, flingyDat, weaponsDat, repBytes);
sim.step();                  // advance one frame
sim.stepTo(1000);            // skip ahead
sim.currentFrame             // getter
sim.unitCount                // getter
sim.unitData()               // flat Int32Array: [x, y, type, owner, hp, maxHp, ...]
sim.playerData()             // [minerals, gas, supplyUsed, supplyMax] x 8 players
sim.visibilityGrid(player)   // Uint8Array: 0=fog, 1=explored, 2=visible
```

## Command Translation

`replay_core::Command` variants are translated to `bw_engine::EngineCommand`:

| Replay Command | Engine Command |
|----------------|----------------|
| Select, SelectAdd, SelectRemove | Select, SelectAdd, SelectRemove |
| Hotkey (Assign/Select) | HotkeyAssign, HotkeyRecall |
| RightClick (ground) | Move |
| RightClick (unit) | Attack |
| TargetedOrder (0x06) | Move |
| TargetedOrder (0x0A) | Attack |
| Stop | Stop |
| Train | Train |
| Build | Build |
| UnitMorph, BuildingMorph | UnitMorph, BuildingMorph |
| Research, Upgrade | Research, Upgrade |

## Build

```sh
wasm-pack build crates/replay-wasm --target web --out-dir ../../pkg
```

Output: `pkg/` directory with `.wasm` + JS glue (~322KB).

## Dependencies

- `replay-core` — replay parsing
- `bw-engine` — game simulation
- `wasm-bindgen` — Rust/JS FFI
- `serde-wasm-bindgen` — serialize Rust types to JsValue
