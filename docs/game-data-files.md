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

## Required for Map Rendering

| File | Size | Contains |
|------|------|----------|
| `<tileset>.cv5` | ~70 KB | Tile groups: flags + 16 mega-tile references per group |
| `<tileset>.vf4` | ~30 KB | Mega-tiles: 4x4 mini-tile walkability/height flags |

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
StarCraft/tileset/badlands.cv5
StarCraft/tileset/badlands.vf4
...
```

### StarCraft 1.16.1 (original)
Same paths relative to the installation directory. Files may also be inside CASC/MPQ archives depending on version.

## Without Game Data

The parser (`replay-core`) works without any game data files. You get:
- Full replay metadata and player info
- Complete command stream
- Build orders with human-readable unit/tech names (hardcoded lookup tables)
- APM statistics and timeline

The simulation (`bw-engine`) can run with minimal synthetic data but unit behavior won't match the real game (wrong speeds, no combat, wrong HP values).
