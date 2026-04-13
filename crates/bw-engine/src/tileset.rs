use crate::error::{EngineError, Result};

/// The 8 StarCraft tilesets, indexed 0-7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Tileset {
    Badlands = 0,
    SpacePlatform = 1,
    Installation = 2,
    Ashworld = 3,
    Jungle = 4,
    Desert = 5,
    Arctic = 6,
    Twilight = 7,
}

impl Tileset {
    pub fn from_index(index: u16) -> Result<Self> {
        match index % 8 {
            0 => Ok(Self::Badlands),
            1 => Ok(Self::SpacePlatform),
            2 => Ok(Self::Installation),
            3 => Ok(Self::Ashworld),
            4 => Ok(Self::Jungle),
            5 => Ok(Self::Desert),
            6 => Ok(Self::Arctic),
            7 => Ok(Self::Twilight),
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Badlands => "Badlands",
            Self::SpacePlatform => "Space Platform",
            Self::Installation => "Installation",
            Self::Ashworld => "Ashworld",
            Self::Jungle => "Jungle",
            Self::Desert => "Desert",
            Self::Arctic => "Arctic",
            Self::Twilight => "Twilight",
        }
    }

    #[must_use]
    pub fn file_stem(self) -> &'static str {
        match self {
            Self::Badlands => "badlands",
            Self::SpacePlatform => "platform",
            Self::Installation => "install",
            Self::Ashworld => "AshWorld",
            Self::Jungle => "Jungle",
            Self::Desert => "Desert",
            Self::Arctic => "Ice",
            Self::Twilight => "Twilight",
        }
    }
}

/// CV5 entry size in bytes (confirmed from OpenBW bwgame.h:21117).
pub const CV5_ENTRY_SIZE: usize = 52;

/// VF4 entry size in bytes (16 x u16 mini-tile flags).
pub const VF4_ENTRY_SIZE: usize = 32;

/// Parsed CV5 entry (one per tile group).
#[derive(Debug, Clone)]
pub(crate) struct Cv5Entry {
    pub flags: u16,
    pub mega_tile_indices: [u16; 16],
}

/// Parsed VF4 entry: 16 mini-tile flags for a 4x4 grid of 8x8px mini-tiles.
#[derive(Debug, Clone)]
pub(crate) struct Vf4Entry {
    pub mini_tile_flags: [u16; 16],
}

/// Parsed tileset data ready for tile lookups.
pub struct TilesetData {
    pub(crate) cv5: Vec<Cv5Entry>,
    pub(crate) vf4: Vec<Vf4Entry>,
}

impl TilesetData {
    /// Parse from raw CV5 and VF4 file bytes.
    pub fn from_bytes(cv5_data: &[u8], vf4_data: &[u8]) -> Result<Self> {
        let cv5 = parse_cv5(cv5_data)?;
        let vf4 = parse_vf4(vf4_data)?;
        Ok(Self { cv5, vf4 })
    }

    /// Look up the VF4 megatile index for a given raw MTXM tile_id.
    ///
    /// Returns `None` if the group_index is out of bounds (treated as empty tile).
    pub(crate) fn megatile_index(&self, tile_id: u16) -> Option<u16> {
        let group_index = ((tile_id >> 4) & 0x7FF) as usize;
        let subtile_index = (tile_id & 0xF) as usize;

        let entry = self.cv5.get(group_index)?;
        Some(entry.mega_tile_indices[subtile_index])
    }

    /// Get the 4x4 mini-tile flags for a megatile.
    pub(crate) fn mini_tile_flags(&self, megatile_idx: u16) -> Result<&[u16; 16]> {
        let idx = megatile_idx as usize;
        self.vf4.get(idx).map(|e| &e.mini_tile_flags).ok_or(
            EngineError::MegatileLookupOutOfBounds {
                index: idx,
                vf4_len: self.vf4.len(),
            },
        )
    }

