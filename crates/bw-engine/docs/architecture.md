# bw-engine Architecture

## Overview

`bw-engine` is a selective Rust reimplementation of the StarCraft: Brood War game engine, based on the OpenBW C++ reference (`broodwar-live/openbw`). Designed for WASM compilation — accepts raw `&[u8]` inputs, no filesystem access, all data owned.

## Module Map

```
lib.rs              Re-exports public types

Map/Terrain:
  chk.rs             CHK section parser (tag-length-value format)
  chk_units.rs       CHK UNIT section → initial unit placements + start locations
  tileset.rs         Tileset enum, CV5/VF4 binary parsing (52/32 bytes per entry)
  tile.rs            TileFlags (bitflags), Tile, MiniTile, GroundHeight
  map.rs             Map struct: from_chk() constructor, walkability/height queries

Data:
  dat.rs             Parse units.dat (228 unit types), flingy.dat (209 flingy types),
                     weapons.dat (130 weapon types)
  fp8.rs             Fixed-point 24.8 math (Fp8, XY, isqrt)
  direction.rs       256-direction type with OpenBW's direction_table, turn logic

Pathfinding:
  regions.rs         Flood-fill walkable regions, neighbor graph, group connectivity
  pathfind.rs        Tile-level A* (8-directional, 2048 node budget) with region fallback

Simulation:
  unit.rs            UnitState (position, velocity, HP, weapons), UnitId (tag encoding),
                     movement physics (acceleration, turning, waypoint following)
  selection.rs       Per-player unit selection + 10 hotkey groups
  game.rs            Game struct: frame loop, EngineCommand dispatch, combat, production
  vision.rs          Per-player visibility grids (visible/explored), sight range circles
  error.rs           EngineError enum + Result alias
```

## Data Flow

```
                    .rep file
                       |
              replay_core::parse()
                       |
          +------------+-------------+
          |            |             |
       Header      Commands      map_data (CHK)
                                     |
                         +-----------+-----------+
                         |           |           |
                    chk::parse   tileset.cv5   tileset.vf4
                         |           |           |
                     Map::from_chk(chk, cv5, vf4)
                         |
                    RegionMap::from_map()
                         |
              Game::new(map, game_data)
                         |
              game.load_initial_units(chk_units)
                         |
              game.create_melee_starting_units(...)
                         |
    for each frame:
        game.apply_command(player, &cmd)
        game.step()
                         |
              game.units()  →  unit positions, HP, types
              game.visibility_grid(player)  →  fog of war
              game.player_state(player)  →  resources
```

## Tile Hierarchy

```
Map tile (32x32 px) → tile_id → CV5[group].mega_tile[subtile] → VF4[megatile] → 4x4 mini-tiles (8x8 px)
```

- **Tile ID encoding**: bits 0-3 = subtile, bits 4-14 = group_index
- **Walkability**: summarized from VF4 mini-tile flags (>12 walkable = WALKABLE)
- **Height**: Low, Middle, High, VeryHigh from mini-tile flags

## Movement Physics (per frame)

Matches OpenBW's `bwgame.h:8677-8792`:

1. Compute desired direction toward current waypoint
2. Turn heading at `turn_rate/2` (ground) or `turn_rate` (air)
3. Accelerate when facing target, decelerate when turning
4. Cap speed at `top_speed`
5. Velocity = heading unit vector * speed
6. Position += velocity (fp8 precision)
7. Check arrival → advance to next waypoint

## Pathfinding

Two-tier system:

1. **Tile A*** (primary): 8-directional on 32px grid, 2048 node budget, diagonal corner-cutting prevention, collinear waypoint simplification
2. **Region A*** (fallback): over flood-filled walkable regions, used when tile A* exceeds budget

## Combat

- Weapon data from `weapons.dat`: damage, cooldown, range, damage factor
- Attack order: move into range → fire when cooldown=0 → damage = (amount * factor - armor), min 1
- Death: HP <= 0 → unit removed

## Production

- `Train` command queues unit type at building (max 5 queue)
- Timer counts down `build_time` frames
- New unit spawned adjacent to building on completion

## Unit Tag System

BW replays reference units by 16-bit tags:
- Bits 0-10: slot index (0-1699)
- Bits 11-15: generation counter

Tag assignment matches BW's order:
- CHK UNIT entries processed sequentially
- Turret subunits consume extra slots (units < type 117 with turret_unit_type)
- Melee starting units appended after CHK entries

## Dependencies

- `thiserror` — error derives
- `bitflags` — TileFlags
