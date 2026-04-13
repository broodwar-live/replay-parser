use std::collections::VecDeque;

use crate::map::Map;

/// A contiguous region of tiles with the same walkability.
#[derive(Debug, Clone)]
pub struct Region {
    pub index: usize,
    pub walkable: bool,
    pub center_x: i32,
    pub center_y: i32,
    pub tile_count: usize,
    pub group: usize,
    pub neighbors: Vec<usize>,
}

/// Tile-to-region mapping built from a Map's walkability data.
pub struct RegionMap {
    tile_region: Vec<u16>,
    pub regions: Vec<Region>,
    width: u16,
    height: u16,
}

const UNASSIGNED: u16 = u16::MAX;

impl RegionMap {
    /// Build regions from a Map by flood-filling contiguous walkable/unwalkable areas.
    pub fn from_map(map: &Map) -> Self {
        let w = map.width();
        let h = map.height();
        let tile_count = w as usize * h as usize;

        // Classify tiles: passable or not.
        let passable: Vec<bool> = (0..tile_count)
            .map(|i| {
                let tx = (i % w as usize) as u16;
                let ty = (i / w as usize) as u16;
                map.is_tile_passable(tx, ty)
            })
            .collect();

        let mut tile_region = vec![UNASSIGNED; tile_count];
        let mut regions: Vec<Region> = Vec::new();

        // Flood-fill to create regions.
        for start in 0..tile_count {
            if tile_region[start] != UNASSIGNED {
                continue;
            }
            let is_walkable = passable[start];
            let region_idx = regions.len() as u16;

            let mut queue = VecDeque::new();
            queue.push_back(start);
            tile_region[start] = region_idx;

            let mut sum_x: i64 = 0;
            let mut sum_y: i64 = 0;
            let mut count: usize = 0;

            while let Some(idx) = queue.pop_front() {
                let tx = (idx % w as usize) as i32;
                let ty = (idx / w as usize) as i32;
                sum_x += tx as i64;
                sum_y += ty as i64;
                count += 1;

                // 4-connected neighbors.
                for (nx, ny) in [(tx - 1, ty), (tx + 1, ty), (tx, ty - 1), (tx, ty + 1)] {
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let ni = ny as usize * w as usize + nx as usize;
                    if tile_region[ni] != UNASSIGNED {
                        continue;
                    }
                    if passable[ni] != is_walkable {
                        continue;
                    }
                    tile_region[ni] = region_idx;
                    queue.push_back(ni);
                }
            }

            // Center in pixel coords (tile center = tile * 32 + 16).
            let cx = ((sum_x as f64 / count as f64) * 32.0 + 16.0) as i32;
            let cy = ((sum_y as f64 / count as f64) * 32.0 + 16.0) as i32;

            regions.push(Region {
                index: regions.len(),
                walkable: is_walkable,
                center_x: cx,
                center_y: cy,
                tile_count: count,
                group: 0,
                neighbors: Vec::new(),
            });
        }

        let mut rm = Self {
            tile_region,
            regions,
            width: w,
            height: h,
        };
        rm.build_neighbors();
        rm.build_groups();
        rm
    }

    /// Get the region index at a pixel position.
    pub fn region_at_px(&self, px: i32, py: i32) -> Option<usize> {
        if px < 0 || py < 0 {
            return None;
        }
        let tx = px / 32;
        let ty = py / 32;
        if tx >= self.width as i32 || ty >= self.height as i32 {
            return None;
        }
        let idx = ty as usize * self.width as usize + tx as usize;
        let ri = self.tile_region[idx];
        if ri == UNASSIGNED {
            None
        } else {
            Some(ri as usize)
        }
    }

    /// Get a region by index.
    pub fn region(&self, index: usize) -> Option<&Region> {
        self.regions.get(index)
    }

