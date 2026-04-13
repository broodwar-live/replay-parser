use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::map::Map;
use crate::regions::RegionMap;

/// Maximum tiles the tile-level A* will explore before giving up.
const TILE_ASTAR_MAX_NODES: usize = 2048;

/// Find a path from source to destination in pixel coords.
///
/// Uses tile-level A* for precise paths. Falls back to region-center
/// waypoints if the tile search exceeds its node budget.
///
/// Returns `None` if unreachable or out of bounds.
pub fn find_path(
    map: &Map,
    region_map: &RegionMap,
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
) -> Option<Vec<(i32, i32)>> {
    // Bounds / reachability check via regions.
    let src_ri = region_map.region_at_px(src_x, src_y)?;
    let dst_ri = region_map.region_at_px(dst_x, dst_y)?;
    let src_region = region_map.region(src_ri)?;
    let dst_region = region_map.region(dst_ri)?;

    if !src_region.walkable || !dst_region.walkable {
        return None;
    }
    if src_ri != dst_ri && src_region.group != dst_region.group {
        return None;
    }

    // Same tile: go directly.
    if src_x / 32 == dst_x / 32 && src_y / 32 == dst_y / 32 {
        return Some(vec![(dst_x, dst_y)]);
    }

    // Try tile-level A* first (precise path).
    if let Some(path) = find_tile_path(map, src_x, src_y, dst_x, dst_y) {
        return Some(path);
    }

    // Fall back to region-center waypoints.
    find_region_path(region_map, src_x, src_y, dst_x, dst_y, src_ri, dst_ri)
}

/// Tile-level A* with 8-directional movement.
///
/// Operates on the tile grid (32x32 px). Returns waypoints as pixel coords
/// (tile centers). Returns `None` if no path found within the node budget.
fn find_tile_path(
    map: &Map,
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
) -> Option<Vec<(i32, i32)>> {
    let w = map.width() as i32;
    let h = map.height() as i32;

    let src_tx = (src_x / 32).clamp(0, w - 1);
    let src_ty = (src_y / 32).clamp(0, h - 1);
    let dst_tx = (dst_x / 32).clamp(0, w - 1);
    let dst_ty = (dst_y / 32).clamp(0, h - 1);

    if !map.is_tile_passable(dst_tx as u16, dst_ty as u16) {
        return None;
    }

    let tile_count = (w * h) as usize;
    let idx = |tx: i32, ty: i32| -> usize { (ty * w + tx) as usize };

    let mut g_cost = vec![f32::INFINITY; tile_count];
    let mut came_from = vec![u32::MAX; tile_count];
    let mut closed = vec![false; tile_count];
    let mut nodes_explored: usize = 0;

    let si = idx(src_tx, src_ty);
    g_cost[si] = 0.0;

    let mut open = BinaryHeap::new();
    open.push(TileNode {
        index: si as u32,
        f_cost: tile_dist(src_tx, src_ty, dst_tx, dst_ty),
    });

    // 8 directions: dx, dy, cost multiplier (1.0 for cardinal, ~1.414 for diagonal).
    const DIRS: [(i32, i32, f32); 8] = [
        (0, -1, 1.0),
        (1, 0, 1.0),
        (0, 1, 1.0),
        (-1, 0, 1.0),
        (1, -1, 1.414),
        (1, 1, 1.414),
        (-1, 1, 1.414),
        (-1, -1, 1.414),
    ];

    while let Some(current) = open.pop() {
        let ci = current.index as usize;
        if ci == idx(dst_tx, dst_ty) {
            // Reconstruct path.
            return Some(reconstruct_tile_path(
                &came_from, ci, si, w, src_x, src_y, dst_x, dst_y,
            ));
        }
        if closed[ci] {
            continue;
        }
        closed[ci] = true;
        nodes_explored += 1;

        if nodes_explored >= TILE_ASTAR_MAX_NODES {
            return None; // Budget exceeded, fall back.
        }

        let cx = (ci % w as usize) as i32;
        let cy = (ci / w as usize) as i32;

        for &(ddx, ddy, step_cost) in &DIRS {
            let nx = cx + ddx;
            let ny = cy + ddy;
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            if !map.is_tile_passable(nx as u16, ny as u16) {
                continue;
            }

            // For diagonal moves, both adjacent cardinal tiles must be passable
            // (prevents cutting through diagonal wall corners).
            if ddx != 0 && ddy != 0 {
                if !map.is_tile_passable((cx + ddx) as u16, cy as u16)
                    || !map.is_tile_passable(cx as u16, (cy + ddy) as u16)
                {
                    continue;
                }
            }

            let ni = idx(nx, ny);
            if closed[ni] {
                continue;
            }

            let tentative_g = g_cost[ci] + step_cost;
            if tentative_g < g_cost[ni] {
                g_cost[ni] = tentative_g;
                came_from[ni] = ci as u32;
                let h = tile_dist(nx, ny, dst_tx, dst_ty);
                open.push(TileNode {
                    index: ni as u32,
                    f_cost: tentative_g + h,
                });
            }
        }
    }

    None // No path found.
}

