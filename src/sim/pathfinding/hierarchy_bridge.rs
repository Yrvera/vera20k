// Original gamemd582D70, full581F90 / local584550 callers, path walk429780.
// Evidence: PHASE3_TUBE_HIERARCHY_20260910.md and spatial_oracle/tube_hierarchy.

fn register_high_bridge_hierarchy_edges(
    edge_buckets: &mut HierarchyEdgeBuckets,
    zone_ids: &[ZoneId],
    terrain: &ResolvedTerrainGrid,
    bridge_records: &[BridgeEndpointRecord],
    width: u16,
    height: u16,
    source_size: Option<(i32, i32)>,
) {
    // Full581F90 uses forward order; local584550's reverse gated loop lives
    // at its caller and reaches the same per-record helper below.
    for record in bridge_records.iter().filter(|record| record.active) {
        register_bridge_hierarchy_edges_for_record(
            edge_buckets,
            zone_ids,
            terrain,
            record,
            width,
            height,
            source_size,
        );
    }
}

fn register_bridge_hierarchy_edges_for_record(
    edge_buckets: &mut HierarchyEdgeBuckets,
    zone_ids: &[ZoneId],
    terrain: &ResolvedTerrainGrid,
    record: &BridgeEndpointRecord,
    width: u16,
    height: u16,
    source_size: Option<(i32, i32)>,
) {
    use crate::sim::cell_rect::{CellRef, get_cellclass_fallback};
    let a = record.endpoint_a;
    let b = record.endpoint_b;
    let cell = get_cellclass_fallback(Some(terrain), i32::from(a.0 as i16), i32::from(a.1 as i16));
    let high_offset = match cell {
        CellRef::Real(cell) => terrain.high_bridge_tile_offset(cell),
        // The hash-bound retail theater ranges exclude constructor tile65535.
        CellRef::Dummy { .. } => None,
    };
    let (side_a, opposite_a, side_b, opposite_b) = if let Some(offset) = high_offset {
        let direction = i32::from(HIGH_BRIDGE_HIERARCHY_DIRECTIONS[offset]);
        (
            hierarchy_side_coord(a, direction),
            hierarchy_side_coord(a, direction.wrapping_sub(4)),
            hierarchy_side_coord(b, direction),
            hierarchy_side_coord(b, direction.wrapping_sub(4)),
        )
    } else {
        // Active producer records supply a center Tube. Invalid externally
        // supplied records return safely instead of native's null dereference.
        let Some(tube) = terrain.tube_at_native_coord(a) else {
            return;
        };
        let side = hierarchy_side_coord(a, tube.direction.wrapping_add(2));
        let opposite = hierarchy_side_coord(a, tube.direction.wrapping_sub(2));
        // Both lookups happen before either null test, including dummy writes.
        let side_tube = terrain.tube_at_native_coord(side);
        let opposite_tube = terrain.tube_at_native_coord(opposite);
        let (Some(side_tube), Some(opposite_tube)) = (side_tube, opposite_tube) else {
            return;
        };
        let far =
            hierarchy_walk_tube_path(terrain, side, &side_tube.path_steps).and_then(|far_side| {
                hierarchy_walk_tube_path(terrain, opposite, &opposite_tube.path_steps)
                    .map(|far_opposite| (far_side, far_opposite))
            });
        let (far_side, far_opposite) = match far {
            Ok(far) => far,
            Err(error) => {
                // Explicit parity residual: retain the prior absent-pair result
                // for unmodeled process-memory reads; never mask or panic.
                log::warn!("unresolved native Tube hierarchy path read: {error:?}");
                return;
            }
        };
        (side, opposite, far_side, far_opposite)
    };
    debug_assert_eq!(zone_ids.len(), usize::from(width) * usize::from(height));
    for (from, to) in [(a, b), (side_a, side_b), (opposite_a, opposite_b)] {
        let from = bridge_endpoint_base_zone(zone_ids, width, source_size, from).unwrap_or(0);
        let to = bridge_endpoint_base_zone(zone_ids, width, source_size, to).unwrap_or(0);
        register_native_hierarchy_pair(edge_buckets, from, to, 0);
    }
}

fn hierarchy_side_coord(coord: (u16, u16), direction: i32) -> (u16, u16) {
    let (dx, dy) = crate::util::direction::DIRECTION_DELTAS[(direction as u32 & 7) as usize];
    (
        coord.0.wrapping_add(dx as u16),
        coord.1.wrapping_add(dy as u16),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HierarchyPathReadError {
    DirectionData { raw_token: i32, native_address: u32 },
    TubeIndex(i16),
}

fn hierarchy_walk_tube_path(
    terrain: &ResolvedTerrainGrid,
    mut coord: (u16, u16),
    steps: &[i32],
) -> Result<(u16, u16), HierarchyPathReadError> {
    for &step in steps {
        if step == 8 {
            let index = terrain.raw_tube_index_at_native_coord(coord);
            coord = if index == -1 {
                (0, 0)
            } else {
                terrain
                    .tube(crate::map::tube_facts::TubeId(index as u16))
                    .filter(|_| index >= 0)
                    .ok_or(HierarchyPathReadError::TubeIndex(index))?
                    .exit
            };
        } else {
            // x86 effective address89F688 + step*4 wraps in32 bits. Exact8
            // above is special; address aliases of slot8 are ordinary reads.
            let slot = step as u32 & 0x3fff_ffff;
            let (dx, dy) = match slot {
                0..=7 => crate::util::direction::DIRECTION_DELTAS[slot as usize],
                // Original49F2D0/49F280 initialize slots8..12 to0; slot13
                // is zero BSS with no direct xrefs in the examined image.
                8..=13 => (0, 0),
                _ => {
                    return Err(HierarchyPathReadError::DirectionData {
                        raw_token: step,
                        native_address: 0x0089_f688u32.wrapping_add((step as u32).wrapping_mul(4)),
                    });
                }
            };
            coord = (
                coord.0.wrapping_add(dx as u16),
                coord.1.wrapping_add(dy as u16),
            );
        }
    }
    Ok(coord)
}

fn register_native_hierarchy_pair(
    buckets: &mut HierarchyEdgeBuckets,
    a: ZoneId,
    b: ZoneId,
    flag: u8,
) {
    //582FFD..583012 sign-extends both words before OR. Preserve exact packed
    //transcript, including the native high-bit zone-ID limitation.
    let bucket = (((a & 15) << 4) | (b & 15)) as usize;
    let packed = (u32::from(a) << 16) | (i32::from(b as i16) as u32);
    let edge = HierarchyTempEdge {
        existing: (packed >> 16) as u16,
        current: packed as u16,
        flag,
    };
    if !buckets.buckets[bucket]
        .iter()
        .any(|other| other.existing == edge.existing && other.current == edge.current)
    {
        // No zero/equality filter. First exact key retains its flag and place.
        buckets.buckets[bucket].push(edge);
    }
}