    /// Get the CV5 flags for a tile group.
    pub(crate) fn cv5_flags(&self, tile_id: u16) -> u16 {
        let group_index = ((tile_id >> 4) & 0x7FF) as usize;
        self.cv5.get(group_index).map(|e| e.flags).unwrap_or(0)
    }
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn parse_cv5(data: &[u8]) -> Result<Vec<Cv5Entry>> {
    if !data.len().is_multiple_of(CV5_ENTRY_SIZE) {
        return Err(EngineError::TilesetDataTooShort {
            file: "cv5",
            expected: CV5_ENTRY_SIZE,
            actual: data.len() % CV5_ENTRY_SIZE,
        });
    }

    let count = data.len() / CV5_ENTRY_SIZE;
    let mut entries = Vec::with_capacity(count);

    for i in 0..count {
        let base = i * CV5_ENTRY_SIZE;
        // bytes 0-1: skipped (index/type)
        // bytes 2-3: flags
        let flags = read_u16_le(data, base + 2);
        // bytes 4-19: skipped (4x u16 misc fields)
        // bytes 20-51: 16x u16 mega_tile_indices
        let mut mega_tile_indices = [0u16; 16];
        for (j, slot) in mega_tile_indices.iter_mut().enumerate() {
            *slot = read_u16_le(data, base + 20 + j * 2);
        }
        entries.push(Cv5Entry {
            flags,
            mega_tile_indices,
        });
    }

    Ok(entries)
}

fn parse_vf4(data: &[u8]) -> Result<Vec<Vf4Entry>> {
    if !data.len().is_multiple_of(VF4_ENTRY_SIZE) {
        return Err(EngineError::TilesetDataTooShort {
            file: "vf4",
            expected: VF4_ENTRY_SIZE,
            actual: data.len() % VF4_ENTRY_SIZE,
        });
    }

    let count = data.len() / VF4_ENTRY_SIZE;
    let mut entries = Vec::with_capacity(count);

    for i in 0..count {
        let base = i * VF4_ENTRY_SIZE;
        let mut mini_tile_flags = [0u16; 16];
        for (j, slot) in mini_tile_flags.iter_mut().enumerate() {
            *slot = read_u16_le(data, base + j * 2);
        }
        entries.push(Vf4Entry { mini_tile_flags });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tileset_from_index() {
        assert_eq!(Tileset::from_index(0).unwrap(), Tileset::Badlands);
        assert_eq!(Tileset::from_index(4).unwrap(), Tileset::Jungle);
        assert_eq!(Tileset::from_index(7).unwrap(), Tileset::Twilight);
    }

    #[test]
    fn test_tileset_modulo_8() {
        assert_eq!(Tileset::from_index(8).unwrap(), Tileset::Badlands);
        assert_eq!(Tileset::from_index(12).unwrap(), Tileset::Jungle);
        assert_eq!(Tileset::from_index(255).unwrap(), Tileset::Twilight);
    }

    #[test]
    fn test_tileset_file_stems() {
        assert_eq!(Tileset::Badlands.file_stem(), "badlands");
        assert_eq!(Tileset::Arctic.file_stem(), "Ice");
        assert_eq!(Tileset::SpacePlatform.file_stem(), "platform");
    }

    fn build_cv5_entry(flags: u16, mega_tile_indices: &[u16; 16]) -> Vec<u8> {
        let mut entry = vec![0u8; CV5_ENTRY_SIZE];
        // bytes 2-3: flags
        entry[2..4].copy_from_slice(&flags.to_le_bytes());
        // bytes 20-51: mega_tile_indices
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

    #[test]
    fn test_parse_cv5_single_entry() {
        let indices: [u16; 16] = [
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        ];
        let data = build_cv5_entry(0x00FF, &indices);
        let entries = parse_cv5(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].flags, 0x00FF);
        assert_eq!(entries[0].mega_tile_indices, indices);
    }

    #[test]
    fn test_parse_vf4_single_entry() {
        let flags: [u16; 16] = [1, 0, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1];
        let data = build_vf4_entry(&flags);
        let entries = parse_vf4(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].mini_tile_flags, flags);
    }

    #[test]
    fn test_parse_cv5_bad_size() {
        let data = vec![0u8; 51]; // not a multiple of 52
        assert!(parse_cv5(&data).is_err());
    }

    #[test]
    fn test_parse_vf4_bad_size() {
        let data = vec![0u8; 33]; // not a multiple of 32
        assert!(parse_vf4(&data).is_err());
    }

    #[test]
    fn test_megatile_index_lookup() {
        let mut indices = [0u16; 16];
        indices[5] = 42; // subtile 5 -> megatile 42
        let cv5_data = build_cv5_entry(0, &indices);
        let vf4_data = build_vf4_entry(&[0u16; 16]);

        let ts = TilesetData::from_bytes(&cv5_data, &vf4_data).unwrap();

        // tile_id with group_index=0, subtile=5: (0 << 4) | 5 = 5
        assert_eq!(ts.megatile_index(0x0005), Some(42));
    }

    #[test]
    fn test_megatile_index_out_of_bounds() {
        let cv5_data = build_cv5_entry(0, &[0u16; 16]); // 1 entry = group 0 only
        let vf4_data = build_vf4_entry(&[0u16; 16]);
        let ts = TilesetData::from_bytes(&cv5_data, &vf4_data).unwrap();

        // group_index=1 is out of bounds -> None
        let tile_id = 1u16 << 4;
        assert_eq!(ts.megatile_index(tile_id), None);
    }

    #[test]
    fn test_tile_id_encoding() {
        // group_index=3, subtile=7 -> raw = (3 << 4) | 7 = 55
        let raw: u16 = (3 << 4) | 7;
        assert_eq!((raw >> 4) & 0x7FF, 3);
        assert_eq!(raw & 0xF, 7);
    }
}
