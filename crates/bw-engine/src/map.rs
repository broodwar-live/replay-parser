use crate::chk::{self, ChkTerrain};
use crate::error::Result;
use crate::tile::{GroundHeight, MiniTile, Tile, TileFlags};
use crate::tileset::{Tileset, TilesetData};

/// Mask to strip walkability/height flags from CV5 flags before merging.
/// These are computed from VF4 mini-tile data, not taken from CV5 directly.
const CV5_STRIP_MASK: u16 = !(TileFlags::WALKABLE.bits()
    | TileFlags::UNWALKABLE.bits()
    | TileFlags::VERY_HIGH.bits()
    | TileFlags::MIDDLE.bits()
    | TileFlags::HIGH.bits()
    | TileFlags::PARTIALLY_WALKABLE.bits());

/// A fully parsed and queryable StarCraft: Brood War map.
#[derive(Debug)]
pub struct Map {
    width: u16,
    height: u16,
    tileset: Tileset,
    tiles: Vec<Tile>,
    mini_tiles: Vec<MiniTile>,
}

impl Map {
    /// Construct a Map from raw CHK data and raw tileset files.
    ///
    /// - `chk_data`: raw bytes of a `.chk` file (or replay section 3).
    /// - `cv5_data`: raw bytes of the tileset's `.cv5` file.
    /// - `vf4_data`: raw bytes of the tileset's `.vf4` file.
    pub fn from_chk(chk_data: &[u8], cv5_data: &[u8], vf4_data: &[u8]) -> Result<Self> {
        let sections = chk::parse_sections(chk_data)?;
        let terrain = chk::extract_terrain(&sections)?;
        let tileset_data = TilesetData::from_bytes(cv5_data, vf4_data)?;
        Self::from_terrain(&terrain, &tileset_data)
    }

    /// Construct from pre-parsed terrain and tileset data.
    pub fn from_terrain(terrain: &ChkTerrain, tileset_data: &TilesetData) -> Result<Self> {
        let tileset = Tileset::from_index(terrain.tileset_index)?;
        let width = terrain.width;
        let height = terrain.height;
        let tile_count = width as usize * height as usize;
        let mini_width = width as usize * 4;
        let mini_height = height as usize * 4;

        let mut tiles = Vec::with_capacity(tile_count);
        let mut mini_tiles = vec![MiniTile { flags: 0 }; mini_width * mini_height];

        for i in 0..tile_count {
            let tile_id = terrain.tile_ids[i];
            let (tile, tile_minis) = compute_tile(tile_id, tileset_data);

            // Place mini-tiles into the flat mini-tile grid.
            let tx = i % width as usize;
            let ty = i / width as usize;
            for my in 0..4 {
                for mx in 0..4 {
                    let mini_idx = (ty * 4 + my) * mini_width + (tx * 4 + mx);
                    mini_tiles[mini_idx] = tile_minis[my * 4 + mx];
                }
            }

            tiles.push(tile);
        }

        // Force border tiles to unwalkable/unbuildable (bwgame.h:21357-21363).
        apply_border_flags(&mut tiles, width as usize, height as usize);

        Ok(Self {
            width,
            height,
            tileset,
            tiles,
            mini_tiles,
        })
    }

    /// Map width in 32px tiles.
    #[must_use]
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height in 32px tiles.
    #[must_use]
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Map width in pixels.
    #[must_use]
    pub fn width_px(&self) -> u32 {
        self.width as u32 * 32
    }

    /// Map height in pixels.
    #[must_use]
    pub fn height_px(&self) -> u32 {
        self.height as u32 * 32
    }

    /// The map's tileset.
    #[must_use]
    pub fn tileset(&self) -> Tileset {
        self.tileset
    }

    /// Get the tile at grid position (tx, ty).
    #[must_use]
    pub fn tile(&self, tx: u16, ty: u16) -> Option<&Tile> {
        if tx >= self.width || ty >= self.height {
            return None;
        }
        Some(&self.tiles[ty as usize * self.width as usize + tx as usize])
    }

    /// Whether tile (tx, ty) is fully walkable.
    #[must_use]
    pub fn is_tile_walkable(&self, tx: u16, ty: u16) -> bool {
        self.tile(tx, ty).is_some_and(|t| t.is_walkable())
    }

    /// Ground height at tile (tx, ty).
    #[must_use]
    pub fn tile_ground_height(&self, tx: u16, ty: u16) -> Option<GroundHeight> {
        self.tile(tx, ty).map(|t| t.ground_height())
    }

