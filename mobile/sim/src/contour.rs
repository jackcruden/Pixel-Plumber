//! Marching squares over the density field. Polygons are a derived cache,
//! never authoritative — nothing in the simulation depends on them. They feed
//! debug rendering, the harness overlays, and (later) rigid-body colliders.
//!
//! The field is padded with one ring of empty space, so every emitted contour
//! is a closed loop by construction.

use crate::field::DensityField;
use std::collections::BTreeMap;

/// A closed polyline in cell-space coordinates.
pub type Loop = Vec<(f32, f32)>;

/// Node value for marching squares: density at cell centres, one empty ring
/// of padding around the field so all loops close.
#[inline]
fn node(field: &DensityField, x: i32, y: i32) -> f32 {
    if x < 0 || y < 0 || x >= field.width as i32 || y >= field.height as i32 {
        -1.0
    } else {
        field.density[y as usize * field.width + x as usize]
    }
}

/// Edge identity on the padded node lattice: (x, y, 0=horizontal from
/// (x,y)→(x+1,y), 1=vertical from (x,y)→(x,y+1)). Keying vertices by lattice
/// edge (not by float quantisation) makes stitching exact and deterministic.
type EdgeId = (i32, i32, u8);

fn interp(a: f32, b: f32) -> f32 {
    // Zero crossing between node values a (at 0) and b (at 1).
    let d = b - a;
    if d.abs() < 1e-12 {
        0.5
    } else {
        (-a / d).clamp(0.0, 1.0)
    }
}

fn edge_point(field: &DensityField, e: EdgeId) -> (f32, f32) {
    let (x, y, dir) = e;
    let a = node(field, x, y);
    if dir == 0 {
        let b = node(field, x + 1, y);
        (x as f32 + 0.5 + interp(a, b), y as f32 + 0.5)
    } else {
        let b = node(field, x, y + 1);
        (x as f32 + 0.5, y as f32 + 0.5 + interp(a, b))
    }
}

/// Extract all iso-contours of the field at threshold 0 as closed loops.
pub fn contour_full(field: &DensityField) -> Vec<Loop> {
    contour_full_checked(field).0
}

/// As `contour_full`, but also reports whether every walked loop actually
/// closed back onto its first vertex (the contouring-validity property).
pub fn contour_full_checked(field: &DensityField) -> (Vec<Loop>, bool) {
    let w = field.width as i32;
    let h = field.height as i32;
    // Segments as pairs of edge ids.
    let mut segments: Vec<(EdgeId, EdgeId)> = Vec::new();
    // Marching cells span nodes (x..x+1, y..y+1) over the padded lattice.
    for y in -1..h {
        for x in -1..w {
            let tl = node(field, x, y) > 0.0;
            let tr = node(field, x + 1, y) > 0.0;
            let br = node(field, x + 1, y + 1) > 0.0;
            let bl = node(field, x, y + 1) > 0.0;
            let case = tl as u8 | (tr as u8) << 1 | (br as u8) << 2 | (bl as u8) << 3;
            if case == 0 || case == 15 {
                continue;
            }
            let top: EdgeId = (x, y, 0);
            let bottom: EdgeId = (x, y + 1, 0);
            let left: EdgeId = (x, y, 1);
            let right: EdgeId = (x + 1, y, 1);
            let mut push = |a: EdgeId, b: EdgeId| segments.push((a, b));
            match case {
                1 => push(left, top),
                2 => push(top, right),
                3 => push(left, right),
                4 => push(right, bottom),
                5 => {
                    // Ambiguous: resolve with the cell-centre average.
                    let c = (node(field, x, y)
                        + node(field, x + 1, y)
                        + node(field, x + 1, y + 1)
                        + node(field, x, y + 1))
                        * 0.25;
                    if c > 0.0 {
                        push(left, bottom);
                        push(top, right);
                    } else {
                        push(left, top);
                        push(right, bottom);
                    }
                }
                6 => push(top, bottom),
                7 => push(left, bottom),
                8 => push(bottom, left),
                9 => push(top, bottom),
                10 => {
                    let c = (node(field, x, y)
                        + node(field, x + 1, y)
                        + node(field, x + 1, y + 1)
                        + node(field, x, y + 1))
                        * 0.25;
                    if c > 0.0 {
                        push(left, top);
                        push(right, bottom);
                    } else {
                        push(left, bottom);
                        push(top, right);
                    }
                }
                11 => push(bottom, right),
                12 => push(right, left),
                13 => push(top, right),
                14 => push(left, top),
                _ => unreachable!(),
            }
        }
    }

    // Stitch segments into loops. Every contour vertex lies on a unique
    // lattice edge, and (away from degenerate exact-zero nodes) each edge is
    // used by exactly two segments, so walking neighbours closes every loop.
    let mut incident: BTreeMap<EdgeId, Vec<usize>> = BTreeMap::new();
    for (i, (a, b)) in segments.iter().enumerate() {
        incident.entry(*a).or_default().push(i);
        incident.entry(*b).or_default().push(i);
    }
    let mut used = vec![false; segments.len()];
    let mut loops = Vec::new();
    let mut all_closed = true;
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let (first, mut cursor) = segments[start];
        let mut edge_ids = vec![first, cursor];
        let mut closed = false;
        loop {
            let next = incident[&cursor]
                .iter()
                .copied()
                .find(|&s| !used[s]);
            let Some(seg) = next else { break };
            used[seg] = true;
            let (a, b) = segments[seg];
            cursor = if a == cursor { b } else { a };
            if cursor == first {
                closed = true;
                break;
            }
            edge_ids.push(cursor);
        }
        all_closed &= closed;
        loops.push(edge_ids.iter().map(|&e| edge_point(field, e)).collect());
    }
    (loops, all_closed)
}
