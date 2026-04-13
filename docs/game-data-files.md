# Game Data Files

The simulation engine needs data files from a StarCraft: Brood War installation to accurately simulate unit behavior. These are small binary files in the `arr/` and `tileset/` directories.

## Required for Simulation

| File | Size | Contains |
|------|------|----------|
| `units.dat` | ~20 KB | 228 unit types: flingy mapping, HP, weapons, armor, build time, sight range |
| `flingy.dat` | ~3 KB | 209 movement types: speed, acceleration, halt distance, turn rate |

## Required for Combat

| File | Size | Contains |
|------|------|----------|
| `weapons.dat` | ~5 KB | 130 weapon types: damage, cooldown, range, damage factor |

## Optional: Tech & Upgrades

| File | Size | Contains |
|------|------|----------|
| `techdata.dat` | ~1 KB | 44 tech types: mineral/gas cost, research time, energy cost |
| `upgrades.dat` | ~2 KB | 61 upgrade types: base/factor cost scaling, max level |
| `orders.dat` | ~5 KB | 189 order types: interruptable, queueable, weapon targeting |

## Optional: String Tables

| File | Size | Contains |
|------|------|----------|
| `stat_txt.tbl` | ~20 KB | Indexed string table: unit names, tech names, tooltips |

## Required for Map Terrain

| File | Size | Contains |
|------|------|----------|
| `<tileset>.cv5` | ~70 KB | Tile groups: flags + 16 mega-tile references per group |
| `<tileset>.vf4` | ~30 KB | Mega-tiles: 4x4 mini-tile walkability/height flags |

## Optional: Map Rendering

| File | Size | Contains |
|------|------|----------|
| `<tileset>.vx4` | ~30 KB | Mega-tiles: 4x4 mini-tile graphic references (VR4 index + flip) |
| `<tileset>.vr4` | ~300 KB | Mini-tile images: 8x8 palette-indexed pixel data |
| `<tileset>.wpe` | 1 KB | 256-color tileset palette (RGBX) |

## Optional: Sprites

| File | Size | Contains |
|------|------|----------|
| `*.grp` | varies | Unit/building sprites: RLE-encoded frames with palette indices |

Tileset names by map type:

| Tileset Index | Name | File Stem |
|---------------|------|-----------|
| 0 | Badlands | `badlands` |
| 1 | Space Platform | `platform` |
| 2 | Installation | `install` |
| 3 | Ashworld | `AshWorld` |
| 4 | Jungle | `Jungle` |
| 5 | Desert | `Desert` |
| 6 | Arctic | `Ice` |
| 7 | Twilight | `Twilight` |

The replay header's tileset can be read from the `ERA` section of the CHK data, or the tileset index is available after parsing.

## Where to Find These Files

### StarCraft: Remastered (free to play)
```
StarCraft/arr/units.dat
StarCraft/arr/flingy.dat
StarCraft/arr/weapons.dat
StarCraft/arr/techdata.dat
StarCraft/arr/upgrades.dat
StarCraft/arr/orders.dat
StarCraft/rez/stat_txt.tbl
StarCraft/tileset/badlands.cv5
StarCraft/tileset/badlands.vf4
StarCraft/tileset/badlands.vx4
StarCraft/tileset/badlands.vr4
StarCraft/tileset/badlands.wpe
...
```

### StarCraft 1.16.1 (original)
Same paths relative to the installation directory. Files may also be inside MPQ archives (StarDat.mpq, BrooDat.mpq). You can use `MpqArchive::from_bytes()` to extract files from MPQ archives directly.

### Loading from MPQ archives

```rust
use bw_engine::MpqArchive;

let archive = MpqArchive::from_bytes(std::fs::read("BrooDat.mpq")?)?;
let units_dat = archive.read_file("arr\\units.dat")?;
```

### Loading SCX/SCM map files

Map files (`.scx`, `.scm`) are small MPQ archives. Use `ScxMap` to extract the CHK data:

```rust
use bw_engine::ScxMap;

let scx = ScxMap::from_bytes(std::fs::read("(4)Fighting Spirit.scx")?)?;
let map = scx.to_map(&cv5_data, &vf4_data)?;
```

## Without Game Data

The parser (`replay-core`) works without any game data files. You get:
- Full replay metadata and player info
- Complete command stream
- Build orders with human-readable unit/tech names (hardcoded lookup tables)
- APM statistics and timeline

The simulation (`bw-engine`) can run with minimal synthetic data but unit behavior won't match the real game (wrong speeds, no combat, wrong HP values).
