/// Per-player tile visibility.
///
/// Tracks which tiles each player can currently see and has explored.
/// Updated each frame based on alive unit positions and sight ranges.
pub struct VisionMap {
    width: u16,
    height: u16,
    /// Per-player visibility. `visible[player][ty * width + tx]` is true if
    /// the tile is currently in sight range of one of the player's units.
    visible: [Vec<bool>; 8],
    /// Per-player exploration. Once a tile is seen, it stays explored.
    explored: [Vec<bool>; 8],
}

impl VisionMap {
    pub fn new(width: u16, height: u16) -> Self {
        let tile_count = width as usize * height as usize;
        Self {
            width,
            height,
            visible: std::array::from_fn(|_| vec![false; tile_count]),
            explored: std::array::from_fn(|_| vec![false; tile_count]),
        }
    }

    /// Clear all visibility (called at start of each frame before re-computing).
    pub fn clear_visible(&mut self) {
        for vis in &mut self.visible {
            vis.fill(false);
        }
    }

    /// Mark tiles visible around a unit's position.
    ///
    /// `px`, `py`: unit position in pixels.
    /// `sight`: sight range in tiles.
    /// `owner`: player index (0-7).
    pub fn reveal(&mut self, px: i32, py: i32, sight: u8, owner: u8) {
        if owner >= 8 {
            return;
        }
        let cx = px / 32;
        let cy = py / 32;
        let r = sight as i32;
        let r_sq = r * r;
        let w = self.width as i32;
        let h = self.height as i32;
        let pi = owner as usize;

        for ty in (cy - r).max(0)..=(cy + r).min(h - 1) {
            for tx in (cx - r).max(0)..=(cx + r).min(w - 1) {
                let dx = tx - cx;
                let dy = ty - cy;
                if dx * dx + dy * dy <= r_sq {
                    let idx = ty as usize * w as usize + tx as usize;
                    self.visible[pi][idx] = true;
                    self.explored[pi][idx] = true;
                }
            }
        }
    }

    /// Whether a tile is currently visible to a player.
    pub fn is_visible(&self, player: u8, tx: u16, ty: u16) -> bool {
        if player >= 8 || tx >= self.width || ty >= self.height {
            return false;
        }
        self.visible[player as usize][ty as usize * self.width as usize + tx as usize]
    }

    /// Whether a tile has been explored by a player.
    pub fn is_explored(&self, player: u8, tx: u16, ty: u16) -> bool {
        if player >= 8 || tx >= self.width || ty >= self.height {
            return false;
        }
        self.explored[player as usize][ty as usize * self.width as usize + tx as usize]
    }

    /// Get the full visibility grid for a player as a flat byte array.
    /// 0 = fog (unexplored), 1 = explored but not visible, 2 = visible.
    pub fn visibility_grid(&self, player: u8) -> Vec<u8> {
        if player >= 8 {
            return Vec::new();
        }
        let pi = player as usize;
        let len = self.width as usize * self.height as usize;
        let mut grid = vec![0u8; len];
        for (i, cell) in grid.iter_mut().enumerate() {
            if self.visible[pi][i] {
                *cell = 2;
            } else if self.explored[pi][i] {
                *cell = 1;
            }
        }
        grid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reveal_and_query() {
        let mut vm = VisionMap::new(16, 16);
        // Unit at pixel (128, 128) = tile (4, 4), sight range 3 tiles.
        vm.reveal(128, 128, 3, 0);

        assert!(vm.is_visible(0, 4, 4)); // center
        assert!(vm.is_visible(0, 5, 4)); // 1 tile east
        assert!(vm.is_visible(0, 7, 4)); // 3 tiles east
        assert!(!vm.is_visible(0, 8, 4)); // 4 tiles east (out of range: 4*4=16 > 9)
        assert!(!vm.is_visible(1, 4, 4)); // different player
    }

    #[test]
    fn test_explored_persists() {
        let mut vm = VisionMap::new(8, 8);
        vm.reveal(64, 64, 2, 0);
        assert!(vm.is_explored(0, 2, 2));
        assert!(vm.is_visible(0, 2, 2));

        vm.clear_visible();
        assert!(!vm.is_visible(0, 2, 2)); // no longer visible
        assert!(vm.is_explored(0, 2, 2)); // still explored
    }

    #[test]
    fn test_visibility_grid() {
        let mut vm = VisionMap::new(4, 4);
        vm.reveal(48, 48, 1, 0);
        let grid = vm.visibility_grid(0);
        assert_eq!(grid.len(), 16);
        assert_eq!(grid[4 + 1], 2); // tile (1,1) = visible
        assert_eq!(grid[0], 0); // tile (0,0) = fog
    }
}
