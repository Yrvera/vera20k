# Fire Origin Parity Plan

> **For Codex:** Execute this plan task-by-task. Each task is intended to be small enough to verify before moving on.

## Goal

Make every player-visible fire effect use the same RA2/YR-style weapon source origin:

- muzzle flash placement
- report sound position
- projectile spawn point
- projectile facing and travel timing
- garrison muzzle flash placement

The fix must cover both normal techno FLH paths and building-specific fire origin paths, including fixed building pixel offsets and garrison muzzle ports.

## Design Doc

- `docs/plans/2026-05-22-fire-origin-parity-design.md`

## Grounding Summary

Trace and research show the current Rust implementation computes only a screen-space visual offset for non-garrison fire effects while leaving the world origin at the attacker cell. This makes sounds, muzzle flashes, projectile spawn positions, projectile facing, and projectile timing diverge from `gamemd.exe`.

Verified source material:

- `docs/research/traces/FLH_MUZZLE_WORLD_REFERENCE_TRACE.md`
- `docs/research/FLH_TURRET_AND_VISUAL_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/BURST_WEAPON_FIRING_GHIDRA_REPORT.md`

Key verified facts:

- `TechnoClass::GetFLH` returns a world/lepton coordinate, not only a screen pixel offset.
- `CurrentBurstIndex` is the pre-shot burst index. For a two-shot burst, shot 1 uses index 0 and shot 2 uses index 1.
- Building `PrimaryFirePixelOffset` and `SecondaryFirePixelOffset` are screen pixel offsets converted through the engine's isometric pixel-to-world helper.
- Garrison muzzle ports are screen pixel pairs converted to world coordinates.
- `BuildingClass::GetRenderCoords` applies a `-0x80` X/Y render-coordinate shift before fire origin math.

## Architecture Rules

- `sim/` may snapshot deterministic gameplay facts, but it must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or app presentation code.
- Art/rules metadata lookup and visual coordinate conversion stay above `sim/`.
- Do not use current post-tick entity state to resolve a fire event origin. The event must carry the source snapshot needed for replay-stable visual/audio placement.
- Missing metadata must be explicit. Do not silently fall back to attacker cell center for a branch that is supposed to use FLH, building pixel offsets, turret origin, or garrison muzzle ports.

## File Map

Likely touched files:

- `src/sim/world/mod.rs`
- `src/sim/combat/mod.rs`
- `src/app_fire_effects.rs`
- `src/app_sim_tick.rs`
- `src/rules/art_data.rs`
- `src/util/flh_transform.rs`
- top-level module file that declares `app_fire_effects`

Possible new file:

- `src/app_fire_origin.rs`

Test files may be colocated in existing module test sections unless a new integration fixture is clearer.

## Interface Changes

Add a fire-origin snapshot to `SimFireEvent`, with enough deterministic source facts to resolve origin later:

- attacker cell or render cell
- attacker subcell/lepton offset if available
- attacker Z
- firing facing
- attacker category
- pre-shot burst index

The exact field names can follow local style, but the structure should make this invariant obvious:

```rust
SimFireEvent {
    origin_snapshot: FireOriginSnapshot,
    ...
}
```

Add an app-side resolved origin type:

```rust
FireWorldOrigin {
    rx,
    ry,
    sub_x,
    sub_y,
    z,
    screen_x,
    screen_y,
    branch,
}
```

`branch` should be diagnostic-only and useful in tests/logging, for example `Flh`, `BuildingPixelOffset`, `BuildingTurret`, or `GarrisonPort`.

## Task 1: Add Fire Origin Snapshot Types

Add `FireOriginSnapshot` near `SimFireEvent` in `src/sim/world/mod.rs`.

Requirements:

- Keep it presentation-agnostic.
- Include pre-shot burst index, not post-shot value.
- Derive the same traits as nearby event structs need for drain/clone/debug behavior.
- Update any test fixture construction of `SimFireEvent`.

Verification:

