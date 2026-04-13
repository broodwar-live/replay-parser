use std::collections::BinaryHeap;
use std::cmp::Ordering;

use crate::regions::RegionMap;

/// A* over the region graph. Returns waypoints (pixel coords) from source to destination.
///
/// Returns `None` if source/destination are out of bounds or unreachable.
/// Returns `Some(vec)` with waypoints including the destination.
pub fn find_path(
    region_map: &RegionMap,
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
) -> Option<Vec<(i32, i32)>> {
    let src_ri = region_map.region_at_px(src_x, src_y)?;
    let dst_ri = region_map.region_at_px(dst_x, dst_y)?;

    let src_region = region_map.region(src_ri)?;
    let dst_region = region_map.region(dst_ri)?;

    // Both must be walkable.
    if !src_region.walkable || !dst_region.walkable {
        return None;
    }

    // Same region: go directly.
    if src_ri == dst_ri {
        return Some(vec![(dst_x, dst_y)]);
    }

    // Different groups: unreachable.
    if src_region.group != dst_region.group {
        return None;
    }

    // A* over regions.
    let region_count = region_map.regions.len();
    let mut g_cost = vec![f64::INFINITY; region_count];
    let mut came_from = vec![usize::MAX; region_count];
    let mut closed = vec![false; region_count];

    g_cost[src_ri] = 0.0;

    let mut open = BinaryHeap::new();
    open.push(AStarNode {
        region: src_ri,
        f_cost: dist(src_x, src_y, dst_x, dst_y),
    });

    while let Some(current) = open.pop() {
        let ri = current.region;
        if ri == dst_ri {
            break;
        }
        if closed[ri] {
            continue;
        }
        closed[ri] = true;

        let r = &region_map.regions[ri];
        let (rx, ry) = if ri == src_ri {
            (src_x, src_y)
        } else {
            (r.center_x, r.center_y)
        };

        for &ni in &r.neighbors {
            if closed[ni] || !region_map.regions[ni].walkable {
                continue;
            }
            let n = &region_map.regions[ni];
            let (nx, ny) = (n.center_x, n.center_y);

            let tentative_g = g_cost[ri] + dist(rx, ry, nx, ny);
            if tentative_g < g_cost[ni] {
                g_cost[ni] = tentative_g;
                came_from[ni] = ri;
                let h = dist(nx, ny, dst_x, dst_y);
                open.push(AStarNode {
                    region: ni,
                    f_cost: tentative_g + h,
                });
            }
        }
    }

    // Reconstruct path.
    if came_from[dst_ri] == usize::MAX && src_ri != dst_ri {
        return None; // No path found.
    }

    let mut path_regions = Vec::new();
    let mut current = dst_ri;
    while current != src_ri {
        path_regions.push(current);
        current = came_from[current];
        if current == usize::MAX {
            return None;
        }
    }
    path_regions.reverse();

    // Convert region sequence to waypoints.
    let mut waypoints = Vec::with_capacity(path_regions.len() + 1);
    for &ri in &path_regions {
        if ri == dst_ri {
            waypoints.push((dst_x, dst_y));
        } else {
            let r = &region_map.regions[ri];
            waypoints.push((r.center_x, r.center_y));
        }
    }
    // Ensure destination is the last waypoint.
    if waypoints.last() != Some(&(dst_x, dst_y)) {
        waypoints.push((dst_x, dst_y));
    }

    Some(waypoints)
}

fn dist(ax: i32, ay: i32, bx: i32, by: i32) -> f64 {
    let dx = (ax - bx) as f64;
    let dy = (ay - by) as f64;
    (dx * dx + dy * dy).sqrt()
}

#[derive(PartialEq)]
struct AStarNode {
    region: usize,
    f_cost: f64,
}

impl Eq for AStarNode {}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (BinaryHeap is max-heap).
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Map;
    use crate::regions::RegionMap;
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

    fn build_map_with_mask(width: u16, height: u16, walkable_mask: &[bool]) -> Map {
        let mut vf4 = build_vf4_entry(&[MiniTile::WALKABLE; 16]);
        vf4.extend_from_slice(&build_vf4_entry(&[0u16; 16]));
        let mut indices = [0u16; 16];
        indices[0] = 0;
        indices[1] = 1;
        let cv5 = build_cv5_entry(&indices);
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
    fn test_same_region_direct_path() {
        let map = build_map_with_mask(4, 4, &vec![true; 16]);
        let rm = RegionMap::from_map(&map);
        let path = find_path(&rm, 16, 16, 80, 80).unwrap();
        assert_eq!(path, vec![(80, 80)]);
    }

    #[test]
    fn test_unreachable() {
        // Full wall splits map.
        let w = 6u16;
        let h = 4u16;
        let mut mask = vec![true; (w * h) as usize];
        for y in 0..h {
            mask[(y * w + 3) as usize] = false;
        }
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        // Left to right: unreachable.
        assert!(find_path(&rm, 16, 16, 160, 16).is_none());
    }

    #[test]
    fn test_path_around_obstacle() {
        // 8x6 map with a partial wall:
        //   01234567
        // 0 WWWWWWWW
        // 1 WWWW.WWW   . = unwalkable
        // 2 WWWW.WWW
        // 3 WWWW.WWW
        // 4 WWWWWWWW   <- gap at bottom
        // 5 WWWWWWWW
        let w = 8u16;
        let h = 6u16;
        let mut mask = vec![true; (w * h) as usize];
        mask[(1 * w + 4) as usize] = false;
        mask[(2 * w + 4) as usize] = false;
        mask[(3 * w + 4) as usize] = false;

        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        // Move from left side (col 2) to right side (col 6), row 2.
        let src_x = 2 * 32 + 16; // pixel center of tile (2, 2)
        let src_y = 2 * 32 + 16;
        let dst_x = 6 * 32 + 16;
        let dst_y = 2 * 32 + 16;

        let path = find_path(&rm, src_x, src_y, dst_x, dst_y);
        assert!(path.is_some(), "path should exist (connected via row 0 and rows 4-5)");

        let waypoints = path.unwrap();
        assert!(!waypoints.is_empty());
        // Last waypoint should be the destination.
        assert_eq!(*waypoints.last().unwrap(), (dst_x, dst_y));
    }

    #[test]
    fn test_out_of_bounds() {
        let map = build_map_with_mask(4, 4, &vec![true; 16]);
        let rm = RegionMap::from_map(&map);
        assert!(find_path(&rm, -10, -10, 16, 16).is_none());
    }
}
