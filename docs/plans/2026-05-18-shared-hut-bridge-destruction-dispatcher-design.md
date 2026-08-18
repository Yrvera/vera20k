# Shared Hut Bridge Destruction Dispatcher Design

## Goal

Implement a shared gamemd-style bridge-hut destruction dispatcher so C4 on `CABHUT` collapses the connected bridge on real map topologies, including ramp/bridgehead fallback, while preserving existing direct-overlay C4 behavior.

## Architecture Context

The C4 order path is already split cleanly:

1. App command code queues `Command::PlantC4`.
2. `world_commands.rs` validates the attacker and target, sets `c4_plant`, and starts movement.
3. `tick_c4_plants` claims the plant, sets `pending_c4_detonation`, emits `SimSoundEvent::C4Planted`, waits `rules.c4_delay_ticks`, and calls `apply_c4_damage_to_building`.
4. `apply_c4_damage_to_building` detects `BridgeRepairHut=yes` and calls `bridge_orchestrator::dispatch_bridge_collapse_from_hut`.
5. The app only sees `TickResult.bridge_state_changed` and rebuilds pathing/render data after sim mutation.

The bridge runtime already has most of the needed machinery:

- `BridgeRuntimeState::destroy_bridge_low/high` drive direct overlay bridge destruction.
- `BridgeRuntimeState::bridgehead_advance_state` and `body_cell_advance_state` drive state-machine damage used by non-overlay bridgehead/ramp/anchor cases.
- `bridge_orchestrator.rs` already aggregates `StateOutcome::Collapsed` into the player-visible side effects: ground occupant kill, deck drop-in, debris, adjacent bridge updates, trigger notification, and zone refresh.

The current CABHUT dispatcher is close but still not gamemd-shaped enough. It scans for a matching destroy overlay and its fallback still returns only a destroy-overlay entry. If the hut-local topology exposes bridgehead/ramp/flag evidence but no direct body overlay in that search path, Rust can plant C4 and clear the marker while producing no visible bridge collapse.

This design remains entirely under `sim/`. It must not add dependencies from `sim/` to app, render, UI, sidebar, audio, or net.

## Impact Analysis

Touched modules:

- `src/sim/world/bridge_orchestrator.rs`
  - Owns the shared hut destruction dispatcher.
  - Needs a no-RNG, gamemd-style "apply damage to this bridge cell" helper for map-init/hut destruction fallback.
  - Should reuse existing cascade aggregation rather than duplicate collapse side effects.

- `src/sim/world/world_orders.rs`
  - Keeps CABHUT C4 timer and pending-marker cleanup.
  - Continues to call the same hut dispatcher signature or a thin wrapper around the new shared helper.

- `src/sim/bridge_state/mod.rs` and `src/sim/bridge_state/walker.rs`
  - Provide existing direct/state-machine mutation APIs.
  - May need small crate-visible helpers only if current visibility blocks clean orchestration.

- `src/sim/world/world_orders_bridge_repair_tests.rs`
  - Needs tests for direct overlay, bridgehead/ramp fallback, marker cleanup, and existing CABHUT C4 happy path.

Risk areas:

- Fallback direction and search bounds can collapse the wrong bridge if made too broad.
- Using `apply_bridge_damage_events` directly would be wrong because normal combat bridge damage applies `BridgeStrength` RNG gates; gamemd hut destruction fallback calls `ApplyDamageToCell` without that combat RNG gate.
- Terminal destroyed overlays (`0x64`, `0x65`, `0xE7`, `0xE8`) are part of the map-init overlay scan range but direct `ApplyDamageToCell` treats terminal high overlays differently. Tests must pin the intended behavior.
- C4 marker cleanup must not regress non-hut Iron Curtain retry behavior.

## Chosen Approach

Build a shared hut-destruction dispatcher inside `bridge_orchestrator.rs`.

Keep this public crate-local entry stable:

```rust
pub(crate) fn dispatch_bridge_collapse_from_hut(
    sim: &mut Simulation,
    rules: &RuleSet,
    hut_center: (u16, u16),
) -> bool
```

Internally, route it through a clearer helper such as:

```rust
fn dispatch_hut_bridge_destruction(
    sim: &mut Simulation,
    rules: &RuleSet,
    hut_center: (u16, u16),
) -> bool
```

The helper should mirror gamemd output behavior:

1. Build the hut-centered 5x5 scan with the existing `cells_in_5x5_scan`.
2. Choose low/high:
   - low if the 5x5 scan contains low bridge tile evidence or low overlay `0x4A..=0x65`;
   - high otherwise.