- `cargo test -p ra2-rust-game sim_fire_event` or the closest focused test target if no matching target exists.
- Full `cargo check` if focused tests are not discoverable.

## Task 2: Emit Snapshot And Pre-Shot Burst Index

Update `src/sim/combat/mod.rs` where `SimFireEvent` is emitted.

Requirements:

- Compute weapon burst count before event emission.
- Derive pre-shot burst index from the current burst state before updating `burst_remaining`.
- Clamp defensively to the weapon burst range.
- Capture the attacker's source position/facing from the fire-tick snapshot already being used for the shot, not by looking up the attacker again after mutation.
- Preserve existing burst delay/cooldown behavior.

Expected burst behavior:

- `Burst=1`: every shot has burst index `0`.
- `Burst=2`: first shot has burst index `0`, second shot has burst index `1`.
- If state is malformed, clamp rather than panic in normal gameplay.

Verification:

- Add or update a focused combat test for `Burst=2` event indices.
- Run the focused combat test.

## Task 3: Parse Missing Building Fire Metadata

Inspect `src/rules/art_data.rs`, the INI files, and existing docs for building fire-origin keys.

Add parsed fields for:

- `PrimaryFirePixelOffset`
- `SecondaryFirePixelOffset`
- `PrimaryFireDualOffset`
- any already-verified building turret origin field that is present in the retail INI/art data and not currently represented

Requirements:

- YR `*md` override semantics must remain unchanged.
- Unknown or absent keys should be represented as `None` or an explicit default matching the parser's existing style.
- Do not invent hardcoded offsets for stock buildings.

Verification:

- Add parser tests using representative art snippets.
- Confirm stock `GAWEAP` data still parses.

## Task 4: Create App-Side Fire Origin Resolver

Add `src/app_fire_origin.rs` or a similarly small module if that matches current top-level module style.

Responsibilities:

- Accept `SimFireEvent`, attacker art/rules metadata, and optional garrison/building context.
- Return one `FireWorldOrigin`.
- Convert the resolved world coordinate to screen coordinates once.
- Expose explicit branch diagnostics for tests.

Branches to implement:

- normal techno FLH
- building fixed pixel offset
- building voxel turret origin if metadata exists
- garrison muzzle port
- explicit missing-metadata result

Projection/conversion requirements:

- Use the existing render/lepton projection helpers where possible.
- Implement the inverse isometric pixel-to-world conversion with integer or fixed-rational math, not floating point in deterministic state.
- Keep conversion app-side unless it is already a reusable low-level utility with no render dependency.

Verification:

- Unit-test the pixel-to-world conversion round trip with small known offsets.
- Unit-test that missing required metadata returns an explicit unresolved result instead of attacker-cell center.

## Task 5: Implement Normal Techno FLH Origin

Move normal non-garrison FLH origin resolution into the new resolver.

Requirements:

- Use the event snapshot facing.
- Apply burst lateral alternation using pre-shot burst index.
- Resolve a world/lepton source coordinate first, then derive screen coordinates from it.
- Preserve current muzzle flash art selection behavior.

Verification:

- Add tests for burst index `0` and `1` producing opposite lateral offsets for a dual/burst weapon.
- Add a fixture using an existing infantry/vehicle art entry with FLH data.

## Task 6: Implement Building Fixed Pixel Offsets

Add building branch handling for `PrimaryFirePixelOffset` and `SecondaryFirePixelOffset`.

Requirements:

- Apply the building render coordinate base, including the verified `-0x80` X/Y shift.
- Choose primary vs secondary offset from `weapon_slot`.
- Convert the pixel offset through the isometric pixel-to-world helper.
- Respect `PrimaryFireDualOffset` where it changes lateral side selection.
- Do not reuse normal unit FLH logic for this branch.

Verification:

- Add a parser/resolver test with `PrimaryFirePixelOffset=...` and `SecondaryFirePixelOffset=...`.
- Assert the resolved origin is not the building cell center.

## Task 7: Implement Garrison Port Origin

