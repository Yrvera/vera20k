//! Safe lowering from native-shaped tactical plans to existing GPU buffers.
//!
//! The adapter only reorders families that retain all native ordering metadata.
//! It refuses lossy atlas vectors rather than inferring coordinates or registration.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::render::batch::SpriteInstance;
use crate::render::tactical_draw_plan::{
    BlitPolicy, BuildingOwnedPlan, BuildingPiece, BuildingPieceKind, CellDraw, CellDrawKind,
    DrawId, ObjectDraw, TacticalCoord, TacticalDrawInput, TacticalDrawPlan, TacticalLayer,
};

/// A cell-pass instance paired with the metadata needed by `YR TacticalClass::Draw`.
pub(crate) struct PlannedCellInstance {
    pub draw: CellDraw,
    pub instance: SpriteInstance,
}

/// One GPU sprite that remains owned by a single building during lowering.
///
/// Keeping the atlas page alongside the sprite lets the render submission
/// preserve `BuildingClass::Draw` piece order without inferring it from floats.
pub(crate) struct PlannedBuildingPieceInstance {
    pub kind: BuildingPieceKind,
    pub z_bias: i32,
    pub policy: BlitPolicy,
    pub page: usize,
    pub instance: SpriteInstance,
}

/// Families whose existing buffers cannot be safely promoted to `LayerClass` ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectOrderFamily {
    UnitAtlas,
    ShpAtlas,
    SlopeTransitionAtlas,
}

/// A deliberate refusal to guess missing `ObjectClass::GetYSort` inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectOrderRefusal {
    MissingTacticalCoord { family: ObjectOrderFamily },
    MissingRegistrationOrder { family: ObjectOrderFamily },
    SplitOwnedBuildingGroup { family: ObjectOrderFamily },
}

/// Metadata required before a live object can enter the native integer sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeObjectOrderMetadata {
    pub id: DrawId,
    pub layer: TacticalLayer,
    pub coord: Option<TacticalCoord>,
    pub y_sort_adjust: i32,
    pub registration_order: Option<u64>,
}

pub(crate) fn object_draw_from_metadata(
    family: ObjectOrderFamily,
    metadata: NativeObjectOrderMetadata,
) -> Result<ObjectDraw, ObjectOrderRefusal> {
    let coord = metadata
        .coord
        .ok_or(ObjectOrderRefusal::MissingTacticalCoord { family })?;
    let registration_order = metadata
        .registration_order
        .ok_or(ObjectOrderRefusal::MissingRegistrationOrder { family })?;
    Ok(ObjectDraw {
        id: metadata.id,
        layer: metadata.layer,
        coord,
        y_sort_adjust: metadata.y_sort_adjust,
        registration_order,
        policy: crate::render::tactical_draw_plan::BlitPolicy::opaque(
            crate::render::tactical_draw_plan::SpriteEncoding::Plain,
        ),
    })
}

/// Return explicit fallbacks for the current GPU buffers that lost native keys.
pub(crate) fn object_buffer_fallbacks(
    has_units: bool,
    has_slope_transitions: bool,
    has_shp: bool,
) -> Vec<ObjectOrderRefusal> {
    let mut refusals = Vec::new();
    if has_units {
        refusals.push(ObjectOrderRefusal::MissingTacticalCoord {
            family: ObjectOrderFamily::UnitAtlas,
        });
    }
    if has_slope_transitions {
        refusals.push(ObjectOrderRefusal::MissingTacticalCoord {
            family: ObjectOrderFamily::SlopeTransitionAtlas,
        });
    }
    if has_shp {
        refusals.push(ObjectOrderRefusal::MissingTacticalCoord {
            family: ObjectOrderFamily::ShpAtlas,
        });
    }
    refusals
}

/// Report each lossless-lowering refusal once per process, not once per frame.
pub(crate) fn report_object_buffer_fallbacks(
    has_units: bool,
    has_slope_transitions: bool,
    has_shp: bool,
) {
    static REPORTED: AtomicU8 = AtomicU8::new(0);
    for refusal in object_buffer_fallbacks(has_units, has_slope_transitions, has_shp) {
        let bit = match refusal {
            ObjectOrderRefusal::MissingTacticalCoord {
                family: ObjectOrderFamily::UnitAtlas,
            } => 1,
            ObjectOrderRefusal::MissingTacticalCoord {
                family: ObjectOrderFamily::SlopeTransitionAtlas,
            } => 2,
            ObjectOrderRefusal::MissingTacticalCoord {
                family: ObjectOrderFamily::ShpAtlas,
            } => 4,
            _ => 0,
        };
        if REPORTED.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
            log::warn!(
                "native tactical object ordering fallback: {refusal:?}; retaining existing GPU buffer order"
            );
        }
    }
}

/// Lower a complete cell family while preserving the existing GPU instance type.
///
/// `TacticalDrawPlan` owns the family ordering; this adapter only maps ordered
/// IDs back to the existing instances. Duplicate IDs are rejected at the source.
pub(crate) fn lower_cell_instances(entries: Vec<PlannedCellInstance>) -> Vec<SpriteInstance> {
    let mut instances = BTreeMap::new();
    let inputs = entries.into_iter().map(|entry| {
        assert!(
            instances.insert(entry.draw.id, entry.instance).is_none(),
            "each planned cell instance must have a unique draw id"
        );
        TacticalDrawInput::Cell(entry.draw)
    });
    let plan = TacticalDrawPlan::build(inputs);
    let mut ordered = Vec::with_capacity(instances.len());
    for draw in plan
        .cell_pass
        .terrain
        .iter()
        .chain(&plan.cell_pass.smudges)
        .chain(&plan.cell_pass.overlays)
        .chain(&plan.cell_pass.primary_objects)
    {
        ordered.push(
            instances
                .remove(&draw.id)
                .expect("plan entry must resolve to its existing GPU instance"),
        );
    }
    ordered
}