    /// Check if two pixel positions are reachable (same walkable group).
    pub fn reachable(&self, ax: i32, ay: i32, bx: i32, by: i32) -> bool {
        let ra = self.region_at_px(ax, ay);
        let rb = self.region_at_px(bx, by);
        match (ra, rb) {
            (Some(a), Some(b)) => {
                let ra = &self.regions[a];
                let rb = &self.regions[b];
                ra.walkable && rb.walkable && ra.group == rb.group
            }
            _ => false,
        }
    }

    fn build_neighbors(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;

        // Collect neighbor pairs from adjacent tiles.
        let mut neighbor_set: Vec<std::collections::HashSet<usize>> =
            vec![std::collections::HashSet::new(); self.regions.len()];

        for ty in 0..h {
            for tx in 0..w {
                let idx = ty * w + tx;
                let ri = self.tile_region[idx] as usize;

                for (nx, ny) in [(tx.wrapping_sub(1), ty), (tx + 1, ty), (tx, ty.wrapping_sub(1)), (tx, ty + 1)]
                {
                    if nx >= w || ny >= h {
                        continue;
                    }
                    let ni = ny * w + nx;
                    let rj = self.tile_region[ni] as usize;
                    if rj != ri && rj < self.regions.len() {
                        neighbor_set[ri].insert(rj);
                    }
                }
            }
        }

        for (i, nset) in neighbor_set.into_iter().enumerate() {
            self.regions[i].neighbors = nset.into_iter().collect();
        }
    }