    /// Get the mini-tile at walk-grid position (mx, my).
    #[must_use]
    pub fn mini_tile(&self, mx: u16, my: u16) -> Option<&MiniTile> {
        let mw = self.width as u16 * 4;
        let mh = self.height as u16 * 4;
        if mx >= mw || my >= mh {
            return None;
        }
        Some(&self.mini_tiles[my as usize * mw as usize + mx as usize])
    }

    /// Whether mini-tile position (mx, my) is walkable.
    #[must_use]
    pub fn is_walkable(&self, mx: u16, my: u16) -> bool {
        self.mini_tile(mx, my).is_some_and(|mt| mt.is_walkable())
    }

    /// Ground height at mini-tile position (mx, my).
    #[must_use]
    pub fn ground_height(&self, mx: u16, my: u16) -> Option<GroundHeight> {
        self.mini_tile(mx, my).map(|mt| mt.ground_height())
    }

    /// Whether the pixel position (px, py) is walkable.
    #[must_use]
    pub fn is_walkable_px(&self, px: u32, py: u32) -> bool {
        let mx = (px / 8) as u16;
        let my = (py / 8) as u16;
        self.is_walkable(mx, my)
    }

    /// Ground height at pixel position (px, py).
    #[must_use]
    pub fn ground_height_px(&self, px: u32, py: u32) -> Option<GroundHeight> {
        let mx = (px / 8) as u16;
        let my = (py / 8) as u16;
        self.ground_height(mx, my)
    }

    /// Access the flat tile array (row-major, width * height).
    #[must_use]
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }

    /// Access the flat mini-tile array (row-major, width*4 * height*4).
    #[must_use]
    pub fn mini_tiles(&self) -> &[MiniTile] {
        &self.mini_tiles
    }
}

/// Compute a single tile's flags and its 16 mini-tiles from tileset data.
fn compute_tile(tile_id: u16, tileset_data: &TilesetData) -> (Tile, [MiniTile; 16]) {
    let empty_tile = (
        Tile {
            flags: TileFlags::UNWALKABLE,
            raw_tile_id: tile_id,
        },
        [MiniTile { flags: 0 }; 16],
    );

    // Look up megatile index from CV5. Out-of-bounds = empty tile.
    let Some(megatile_idx) = tileset_data.megatile_index(tile_id) else {
        return empty_tile;
    };

    // Look up mini-tile flags from VF4. Out-of-bounds = empty tile.
    let Ok(mini_flags) = tileset_data.mini_tile_flags(megatile_idx) else {
        return empty_tile;
    };

    let mut tile_flags = TileFlags::empty();
    let mut walkable_count = 0u8;
    let mut middle_count = 0u8;
    let mut high_count = 0u8;
    let mut very_high_count = 0u8;
    let mut mini_tiles = [MiniTile { flags: 0 }; 16];

    for (i, &mf) in mini_flags.iter().enumerate() {
        mini_tiles[i] = MiniTile { flags: mf };

        if mf & MiniTile::WALKABLE != 0 {
            walkable_count += 1;
        }
        if mf & MiniTile::MIDDLE != 0 {
            middle_count += 1;
        }
        if mf & MiniTile::HIGH != 0 {
            high_count += 1;
        }
        if mf & MiniTile::VERY_HIGH != 0 {
            very_high_count += 1;
        }
    }

    // Summarize walkability at tile level (bwgame.h:21094-21096).
    if walkable_count > 12 {
        tile_flags |= TileFlags::WALKABLE;
    } else {
        tile_flags |= TileFlags::UNWALKABLE;
    }
    if walkable_count > 0 && walkable_count != 16 {
        tile_flags |= TileFlags::PARTIALLY_WALKABLE;
    }

    // Height flags (bwgame.h:21097-21099).
    if high_count < 12 && (middle_count + high_count) >= 12 {
        tile_flags |= TileFlags::MIDDLE;
    }
    if high_count >= 12 {
        tile_flags |= TileFlags::HIGH;
    }
    if very_high_count > 0 {
        tile_flags |= TileFlags::VERY_HIGH;
    }

    // Merge CV5 group flags (non-walkability/height bits).
    let cv5_flags = tileset_data.cv5_flags(tile_id) & CV5_STRIP_MASK;
    tile_flags |= TileFlags::from_bits_truncate(cv5_flags);

    let tile = Tile {
        flags: tile_flags,
        raw_tile_id: tile_id,
    };

    (tile, mini_tiles)
}

