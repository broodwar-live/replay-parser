use bitflags::bitflags;

bitflags! {
    /// Runtime tile flags matching OpenBW's `tile_t` flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TileFlags: u16 {
        const WALKABLE           = 0x0001;
        const UNWALKABLE         = 0x0004;
        const PROVIDES_COVER     = 0x0010;
        const HAS_CREEP          = 0x0040;
        const UNBUILDABLE        = 0x0080;
        const VERY_HIGH          = 0x0100;
        const MIDDLE             = 0x0200;
        const HIGH               = 0x0400;
        const OCCUPIED           = 0x0800;
        const PARTIALLY_WALKABLE = 0x2000;
    }
}

/// Ground height level derived from tile or mini-tile flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroundHeight {
    Low,
    Middle,
    High,
    VeryHigh,
}

/// A single map tile (32x32 pixels).
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub flags: TileFlags,
    pub raw_tile_id: u16,
}

impl Tile {
    #[must_use]
    pub fn is_walkable(self) -> bool {
        self.flags.contains(TileFlags::WALKABLE)
    }

    #[must_use]
    pub fn is_unwalkable(self) -> bool {
        self.flags.contains(TileFlags::UNWALKABLE)
    }

    #[must_use]
    pub fn is_partially_walkable(self) -> bool {
        self.flags.contains(TileFlags::PARTIALLY_WALKABLE)
    }

    #[must_use]
    pub fn ground_height(self) -> GroundHeight {
        if self.flags.contains(TileFlags::VERY_HIGH) {
            GroundHeight::VeryHigh
        } else if self.flags.contains(TileFlags::HIGH) {
            GroundHeight::High
        } else if self.flags.contains(TileFlags::MIDDLE) {
            GroundHeight::Middle
        } else {
            GroundHeight::Low
        }
    }

    #[must_use]
    pub fn has_creep(self) -> bool {
        self.flags.contains(TileFlags::HAS_CREEP)
    }
}

/// A mini-tile (8x8 pixels). 16 per map tile in a 4x4 grid.
#[derive(Debug, Clone, Copy)]
pub struct MiniTile {
    pub flags: u16,
}

impl MiniTile {
    pub const WALKABLE: u16 = 0x0001;
    pub const MIDDLE: u16 = 0x0002;
    pub const HIGH: u16 = 0x0004;
    pub const VERY_HIGH: u16 = 0x0008;

    #[must_use]
    pub fn is_walkable(self) -> bool {
        self.flags & Self::WALKABLE != 0
    }

    #[must_use]
    pub fn ground_height(self) -> GroundHeight {
        if self.flags & Self::VERY_HIGH != 0 {
            GroundHeight::VeryHigh
        } else if self.flags & Self::HIGH != 0 {
            GroundHeight::High
        } else if self.flags & Self::MIDDLE != 0 {
            GroundHeight::Middle
        } else {
            GroundHeight::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_walkable() {
        let tile = Tile {
            flags: TileFlags::WALKABLE,
            raw_tile_id: 0,
        };
        assert!(tile.is_walkable());
        assert!(!tile.is_unwalkable());
        assert!(!tile.is_partially_walkable());
    }

    #[test]
    fn test_tile_partially_walkable() {
        let tile = Tile {
            flags: TileFlags::PARTIALLY_WALKABLE,
            raw_tile_id: 0,
        };
        assert!(!tile.is_walkable());
        assert!(!tile.is_unwalkable());
        assert!(tile.is_partially_walkable());
    }

    #[test]
    fn test_tile_ground_height() {
        let low = Tile {
            flags: TileFlags::WALKABLE,
            raw_tile_id: 0,
        };
        assert_eq!(low.ground_height(), GroundHeight::Low);

        let mid = Tile {
            flags: TileFlags::MIDDLE,
            raw_tile_id: 0,
        };
        assert_eq!(mid.ground_height(), GroundHeight::Middle);

        let high = Tile {
            flags: TileFlags::HIGH,
            raw_tile_id: 0,
        };
        assert_eq!(high.ground_height(), GroundHeight::High);

        let very_high = Tile {
            flags: TileFlags::VERY_HIGH,
            raw_tile_id: 0,
        };
        assert_eq!(very_high.ground_height(), GroundHeight::VeryHigh);
    }

    #[test]
    fn test_tile_very_high_takes_precedence() {
        let tile = Tile {
            flags: TileFlags::VERY_HIGH | TileFlags::HIGH | TileFlags::MIDDLE,
            raw_tile_id: 0,
        };
        assert_eq!(tile.ground_height(), GroundHeight::VeryHigh);
    }

    #[test]
    fn test_tile_creep() {
        let tile = Tile {
            flags: TileFlags::WALKABLE | TileFlags::HAS_CREEP,
            raw_tile_id: 0,
        };
        assert!(tile.has_creep());
        assert!(tile.is_walkable());
    }

    #[test]
    fn test_mini_tile_walkable() {
        let mt = MiniTile {
            flags: MiniTile::WALKABLE,
        };
        assert!(mt.is_walkable());
        assert_eq!(mt.ground_height(), GroundHeight::Low);
    }

    #[test]
    fn test_mini_tile_height() {
        let mt = MiniTile {
            flags: MiniTile::HIGH | MiniTile::WALKABLE,
        };
        assert!(mt.is_walkable());
        assert_eq!(mt.ground_height(), GroundHeight::High);
    }

    #[test]
    fn test_mini_tile_not_walkable() {
        let mt = MiniTile { flags: 0 };
        assert!(!mt.is_walkable());
        assert_eq!(mt.ground_height(), GroundHeight::Low);
    }
}