    fn build_groups(&mut self) {
        let mut group_id = 0usize;
        let mut visited = vec![false; self.regions.len()];

        for start in 0..self.regions.len() {
            if visited[start] || !self.regions[start].walkable {
                continue;
            }

            // DFS over walkable neighbors.
            let mut stack = vec![start];
            while let Some(ri) = stack.pop() {
                if visited[ri] {
                    continue;
                }
                visited[ri] = true;
                self.regions[ri].group = group_id;

                for &ni in &self.regions[ri].neighbors.clone() {
                    if !visited[ni] && self.regions[ni].walkable {
                        stack.push(ni);
                    }
                }
            }
            group_id += 1;
        }

        // Assign unique groups to unwalkable regions.
        for r in &mut self.regions {
            if !r.walkable {
                r.group = group_id;
                group_id += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::MiniTile;
    use crate::tileset::{CV5_ENTRY_SIZE, VF4_ENTRY_SIZE};

    fn build_vf4_entry(flags: &[u16; 16]) -> Vec<u8> {
        let mut entry = vec![0u8; VF4_ENTRY_SIZE];
        for (j, &f) in flags.iter().enumerate() {
            entry[j * 2..j * 2 + 2].copy_from_slice(&f.to_le_bytes());
        }
        entry
    }

    fn build_cv5_entry(mega_tile_indices: &[u16; 16]) -> Vec<u8> {
        let mut entry = vec![0u8; CV5_ENTRY_SIZE];
        for (j, &idx) in mega_tile_indices.iter().enumerate() {
            entry[20 + j * 2..22 + j * 2].copy_from_slice(&idx.to_le_bytes());
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

    /// Build a map where specific tiles are walkable vs unwalkable.
    /// `walkable_mask[ty * width + tx]` = true if tile is walkable.
    fn build_map_with_mask(width: u16, height: u16, walkable_mask: &[bool]) -> Map {
        // VF4: entry 0 = all walkable, entry 1 = all unwalkable
        let mut vf4 = build_vf4_entry(&[MiniTile::WALKABLE; 16]);
        vf4.extend_from_slice(&build_vf4_entry(&[0u16; 16]));

        // CV5: group 0, subtile 0 -> megatile 0 (walkable), subtile 1 -> megatile 1 (unwalkable)
        let mut indices = [0u16; 16];
        indices[0] = 0; // walkable
        indices[1] = 1; // unwalkable
        let cv5 = build_cv5_entry(&indices);

        // MTXM: walkable tiles get tile_id 0x0000, unwalkable get 0x0001
        let mut mtxm = Vec::new();
        for &w in walkable_mask {
            let tile_id: u16 = if w { 0x0000 } else { 0x0001 };
            mtxm.extend_from_slice(&tile_id.to_le_bytes());
        }

        let mut chk = build_section(b"DIM ", &{
            let mut d = Vec::new();
            d.extend_from_slice(&width.to_le_bytes());
            d.extend_from_slice(&height.to_le_bytes());
            d
        });
        chk.extend_from_slice(&build_section(b"ERA ", &[0, 0]));
        chk.extend_from_slice(&build_section(b"MTXM", &mtxm));

        Map::from_chk(&chk, &cv5, &vf4).unwrap()
    }

    #[test]
    fn test_fully_walkable_map() {
        // 4x4 all walkable → 1 walkable region
        let mask = vec![true; 16];
        let map = build_map_with_mask(4, 4, &mask);
        let rm = RegionMap::from_map(&map);

        let walkable_regions: Vec<_> = rm.regions.iter().filter(|r| r.walkable).collect();
        assert_eq!(walkable_regions.len(), 1);
        assert_eq!(walkable_regions[0].tile_count, 16);
    }

    #[test]
    fn test_wall_splits_regions() {
        // 6x4 map with a vertical wall at column 3:
        // WWW|WWW
        // WWW|WWW
        // WWW|WWW
        // WWW|WWW
        // W = walkable, | = unwalkable column
        let w = 6u16;
        let h = 4u16;
        let mut mask = vec![true; (w * h) as usize];
        for y in 0..h {
            mask[(y * w + 3) as usize] = false; // column 3 unwalkable
        }
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        let walkable_regions: Vec<_> = rm.regions.iter().filter(|r| r.walkable).collect();
        // Should be 2 walkable regions (left and right of wall)
        assert_eq!(walkable_regions.len(), 2);
        // They should be in different groups (not connected)
        assert_ne!(walkable_regions[0].group, walkable_regions[1].group);
    }

    #[test]
    fn test_corridor_connects_regions() {
        // 6x4 map with a wall that has a gap:
        // WWWWWW
        // WWW.WW   (. = unwalkable)
        // WWW.WW
        // WWWWWW
        let w = 6u16;
        let h = 4u16;
        let mut mask = vec![true; (w * h) as usize];
        mask[(1 * w + 3) as usize] = false;
        mask[(2 * w + 3) as usize] = false;
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        // Should still be 1 walkable group (connected via top and bottom rows)
        let walkable_groups: Vec<_> = rm.regions.iter().filter(|r| r.walkable).map(|r| r.group).collect();
        let first = walkable_groups[0];
        assert!(walkable_groups.iter().all(|&g| g == first));
    }

    #[test]
    fn test_region_at_px() {
        let mask = vec![true; 16];
        let map = build_map_with_mask(4, 4, &mask);
        let rm = RegionMap::from_map(&map);

        // Any pixel inside the map should map to a region.
        assert!(rm.region_at_px(16, 16).is_some());
        // Out of bounds.
        assert!(rm.region_at_px(-1, 0).is_none());
        assert!(rm.region_at_px(200, 200).is_none());
    }

    #[test]
    fn test_reachable() {
        // Wall splits map into two unreachable halves.
        let w = 6u16;
        let h = 4u16;
        let mut mask = vec![true; (w * h) as usize];
        for y in 0..h {
            mask[(y * w + 3) as usize] = false;
        }
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        // Left side to left side: reachable.
        assert!(rm.reachable(16, 16, 48, 48));
        // Left side to right side: unreachable.
        assert!(!rm.reachable(16, 16, 160, 16));
    }

    #[test]
    fn test_neighbors() {
        // 4x4, wall at column 2: columns 0-1 walkable, column 2 unwalkable, column 3 walkable
        let w = 4u16;
        let h = 4u16;
        let mut mask = vec![true; (w * h) as usize];
        for y in 0..h {
            mask[(y * w + 2) as usize] = false;
        }
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        // Find the wall region (unwalkable)
        let wall_region = rm.regions.iter().find(|r| !r.walkable).unwrap();
        // Wall should neighbor both walkable regions
        assert!(wall_region.neighbors.len() >= 2);
    }
}