/// Lower one intact building group through the native-shaped planner.
///
/// `YR BuildingClass::Draw` keeps its owned bib/body/anim draws together after
/// the parent object wins the TacticalClass layer sort.
pub(crate) fn lower_building_piece_instances(
    parent: ObjectDraw,
    entries: Vec<PlannedBuildingPieceInstance>,
) -> Vec<(usize, SpriteInstance)> {
    let mut instances = BTreeMap::new();
    let pieces = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let id = index as DrawId;
            assert!(
                instances.insert(id, (entry.page, entry.instance)).is_none(),
                "each planned building piece must have a unique local id"
            );
            BuildingPiece {
                id,
                kind: entry.kind,
                z_bias: entry.z_bias,
                policy: entry.policy,
            }
        })
        .collect();
    let plan = TacticalDrawPlan::build([TacticalDrawInput::Building(BuildingOwnedPlan {
        parent,
        pieces,
    })]);
    let mut ordered = Vec::with_capacity(instances.len());
    for entry in &plan.object_layers {
        for building in &entry.entries {
            let crate::render::tactical_draw_plan::LayerEntry::Building(building) = building else {
                unreachable!("building-only lowering cannot produce an object entry");
            };
            for piece in &building.pieces {
                ordered.push(
                    instances
                        .remove(&piece.id)
                        .expect("plan piece must resolve to its existing GPU instance"),
                );
            }
        }
    }
    ordered
}

pub(crate) fn cell_draw_kind(is_wall: bool) -> CellDrawKind {
    if is_wall {
        CellDrawKind::WallOverlay
    } else {
        CellDrawKind::FlatOverlay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::tactical_draw_plan::{BlitPolicy, RenderZPolicy, SpriteEncoding};

    fn cell(id: DrawId, is_wall: bool) -> PlannedCellInstance {
        PlannedCellInstance {
            draw: CellDraw {
                id,
                kind: cell_draw_kind(is_wall),
                policy: BlitPolicy::translucent(SpriteEncoding::Terrain, RenderZPolicy::None),
            },
            instance: SpriteInstance {
                fx_flags: id as u32,
                ..Default::default()
            },
        }
    }

    #[test]
    fn cell_lowering_keeps_walls_in_the_fixed_overlay_family() {
        let lowered = lower_cell_instances(vec![cell(4, false), cell(3, true), cell(2, false)]);
        assert_eq!(
            lowered
                .iter()
                .map(|instance| instance.fx_flags)
                .collect::<Vec<_>>(),
            [4, 3, 2]
        );
    }

    #[test]
    fn object_lowering_refuses_missing_integer_coordinates() {
        let refusal = object_draw_from_metadata(
            ObjectOrderFamily::ShpAtlas,
            NativeObjectOrderMetadata {
                id: 1,
                layer: TacticalLayer(2),
                coord: None,
                y_sort_adjust: 0,
                registration_order: Some(3),
            },
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ObjectOrderRefusal::MissingTacticalCoord {
                family: ObjectOrderFamily::ShpAtlas
            }
        );
    }

    #[test]
    fn object_lowering_refuses_missing_registration_order() {
        let refusal = object_draw_from_metadata(
            ObjectOrderFamily::UnitAtlas,
            NativeObjectOrderMetadata {
                id: 1,
                layer: TacticalLayer(2),
                coord: Some(TacticalCoord { x: 1, y: 2, z: 0 }),
                y_sort_adjust: 0,
                registration_order: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ObjectOrderRefusal::MissingRegistrationOrder {
                family: ObjectOrderFamily::UnitAtlas
            }
        );
    }

    #[test]
    fn non_empty_gpu_families_publish_typed_fallbacks() {
        assert_eq!(
            object_buffer_fallbacks(true, false, true),
            vec![
                ObjectOrderRefusal::MissingTacticalCoord {
                    family: ObjectOrderFamily::UnitAtlas,
                },
                ObjectOrderRefusal::MissingTacticalCoord {
                    family: ObjectOrderFamily::ShpAtlas,
                },
            ]
        );
    }

    #[test]
    fn building_lowering_keeps_owned_pieces_together_in_native_order() {
        let parent = ObjectDraw {
            id: 9,
            layer: TacticalLayer(2),
            coord: TacticalCoord {
                x: 512,
                y: 768,
                z: 0,
            },
            y_sort_adjust: 0,
            registration_order: 9,
            policy: BlitPolicy::opaque(SpriteEncoding::Plain),
        };
        let piece = |kind, page, marker| PlannedBuildingPieceInstance {
            kind,
            z_bias: 0,
            policy: BlitPolicy::opaque(SpriteEncoding::Plain),
            page,
            instance: SpriteInstance {
                fx_flags: marker,
                ..Default::default()
            },
        };

        let lowered = lower_building_piece_instances(
            parent,
            vec![
                piece(BuildingPieceKind::PoweredOrActiveOverlay, 2, 50),
                piece(BuildingPieceKind::Body, 1, 40),
                piece(BuildingPieceKind::Bib, 0, 30),
            ],
        );

        assert_eq!(
            lowered
                .iter()
                .map(|(page, instance)| (*page, instance.fx_flags))
                .collect::<Vec<_>>(),
            [(0, 30), (1, 40), (2, 50)]
        );
    }
}
