# Unit Can Enter Cell Wall And Overlay Follow-Up Scope

## Purpose

Keep wall, gate, and overlay-specific `UnitClass::Can_Enter_Cell` parity out of the low-bridge TubeClass patch set. The binary branch is broad enough to need its own design and review pass before implementation.

## Verified Binary Inputs To Recheck

- `OverlayType+0x2AA` wall flag and adjacent overlay block fields.
- `OverlayType+0x2A8`, `OverlayType+0x22D`, and `OverlayType+0x9C` gate, damage, and block-related fields.
- Unit crusher flags and movement-zone gates.
- Selected weapon and warhead flags, especially `Wall` and `Wood`.
- Overlay ownership/alliance for friendly-wall code `4` vs enemy/blocking responses.
- Gate open/closed state and whether the gate is allied, neutral, or hostile.

## Rust Dependencies

- Rules parser coverage for overlay, weapon, and warhead flags.
- Overlay registry/runtime overlay grid.
- Gate state and ownership model.
- `src/sim/pathfinding/cell_entry.rs` return-code producers.
- Movement response consumers in `src/sim/movement/movement_occupancy.rs`.
- Combat/targeting behavior for attackable blocking overlays.

## Acceptance For Starting Implementation

Implementation should start only after a dedicated design and review-plan pass confirms the binary branch ordering, required Rust data, and tests for friendly walls, enemy walls, crushable overlays, closed gates, open gates, and wall-weapon interactions.
