# replay-core Architecture

## Overview

`replay-core` is a pure Rust library that parses StarCraft: Brood War replay files (`.rep`) into structured data. It handles all three replay format generations and produces analytics from the parsed command stream.

## Module Map

```
lib.rs              Entry point: parse(&[u8]) -> Result<Replay>
  format.rs          Detect replay format (Legacy/Modern/Modern121)
  section.rs         Decompress sections (PKWare DCL or zlib)
  header.rs          Parse section 1: Header, Player, Engine, Speed, Race, GameType
  command.rs         Parse section 2: GameCommand stream with 40+ Command variants
  analysis.rs        Derive BuildOrderEntry, PlayerApm, ApmSample from commands
  timeline.rs        Build TimelineSnapshot with PlayerState at each build action
  gamedata.rs        Static lookup tables: unit/tech/upgrade names, costs, races
  error.rs           ReplayError enum + Result alias
```

## Replay File Format

A `.rep` file contains 5 compressed sections:

| Section | Content | Size |
|---------|---------|------|
| 0 | Magic bytes (`seRS` or `reRS`) | 4 bytes |
| 1 | Header: game metadata, 12 player slots | 633 bytes |
| 2 | Command stream: all player actions with frame timing | Variable |
| 3 | Map data: CHK scenario (passed through to `bw-engine`) | Variable |
| 4 | Extended player names (optional, 1.21+) | 768 bytes |

### Compression Formats

- **Legacy** (pre-1.18): PKWare DCL Implode via `explode` crate
- **Modern** (1.18-1.20): zlib via `flate2`
- **Modern 1.21+** (Remastered): zlib with 4-byte length field after section 0

Detection is by magic bytes at offsets 12 and 28.

## Key Types

### `Replay`
Top-level output containing all parsed and derived data:
- `header: Header` — map name, dimensions, players, duration
- `commands: Vec<GameCommand>` — raw command stream
- `build_order: Vec<BuildOrderEntry>` — filtered production/tech actions
- `player_apm: Vec<PlayerApm>` — per-player APM and EAPM
- `timeline: Vec<TimelineSnapshot>` — cumulative game state at each build action
- `map_data: Vec<u8>` — raw CHK bytes for `bw-engine`

### `Command` enum
40+ variants covering all BW player actions:
- Selection: `Select`, `SelectAdd`, `SelectRemove`
- Movement: `RightClick`, `TargetedOrder`, `Stop`, `HoldPosition`
- Production: `Train`, `Build`, `UnitMorph`, `BuildingMorph`
- Tech: `Research`, `Upgrade`
- UI: `Hotkey`, `MinimapPing`, `Chat`

### APM Classification
- **Meaningful actions** (`is_meaningful_action`): excludes KeepAlive, Select, Chat
- **Effective actions** (`is_effective_action`): also excludes hotkey recalls

## Character Encoding

Player names are decoded with UTF-8 first, falling back to EUC-KR (CP949) for Korean. StarCraft control characters (bytes < 0x20) are stripped.

## Dependencies

- `thiserror` — error derives
- `flate2` — zlib decompression
- `explode` — PKWare DCL decompression
- `encoding_rs` — EUC-KR character encoding
- `serde` — Serialize derives for WASM serialization