Route garrison muzzle port fire through the same origin resolver.

Requirements:

- Use the event's `garrison_muzzle_index`.
- Use parsed `MuzzleFlash0..9` positions from `ArtEntry::muzzle_flash_positions`.
- Convert the selected screen pixel pair to world origin using the same isometric pixel-to-world helper.
- Preserve `occupant_anim` behavior.
- Return explicit unresolved metadata if the requested port index is absent.

Verification:

- Add a resolver test for a known garrison art snippet with multiple muzzle ports.
- Ensure invalid port index does not silently use port 0 or cell center.

## Task 8: Wire Muzzle Flash And Report Sound To Shared Origin

Update `src/app_fire_effects.rs` to resolve origin once per fire event and use it for:

- non-garrison muzzle flash placement
- garrison muzzle flash placement
- report sound position

Requirements:

- Remove duplicated origin math from individual consumers where practical.
- Keep effect scheduling behavior unchanged.
- If origin is unresolved, degrade explicitly using existing logging/error style rather than pretending parity was achieved.

Verification:

- Existing fire-effect tests compile.
- Add a regression test or focused helper test proving sound and muzzle flash read the same resolved origin.

## Task 9: Wire Projectile Visuals To Shared Origin

Update projectile visual creation in `src/app_fire_effects.rs`.

Requirements:

- Use `FireWorldOrigin` as projectile start.
- Keep target destination behavior unchanged unless the target is already known to be wrong.
- Recompute projectile facing and duration from resolved source-to-target deltas.
- Avoid attacker-cell integer deltas for projectile animation when subcell origin exists.

Verification:

- Add a focused test for a source offset changing projectile start and direction.
- Confirm projectile visual construction still handles ballistic and non-ballistic projectiles.

## Task 10: Handle Building Voxel Turret Origin

Implement the verified building voxel turret branch if required metadata is available in the local rule/art model after Task 3.

Requirements:

- Use building turret origin metadata instead of fixed pixel offsets when that is the verified branch for the building/weapon.
- Use snapshot facing and building render base.
- If metadata remains unrepresented after inspection, leave a narrow `Unresolved(BuildingTurretMetadataMissing)` result and document the exact missing key/source in code comments and this plan's follow-up section.

Verification:

- Add a resolver test if a stock fixture exists.
- If no stock fixture exists, add a synthetic parser/resolver test only after confirming the key names from docs or INI.

## Task 11: Focused Verification Pass

Run the smallest useful test set first, then broaden.

Suggested commands:

```powershell
cargo test -p ra2-rust-game fire_origin
cargo test -p ra2-rust-game fire_effect
cargo test -p ra2-rust-game combat
cargo check
```

If package names or filters differ, use the closest existing focused targets and record the exact commands in the final implementation summary.

## Task 12: Manual Parity Checklist

After tests pass, run or inspect a local scenario that covers:

- vehicle or infantry weapon with FLH
- burst weapon where shot 1 and shot 2 use different lateral origins
- building primary weapon with fixed fire pixel offset
- garrisoned building firing from a port
- projectile art whose visible start point makes source errors obvious

Expected visible outcome:

- muzzle flash, report sound, projectile start, projectile facing, and projectile duration all originate from the same resolved source.
- no branch falls back to attacker cell center unless the legacy behavior for that exact branch is verified.

## Risk Areas

- The exact RA2/YR 32-way FLH transform must not be simplified into an 8-way approximation.
- Building pixel offsets are screen-space inputs, not world-space inputs.
- Burst index must be captured before `burst_remaining` is updated.
- Current entity state after the tick may already differ from the shot source.
- Missing metadata can hide parity failures if it falls back silently.

## Follow-Up Candidates

Only create follow-up work if the implementation exposes a verified missing prerequisite:

- additional building turret origin metadata not currently parsed from art/rules data
- exact rounding mismatch in isometric pixel-to-world conversion
- target endpoint mismatch for projectile visuals independent of source-origin parity