/// Force border tiles unwalkable/unbuildable per OpenBW (bwgame.h:21357-21363).
fn apply_border_flags(tiles: &mut [Tile], width: usize, height: usize) {
    if height < 2 || width < 5 {
        return;
    }

    let strip =
        TileFlags::WALKABLE | TileFlags::HAS_CREEP | TileFlags::PARTIALLY_WALKABLE;

    // Bottom-1 row: leftmost 5 and rightmost 5 tiles.
    let row = height - 2;
    for col in 0..5.min(width) {
        let idx = row * width + col;
        tiles[idx].flags.remove(strip);
        tiles[idx].flags.insert(TileFlags::UNBUILDABLE);
    }
    for col in width.saturating_sub(5)..width {
        let idx = row * width + col;
        tiles[idx].flags.remove(strip);
        tiles[idx].flags.insert(TileFlags::UNBUILDABLE);
    }

    // Bottom row: entire row.
    let row = height - 1;
    for col in 0..width {
        let idx = row * width + col;
        tiles[idx].flags.remove(strip);
        tiles[idx].flags.insert(TileFlags::UNBUILDABLE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tileset::{CV5_ENTRY_SIZE, VF4_ENTRY_SIZE};

    fn build_cv5_entry(flags: u16, mega_tile_indices: &[u16; 16]) -> Vec<u8> {
        let mut entry = vec![0u8; CV5_ENTRY_SIZE];
        entry[2..4].copy_from_slice(&flags.to_le_bytes());
        for (j, &idx) in mega_tile_indices.iter().enumerate() {
            entry[20 + j * 2..22 + j * 2].copy_from_slice(&idx.to_le_bytes());
        }
        entry
    }

    fn build_vf4_entry(mini_tile_flags: &[u16; 16]) -> Vec<u8> {
        let mut entry = vec![0u8; VF4_ENTRY_SIZE];
        for (j, &f) in mini_tile_flags.iter().enumerate() {
            entry[j * 2..j * 2 + 2].copy_from_slice(&f.to_le_bytes());
        }
        entry
    }

    fn build_section(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(tag);
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(data);
        buf
    }

    /// Build a simple test map with known tile properties.
    ///
    /// Creates a 4x4 map where:
    /// - tile_id 0x0000 (group 0, subtile 0) -> megatile 0 -> all walkable, low ground
    /// - tile_id 0x0001 (group 0, subtile 1) -> megatile 1 -> all unwalkable, high ground
    fn build_test_map() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        // VF4: 2 megatile entries
        let walkable_flags = [MiniTile::WALKABLE; 16];
        let high_unwalkable_flags = [MiniTile::HIGH; 16]; // no WALKABLE bit
        let mut vf4_data = build_vf4_entry(&walkable_flags);
        vf4_data.extend_from_slice(&build_vf4_entry(&high_unwalkable_flags));

        // CV5: 1 group entry with subtile 0 -> megatile 0, subtile 1 -> megatile 1
        let mut indices = [0u16; 16];
        indices[0] = 0; // subtile 0 -> megatile 0
        indices[1] = 1; // subtile 1 -> megatile 1
        let cv5_data = build_cv5_entry(0, &indices);

        // CHK: 4x4 map, tileset 0 (Badlands)
        // Top-left 2x2 = walkable (tile_id 0x0000), rest = unwalkable (tile_id 0x0001)
        let mut mtxm = Vec::new();
        for row in 0..4u16 {
            for col in 0..4u16 {
                let tile_id: u16 = if row < 2 && col < 2 { 0x0000 } else { 0x0001 };
                mtxm.extend_from_slice(&tile_id.to_le_bytes());
            }
        }
        let mut chk = build_section(b"DIM ", &[4, 0, 4, 0]);
        chk.extend_from_slice(&build_section(b"ERA ", &[0, 0]));
        chk.extend_from_slice(&build_section(b"MTXM", &mtxm));

        (chk, cv5_data, vf4_data)
    }

    #[test]
    fn test_map_dimensions() {
        let (chk, cv5, vf4) = build_test_map();
        let map = Map::from_chk(&chk, &cv5, &vf4).unwrap();
        assert_eq!(map.width(), 4);
        assert_eq!(map.height(), 4);
        assert_eq!(map.width_px(), 128);
        assert_eq!(map.height_px(), 128);
        assert_eq!(map.tileset(), Tileset::Badlands);
    }

    #[test]
    fn test_map_tile_walkability() {
        let (chk, cv5, vf4) = build_test_map();
        let map = Map::from_chk(&chk, &cv5, &vf4).unwrap();

        // Top-left 2x2 are walkable tiles.
        assert!(map.is_tile_walkable(0, 0));
        assert!(map.is_tile_walkable(1, 0));
        assert!(map.is_tile_walkable(0, 1));
        assert!(map.is_tile_walkable(1, 1));

        // Others are unwalkable (or border-forced unwalkable).
        assert!(!map.is_tile_walkable(2, 0));
        assert!(!map.is_tile_walkable(3, 3));
    }

    #[test]
    fn test_map_tile_height() {
        let (chk, cv5, vf4) = build_test_map();
        let map = Map::from_chk(&chk, &cv5, &vf4).unwrap();

        assert_eq!(map.tile_ground_height(0, 0), Some(GroundHeight::Low));
        assert_eq!(map.tile_ground_height(2, 0), Some(GroundHeight::High));
    }

    #[test]
    fn test_map_mini_tile_walkability() {
        let (chk, cv5, vf4) = build_test_map();
        let map = Map::from_chk(&chk, &cv5, &vf4).unwrap();

        // Mini-tiles within tile (0,0): all walkable.
        assert!(map.is_walkable(0, 0));
        assert!(map.is_walkable(3, 3));

        // Mini-tiles within tile (2,0): all unwalkable.
        assert!(!map.is_walkable(8, 0));
        assert!(!map.is_walkable(11, 3));
    }

    #[test]
    fn test_map_pixel_walkability() {
        let (chk, cv5, vf4) = build_test_map();
        let map = Map::from_chk(&chk, &cv5, &vf4).unwrap();

        // Pixel (0,0) is in tile (0,0) -> walkable.
        assert!(map.is_walkable_px(0, 0));
        // Pixel (31,31) is still in tile (0,0) -> walkable.
        assert!(map.is_walkable_px(31, 31));
        // Pixel (64,0) is in tile (2,0) -> unwalkable.
        assert!(!map.is_walkable_px(64, 0));
    }

    #[test]
    fn test_map_out_of_bounds() {
        let (chk, cv5, vf4) = build_test_map();
        let map = Map::from_chk(&chk, &cv5, &vf4).unwrap();

        assert!(map.tile(100, 100).is_none());
        assert!(map.mini_tile(100, 100).is_none());
        assert!(!map.is_walkable(100, 100));
        assert_eq!(map.ground_height(100, 100), None);
    }

    #[test]
    fn test_border_tiles_unwalkable() {
        // Build a 6x4 map — all tiles walkable, then check borders.
        let walkable_flags = [MiniTile::WALKABLE; 16];
        let vf4_data = build_vf4_entry(&walkable_flags);
        let mut indices = [0u16; 16];
        indices[0] = 0;
        let cv5_data = build_cv5_entry(0, &indices);

        let mut mtxm = Vec::new();
        for _ in 0..(6 * 4) {
            mtxm.extend_from_slice(&0x0000u16.to_le_bytes());
        }
        let mut chk = build_section(b"DIM ", &[6, 0, 4, 0]);
        chk.extend_from_slice(&build_section(b"ERA ", &[0, 0]));
        chk.extend_from_slice(&build_section(b"MTXM", &mtxm));

        let map = Map::from_chk(&chk, &cv5_data, &vf4_data).unwrap();

        // Row 0 and 1 should be walkable.
        assert!(map.is_tile_walkable(0, 0));
        assert!(map.is_tile_walkable(5, 1));

        // Bottom row (row 3) should be forced unwalkable.
        assert!(!map.is_tile_walkable(0, 3));
        assert!(!map.is_tile_walkable(3, 3));

        // Row 2 (bottom-1): leftmost 5 and rightmost 5 forced unwalkable.
        // Width=6, so all 6 columns are within the 5-column border strips.
        assert!(!map.is_tile_walkable(0, 2));
        assert!(!map.is_tile_walkable(4, 2));
    }

    #[test]
    fn test_map_group_out_of_cv5_bounds() {
        // tile_id with group_index=1 but CV5 only has 1 entry (group 0).
        let vf4_data = build_vf4_entry(&[MiniTile::WALKABLE; 16]);
        let cv5_data = build_cv5_entry(0, &[0u16; 16]);

        let tile_id_oob: u16 = (1 << 4) | 0; // group 1, subtile 0
        let mut mtxm = Vec::new();
        mtxm.extend_from_slice(&tile_id_oob.to_le_bytes());
        let mut chk = build_section(b"DIM ", &[1, 0, 1, 0]);
        chk.extend_from_slice(&build_section(b"ERA ", &[0, 0]));
        chk.extend_from_slice(&build_section(b"MTXM", &mtxm));

        let map = Map::from_chk(&chk, &cv5_data, &vf4_data).unwrap();
        // Out-of-bounds group -> empty/unwalkable tile.
        assert!(!map.is_tile_walkable(0, 0));
    }
}