3. Overlay-first path:
   - low dispatcher scans for first overlay `0x4A..=0x65`;
   - high dispatcher scans for first overlay `0xCD..=0xE8`;
   - the first match runs the existing direct walker sweep and returns.
4. Fallback path:
   - if no overlay is found, find bridge/ramp/bridgehead evidence from the hut cell and nearby cells using existing runtime terrain facts;
   - derive a direction-connected fallback path instead of doing arbitrary nearest-overlay search;
   - treat bridgehead/ramp cells as evidence and possible absorbed state writes, but keep walking to body/anchor/direct cells because the current `bridgehead_advance_state` helper never collapses a span;
   - call a local no-RNG `ApplyDamageToCell` equivalent on up to the gamemd-shaped attempt cells.
5. Aggregate any non-`NoChange` outcomes through the existing bridge collapse cascade.
6. Return `true` only when bridge state visibly changed.

The design intentionally avoids persisting exact `CellClass+0x140` flag bits. The Rust terrain/runtime model already stores bridge deck, transition, ramp, anchor, bridgehead, bridge layer, and bridge facts. The fallback should use those as local evidence and keep the approximation isolated inside the hut dispatcher.

## Tiny-Detail Ledger

- `SealPlaceBomb` means the plant was claimed and `pending_c4_detonation` exists. Source: current `tick_c4_plants`, confirmed by user observation.
- CABHUT C4 effect happens after stock `C4Delay=.03`, parsed to 27 ticks. Source: `ini/rulesmd.ini` and `ruleset.rs`.
- CABHUT C4 does not damage the hut. Source: `BuildingClass::Update @ 0x0043FB20`, `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` section 18A.3.
- CABHUT clears the C4 marker and attacker pointer after the hut bridge dispatcher returns. Source: `BuildingClass::Update @ 0x0043FB20`.
- Low/high choice comes from a hut-centered 5x5 scan. Low wins if low tile evidence or low overlay `0x4A..=0x65` is present; otherwise high. Source: `BuildingClass::Update @ 0x0043FB20`.
- `DestroyBridge_High_MapInit @ 0x00574000` overlay-first high scan accepts `0xCD..=0xE8`. Source: live Ghidra.
- `DestroyBridge_Low_MapInit @ 0x00574C20` overlay-first low scan accepts `0x4A..=0x65`. Source: report and compiled twin of high function.
- If overlay-first scan finds no match, gamemd searches bridge/ramp flags and can call `ApplyDamageToCell` on fallback cells. Source: `DestroyBridge_High_MapInit @ 0x00574000`.
- `ApplyDamageToCell @ 0x00587180` direct low range is `0x4A..=0x63`; direct high range is `0xCD..=0xE6`; otherwise it can dispatch to low/high bridge state-machine paths. Source: live Ghidra.
- Hut fallback bridge damage must not use combat `BridgeStrength` RNG. Source: `ApplyDamageToCell @ 0x00587180` path has no combat warhead RNG gate.
- Rust `BridgeRuntimeState::bridgehead_advance_state` is not a collapse path; it returns `Absorbed` or `NoChange` and existing tests assert sustained bridgehead direct damage does not collapse a bridge. Hut fallback must not stop after bridgehead absorption.
- Collapse side effects must flow through the existing Rust cascade: ground occupant kill, deck drop-in, debris, adjacent bridge update, trigger hook, zone refresh. Source: existing `bridge_orchestrator.rs` pattern.
- `TickResult.bridge_state_changed` must become true when the bridge visibly changes so the app rebuilds `PathGrid` and render-facing state. Source: `app_sim_tick.rs`.

## Design

### Components

`dispatch_hut_bridge_destruction`

- Shared internal entry for CABHUT C4 now and later demo-truck/hut destruction callers.
- Owns 5x5 scan, low/high choice, overlay-first path, fallback path, and cascade handoff.

`HutBridgeFamily`

- Existing local enum is sufficient: `Low` or `High`.
- Represents the selected `DestroyBridge_*_MapInit` family, not necessarily every `ApplyDamageToCell` sub-dispatch.

`HutDamageEntry`

- New local enum inside `bridge_orchestrator.rs`:

```rust
enum HutDamageEntry {
    DirectOverlay { rx: u16, ry: u16, family: HutBridgeFamily },
    FallbackCell { rx: u16, ry: u16 },
}
```

- Direct overlay entries call the family-specific direct walker sweep.
- Fallback cells call the no-RNG `ApplyDamageToCell` equivalent.

`apply_hut_damage_to_cell`