/// Reconstruct tile path and convert to pixel waypoints.
/// Simplifies the path by removing collinear waypoints.
fn reconstruct_tile_path(
    came_from: &[u32],
    goal: usize,
    start: usize,
    w: i32,
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
) -> Vec<(i32, i32)> {
    let mut tiles = Vec::new();
    let mut current = goal;
    while current != start {
        tiles.push(current);
        current = came_from[current] as usize;
    }
    tiles.reverse();

    // Convert to pixel coords (tile centers) and simplify.
    let mut waypoints = Vec::new();
    let mut prev_dx: i32 = 0;
    let mut prev_dy: i32 = 0;
    let mut prev_px = src_x;
    let mut prev_py = src_y;

    for (i, &ti) in tiles.iter().enumerate() {
        let tx = (ti % w as usize) as i32;
        let ty = (ti / w as usize) as i32;
        let px = tx * 32 + 16;
        let py = ty * 32 + 16;

        let dx = px - prev_px;
        let dy = py - prev_py;

        // Keep waypoint if direction changed or it's the last tile.
        if i == tiles.len() - 1 || dx != prev_dx || dy != prev_dy {
            if i > 0 && (prev_dx != 0 || prev_dy != 0) && (dx != prev_dx || dy != prev_dy) {
                // Add the previous tile center as a turn point.
                let prev_ti = tiles[i - 1];
                let ptx = (prev_ti % w as usize) as i32;
                let pty = (prev_ti / w as usize) as i32;
                let wp = (ptx * 32 + 16, pty * 32 + 16);
                if waypoints.last() != Some(&wp) {
                    waypoints.push(wp);
                }
            }
            prev_dx = dx;
            prev_dy = dy;
        }
        prev_px = px;
        prev_py = py;
    }

    // Final waypoint is the exact destination (not tile center).
    waypoints.push((dst_x, dst_y));
    waypoints
}

fn tile_dist(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let dx = (ax - bx) as f32;
    let dy = (ay - by) as f32;
    (dx * dx + dy * dy).sqrt()
}