- Local helper that mirrors gamemd `ApplyDamageToCell` for hut/map-init callers without combat RNG.
- Dispatch order:
  1. direct low if overlay is `0x4A..=0x63`;
  2. direct high if overlay is `0xCD..=0xE6`;
  3. low state-machine if runtime/terrain evidence identifies a low bridge state-machine cell;
  4. high state-machine if runtime/terrain evidence identifies a high bridge state-machine cell;
  5. otherwise `NoChange`.
- A bridgehead result of `Absorbed` is not a completed hut collapse. The dispatcher should retain that outcome for any side effects but continue along the same fallback trace until a body/anchor/direct cell collapses or the bounded trace ends.

`find_hut_fallback_cells`

- Finds a short, direction-connected list of fallback cells from hut/ramp/bridgehead evidence.
- Must not return an arbitrary nearest overlay from a square radius.
- The output order should be deterministic and follow hut cell first, then fixed directions.
- Required regression shape: CABHUT center `(9, 10)`, no direct destroy overlay in `x=7..=11`, `y=8..=12`, first eastward evidence at bridgehead/ramp cell `(12, 10)`, first collapsible body/anchor state-machine cell at `(13, 10)`.

### Interfaces / Contracts

No command, rules, save schema, app, render, or audio interface changes.

`dispatch_bridge_collapse_from_hut` remains callable from `world_orders.rs` with the same signature. If a future demo-truck path needs the same behavior, it should call the same shared dispatcher rather than duplicating CABHUT logic.

Any helper visibility changes in `bridge_state` should be `pub(crate)` only.

### Data Flow

1. `tick_c4_plants` sees a pending C4 on CABHUT after `C4Delay`.
2. `apply_c4_damage_to_building` branches before normal building damage and calls the hut dispatcher.
3. The hut dispatcher mutates `BridgeRuntimeState` through direct walkers or state-machine helpers.
4. The dispatcher converts `StateOutcome`s into existing cascade side effects.
5. `apply_c4_damage_to_building` returns `killed_building=false`, `bridge_state_changed=<actual>`, `consumed_pending_marker=true`.
6. `tick_c4_plants` clears the CABHUT pending marker and attacker `c4_plant`.
7. The app observes `bridge_state_changed` and refreshes pathing/render state.

### Error Handling

This is deterministic sim logic; failure cases return `false` rather than logging or panicking.

- Missing terrain or bridge state: return `false`.
- No bridge/ramp evidence: return `false`.
- Evidence found but no state-machine/direct path can mutate: return `false`.
- Out-of-bounds search cells: skip.

C4 marker cleanup remains outside the dispatcher and still occurs for CABHUT even when the dispatcher returns `false`, matching gamemd marker cleanup after dispatch.

### Testing Strategy

Required tests:

- Direct high overlay in the hut 5x5 collapses and clears pending.
- Direct low overlay in the hut 5x5 collapses and clears pending.
- Bridgehead/ramp fallback near CABHUT mutates through state-machine path without a direct overlay entry and does not stop at bridgehead absorption; it must continue to the connected body/anchor cell that collapses.
- Terminal overlay-first scan accepts low `0x65` and high `0xE8`, while fallback `ApplyDamageToCell` direct dispatch does not treat terminal overlays as direct damage entries.
- A nearby unrelated bridge outside the direction-connected fallback path does not collapse.
- CABHUT with no bridge evidence clears pending and leaves hut alive.
- Existing non-hut normal C4 happy path still kills buildings.
- Existing non-hut Iron Curtain C4 retry keeps pending until invulnerability ends.

Verification commands:

```powershell
cargo test c4 --lib -- --nocapture
cargo test c4_on_cabhut --lib -- --nocapture
```

## Architectural Decisions

- Keep bridge-hut destruction in `bridge_orchestrator.rs`, where bridge cascade side effects already live.
- Reuse existing `BridgeRuntimeState` mutators rather than adding a second bridge damage model.
- Do not call `apply_bridge_damage_events` from hut fallback because it models combat/warhead damage and includes RNG gates that hut destruction does not use.
- Keep exact gamemd cell flags as a future refinement; use isolated runtime/terrain evidence now.

## Alternatives Considered

### C4-Only Patch

Rejected. It would make CABHUT C4 work in one path but duplicate the same binary dispatcher that demo-truck and other hut destruction callers need.

### Overlay-Only Fix

Rejected. It preserves synthetic tests but leaves the reported real-map topology broken when only ramp/bridgehead evidence is visible to Rust.

### Full Persistent Cell Flag Clone

Rejected for this change. It may become necessary later, but adding exact `CellClass+0x140` storage would broaden terrain/pathing contracts and increase regression risk. The fallback approximation stays isolated so it can be replaced if review demands exact flags.