/// Region-level A* fallback (original algorithm).
fn find_region_path(
    region_map: &RegionMap,
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
    src_ri: usize,
    dst_ri: usize,
) -> Option<Vec<(i32, i32)>> {
    if src_ri == dst_ri {
        return Some(vec![(dst_x, dst_y)]);
    }

    let region_count = region_map.regions.len();
    let mut g_cost = vec![f64::INFINITY; region_count];
    let mut came_from = vec![usize::MAX; region_count];
    let mut closed = vec![false; region_count];

    g_cost[src_ri] = 0.0;

    let mut open = BinaryHeap::new();
    open.push(RegionNode {
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
                open.push(RegionNode {
                    region: ni,
                    f_cost: tentative_g + h,
                });
            }
        }
    }

    if came_from[dst_ri] == usize::MAX {
        return None;
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

    let mut waypoints = Vec::with_capacity(path_regions.len() + 1);
    for &ri in &path_regions {
        if ri == dst_ri {
            waypoints.push((dst_x, dst_y));
        } else {
            let r = &region_map.regions[ri];
            waypoints.push((r.center_x, r.center_y));
        }
    }
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
struct TileNode {
    index: u32,
    f_cost: f32,
}

impl Eq for TileNode {}

impl PartialOrd for TileNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TileNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

#[derive(PartialEq)]
struct RegionNode {
    region: usize,
    f_cost: f64,
}

impl Eq for RegionNode {}

impl PartialOrd for RegionNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RegionNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_same_tile_direct() {
        let map = build_map_with_mask(4, 4, &vec![true; 16]);
        let rm = RegionMap::from_map(&map);
        let path = find_path(&map, &rm, 20, 20, 30, 30).unwrap();
        assert_eq!(path, vec![(30, 30)]);
    }

    #[test]
    fn test_straight_line() {
        let map = build_map_with_mask(8, 4, &vec![true; 32]);
        let rm = RegionMap::from_map(&map);
        let path = find_path(&map, &rm, 16, 48, 208, 48).unwrap();
        // Should be a relatively direct path.
        assert!(!path.is_empty());
        assert_eq!(*path.last().unwrap(), (208, 48));
    }

    #[test]
    fn test_unreachable() {
        let w = 6u16;
        let h = 4u16;
        let mut mask = vec![true; (w * h) as usize];
        for y in 0..h {
            mask[(y * w + 3) as usize] = false;
        }
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);
        assert!(find_path(&map, &rm, 16, 16, 160, 16).is_none());
    }

    #[test]
    fn test_path_around_wall() {
        // 8x6 map with a partial wall at column 4, rows 1-3:
        //   01234567
        // 0 WWWWWWWW
        // 1 WWWW.WWW
        // 2 WWWW.WWW
        // 3 WWWW.WWW
        // 4 WWWWWWWW
        // 5 WWWWWWWW
        let w = 8u16;
        let h = 6u16;
        let mut mask = vec![true; (w * h) as usize];
        mask[(1 * w + 4) as usize] = false;
        mask[(2 * w + 4) as usize] = false;
        mask[(3 * w + 4) as usize] = false;

        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        let src_x = 2 * 32 + 16;
        let src_y = 2 * 32 + 16;
        let dst_x = 6 * 32 + 16;
        let dst_y = 2 * 32 + 16;

        let path = find_path(&map, &rm, src_x, src_y, dst_x, dst_y);
        assert!(path.is_some(), "path should exist");

        let waypoints = path.unwrap();
        assert_eq!(*waypoints.last().unwrap(), (dst_x, dst_y));

        // Verify no waypoint is on the wall (column 4).
        let wall_x_min = 4 * 32;
        let wall_x_max = 5 * 32;
        for &(wx, wy) in &waypoints {
            let in_wall_col = wx >= wall_x_min && wx < wall_x_max;
            let in_wall_rows = wy >= 1 * 32 && wy < 4 * 32;
            assert!(
                !(in_wall_col && in_wall_rows),
                "waypoint ({}, {}) is inside the wall",
                wx,
                wy
            );
        }
    }

    #[test]
    fn test_diagonal_corner_cutting_blocked() {
        // 3x3 map with diagonal wall:
        // W.W
        // .W.    (. = unwalkable)
        // W.W
        // Only the center tile is walkable, plus corners.
        // Should NOT be able to path from (0,0) to (2,2) because diagonal
        // movement requires both adjacent cardinal tiles to be passable.
        let w = 3u16;
        let h = 3u16;
        let mask = vec![
            true, false, true, // row 0
            false, true, false, // row 1
            true, false, true, // row 2
        ];
        let map = build_map_with_mask(w, h, &mask);
        let rm = RegionMap::from_map(&map);

        let path = find_path(&map, &rm, 16, 16, 80, 80);
        assert!(path.is_none(), "should not be able to cut through diagonal walls");
    }

    #[test]
    fn test_out_of_bounds() {
        let map = build_map_with_mask(4, 4, &vec![true; 16]);
        let rm = RegionMap::from_map(&map);
        assert!(find_path(&map, &rm, -10, -10, 16, 16).is_none());
    }
}
