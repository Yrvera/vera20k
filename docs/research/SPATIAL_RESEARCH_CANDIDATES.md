# Spatial Systems — Research Candidates (Roadmap)

**Date:** 2026-05-07
**Type:** Roadmap / gap inventory. Not a findings doc — these systems have NOT yet been
investigated. Each entry says what to study, why it matters, what's already known, and a
suggested Ghidra entry point.

Companion to [SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md](SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md),
which catalogs what's already known about the spatial primitives layer. This doc lists
the **next-most-foundational consumers** of that layer that don't yet have deep coverage.

**13 candidates** organized into two tiers:

- **Tier 1 — Foundational primitives (8 systems)**: highest cross-cutting impact;
  every-tick or every-fire-event consumers. Drift here is felt across all gameplay.
- **Tier 2 — Specialized but spatial-foundational (5 systems)**: narrower consumer
  surface, but still primitives (geometry, prediction, render-feeding math). Drift here
  is local to a system (e.g., voxel render, bridge correctness) but visible.

---

## How priorities are ranked

Severity = downstream impact × trigger frequency × player visibility:

- **Downstream impact**: how many other systems depend on this one. A bug in facing math
  bleeds into every weapon, turret, projectile, and unit visual; a bug in find-nearest
  bleeds into harvest cycles and garrison entries only.
- **Trigger frequency**: how often the system fires in a normal match. Per tick, per
  fire event, per movement, per minute, per match.
- **Player visibility**: whether drift is felt (turret aim wrong on every shot) or only
  noticed by an attentive player (splash damage 0.5 cell off).

Per the project's parity bar, "felt" matters more than "loud" — small drifts compound.

---

# Tier 1 — Foundational primitives

These eight primitives are read by essentially every system that does anything spatial.
A bug in any of them propagates outward.

---

## 1. Facing & rotation math (HIGHEST priority)

**What it does.** Convert a 2D lepton delta (or a target position) to a facing byte; step
a unit's current facing toward a desired facing at a per-type rotation rate; clamp 8-bit
↔ 16-bit facing conversions consistently across body, turret, and projectile-aim paths.

**Why foundational.** Every entity that can rotate — units, infantry, turrets, voxel
chassis, projectile heads — reads the same primitives. A 1-step (≈1.4°) drift in the
facing-from-delta function or a wrong rotation-rate clamp is invisible per-frame but
shows up everywhere over time as "turret leads off-axis" or "infantry walks slightly
diagonal."

**What's already known.**
- 8-direction cell offsets at `0x89F688` (runtime-init at `0x49F2F0`).
- 8-direction lepton offsets at `0x89F6D8` / `0x89F6DC`.
- Drive track tables encode 72-step facing transitions (`TURN_TRACKS`, `DAT_007E7B30`
  flags) — see [LOCOMOTION_MATH_AND_CONSTANTS.md §4](LOCOMOTION_MATH_AND_CONSTANTS.md).
- `atan2` helper at `0x4CAE30` — generic, not Fly-specific.
- Facing convention: 8-bit byte, 0=N, increasing clockwise.

**What's missing.**
- The canonical "facing-from-delta" helper (almost certainly wraps `atan2` and bins
  to 8-bit). Used wherever a unit needs to face a target.
- Turret rotation step / clamp logic — TechnoTypeClass has a `ROT=` field, but the
  actual per-tick step function isn't decompiled.
- 16-bit facing usage (`facing << 8`) — which paths use it, where the conversion lives.
- Voxel chassis facing → tilt matrix interaction (relevant to render, but the input
  comes from this layer).

**Where to start.**
- Search functions named `Facing` (gamemd has a `FacingClass` for current/desired/ROT).
- Find xrefs to `atan2 @ 0x4CAE30` to enumerate consumers.
- Decompile `UnitClass::AI` and find where body / turret facing get updated.
- TechnoTypeClass `ROT` field offset (search ReadINI for `"ROT"` string).

**Suggested entry point.** `/plan-investigation facing and rotation primitives`.

**Priority justification.** Triggers every tick on every rotating entity. Visibility:
direct (turret aim lead, infantry walk angle, projectile launch direction). Impact: every
weapon / vehicle / projectile system reads these. Highest cross-cutting cost-to-fix-late.

---

## 2. AOE / CellSpread cell enumeration (HIGH priority)

**What it does.** Given an explosion at world position P with `Warhead.CellSpread = R`,
enumerate which cells take damage and how much each gets (with `PercentAtMax`
falloff). Determines splash radius, ground-deformation radius, and wall-collateral.

**Why foundational.** Every explosion runs this. The shape (diamond? circle? cell-square?),
the inclusion rule at the boundary (≤R or <R), and the per-cell falloff curve all affect
gameplay balance and feel. Drift here makes Tesla coils, V3s, and nukes deal subtly
wrong damage — invisible per-shot, decisive over a match.

**What's already known.**
- `WarheadTypeClass.CellSpread` at `+0x124` (float, cells), `PercentAtMax` at `+0x12C`
  (float, default 1.0). See [WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md](WARHEADTYPECLASS_FULL_STRUCT_LAYOUT.md).
- `BulletClass::Detonate / WarheadTypeClass::Detonate` at `0x4690B0` — entry point per
  [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md).
- AOE damage is documented partially in [DAMAGE_MATH_GHIDRA_REPORT.md](DAMAGE_MATH_GHIDRA_REPORT.md)
  (per-cell damage formula), but the cell-enumeration *pattern* is not.

**What's missing.**
- The actual cell-walk: does it iterate a square bbox and filter by distance, or a
  pre-computed cell template? What metric (Euclidean leptons? Chebyshev cells?
  squared-leptons?) is used for the radius test?
- Boundary inclusivity at R = integer cells.
- Falloff curve: linear from 1.0 at center to `PercentAtMax` at R, or some other shape?
- Whether walls / buildings block splash propagation between cells (LOS-style) or
  every cell-in-radius takes damage regardless.

**Where to start.**
- Decompile `0x4690B0` (Warhead Detonate).
- Find xrefs to `WarheadTypeClass+0x124` (CellSpread) — enumerate consumers.
- Look for cell-iteration loops near the damage application.

**Suggested entry point.** `/plan-investigation warhead cell-spread enumeration`.

**Priority justification.** Triggers every explosion (every weapon impact, every death).
Visibility: medium (a careful player notices splash hitting/missing edge units). Impact:
all combat balance.

---

## 3. LOS / sight reveal cell walk (HIGH priority)

**What it does.** When a unit moves into a new cell or is created, walk outward from its
position to its `Sight=` radius and reveal cells from the shroud. Same algorithm gates
"can A see B" for cloak-detection, target-acquisition, and bandbox.

**Why foundational.** Every move, every scan, every reveal goes through this. A wrong
visit pattern means units reveal a different shape than gamemd, leading to "I can see them
but they can't see me" parity bugs. Highly visible feel-bug.

**What's already known.**
- [SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md](SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md) and
  [SHROUD_ALGORITHM_DISTILLED.md](SHROUD_ALGORITHM_DISTILLED.md) cover the shroud bitmap
  and high-level reveal flow.
- [SHROUD_DISPARITIES.md](SHROUD_DISPARITIES.md) lists known Rust↔gamemd differences.
- Fog of war is TS-only and disabled in YR per CLAUDE.md.

**What's missing.**
- The *exact cell-visit pattern* for sight reveal: spiral? bbox-filter-by-Euclidean?
  pre-computed mask per integer radius?
- Height-occlusion rules: does a tall cell block sight to cells "behind" it from the
  observer, or is sight purely radial?
- Whether buildings / terrain types interrupt the reveal walk.
- The bandbox/cloak-sense reuse: same algorithm or different?

**Where to start.**
- Re-read the existing shroud docs and identify which functions they reference.
- Likely entry: `DisplayClass::Look_At` or similar — search functions with "Look" or
  "Reveal".
- Find xrefs to `TechnoTypeClass.Sight` field offset.

**Suggested entry point.** `/verify-doc SHROUD_REVEAL_SYSTEM_GHIDRA_REPORT.md` first
(verify what we already documented), then `/plan-investigation` for the cell-walk specifics.

**Priority justification.** Triggers on every unit move (per cell crossing) and per unit
creation. Visibility: high (player sees / doesn't see enemies based on this). Impact: all
of vision, cloak-detect, auto-target.

---

## 4. Bullet trajectory frame-step (MEDIUM-HIGH priority)

**What it does.** Per-tick position update for in-flight projectiles: integrate velocity,
apply gravity (for `Arcing`), apply turn rate (for `ROT > 0` homing), handle altitude
phases (V3 launch / cruise / terminal), check for impact.

**Why foundational.** All ranged weapons that aren't `Inviso=yes` raycast use this. The
exact integration determines where the warhead detonates, which feeds cell-spread (§2),
which determines who takes damage. Wrong trajectory ⇒ wrong impact cell ⇒ wrong damage.

**What's already known.**
- `BulletClass+0xAC` points to BulletTypeClass; `BulletClass::AI` is the per-tick update.
- [BULLETCLASS_TRAJECTORY_AND_HOMING.md](BULLETCLASS_TRAJECTORY_AND_HOMING.md) and
  [BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md](BULLET_PROJECTILE_SYSTEM_CONSOLIDATED_REPORT.md)
  partially cover trajectory.
- V3 / DMisl / CMisl per-rocket constants (12 INI keys each) documented in
  [LOCOMOTION_MATH_AND_CONSTANTS.md §11](LOCOMOTION_MATH_AND_CONSTANTS.md). 36 keys, all
  unparsed in Rust currently.
- `Rules.Gravity` at `Rules+0x16B8` — global gravity scalar (verified 2026-05-07).
- Bullet-type flags (`Arcing`, `ROT`, `Acceleration`, `CourseLockDuration`,
  `DetonationAltitude`, `Vertical`) in [BULLETTYPECLASS_GHIDRA_REPORT.md](BULLETTYPECLASS_GHIDRA_REPORT.md).

**What's missing.**
- The actual integration math per-frame: how velocity is updated (`Acceleration`? `ROT`
  toward homing target?), how gravity applies to Z each frame, how the multi-phase
  V3/DMisl/CMisl pause→tilt→cruise→terminal state machine reads its ~12 INI keys.
- Impact detection: bbox? radius? cell-entry? altitude threshold?
- How `Inaccurate=yes` perturbs the impact position.

**Where to start.**
- Decompile `BulletClass::AI` (find via search for class methods).
- Verify [BULLETCLASS_TRAJECTORY_AND_HOMING.md](BULLETCLASS_TRAJECTORY_AND_HOMING.md)
  against the binary first (`/verify-doc`) to know which existing claims hold.
- Decompile `RocketLocomotionClass::Process` (the 4-phase rocket FSM) at the constructor
  `0x661EC0`'s downstream Process.

**Suggested entry point.** `/verify-doc BULLETCLASS_TRAJECTORY_AND_HOMING.md` →
`/plan-investigation arcing and homing per-frame integration` for whatever's missing.

**Priority justification.** Triggers per-tick on every in-flight projectile. Visibility:
medium-high (V3 misses are loud; tank-howitzer arc looks wrong; missile lead changes who
gets hit). Impact: combat correctness for all non-instant weapons.

---

## 5. Find-nearest cell-spiral variants (MEDIUM priority)

**What it does.** Given an origin cell and a predicate (passable? unoccupied? ore-bearing?
dock-pad?), walk the surrounding cells in a deterministic spiral to find the first match.
Used by harvester dock-find, infantry bump-out, scatter, building placement preview,
garrison entry, eject-on-destruction.

**Why foundational.** Many systems read this; if the spiral order differs, "deterministic
random" outcomes differ — e.g., which corner an ejected garrison occupant lands on.
Network-determinism risk if the order isn't identical to gamemd.

**What's already known.**
- [FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md](FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md)
  covers one variant.
- `FootClass::Find_Nearest_Dock @ 0x4DFCB0` — harvester variant.
- Various scatter / placement helpers exist throughout the binary.

**What's missing.**
- Whether all variants share a single underlying spiral generator, or each has its own
  pattern.
- Exact step order (NSEW first? NW NE SE SW? clockwise? expanding rings?).
- Termination conditions (max-radius? predicate-found? failure sentinel cell).
- Whether bridge-layer is searched separately or merged.

**Where to start.**
- Cross-reference `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` and `Find_Nearest_Dock`
  to see if they call a common helper.
- Find xrefs to the failure-coords sentinel `DAT_0089E778` (per
  [INFANTRY_SUBCELL_POSITIONING.md](INFANTRY_SUBCELL_POSITIONING.md)) — its callers all
  use a "no-cell-found" pattern.

**Suggested entry point.** `/plan-investigation cell-spiral search variants` with the
goal of producing a unified table of all variants and their ordering.

**Priority justification.** Triggers on harvester returns (every ~30s in dense play),
infantry ejection (per garrison destroy), scatter (combat). Visibility: low-medium per
event but cumulative drift across a match. Impact: harvest cycle timing, garrison reload
positions, scatter pattern aesthetics.

---

## 6. Cell passability primitive (HIGH priority)

**What it does.** Given an entity (with locomotor, MovementZone, size) and a cell
coordinate, return whether the entity can occupy or pass through that cell. Considers
terrain type per SpeedType, occupancy state (other units, buildings, walls), height/cliff
thresholds, bridge layer membership, water vs. land, and special states (cloak,
in-tunnel).

**Why foundational.** Every path step queries this. Every unit placement, garrison entry,
crate pickup, IFV mount, building scaffold, harvester ore-cell pick goes through it. If
the truth-table differs even on edge cases (bridge cell with infantry on top? amphibious
on shore tile?), pathfinding produces different routes than gamemd, units phase through
walls, or get stuck where they shouldn't.

**What's already known.**
- [UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md](UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md) — primary
  doc, partial coverage.
- [ZONE_PASSABILITY_VERIFIED.md](ZONE_PASSABILITY_VERIFIED.md) — MovementZone semantics.
- [TERRAIN_COST_FACTSHEET.md](TERRAIN_COST_FACTSHEET.md) — per-SpeedType terrain costs.
- [LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md](LIMBO_AND_CELL_OCCUPATION_LIFECYCLE_GHIDRA_REPORT.md)
  — when units are added/removed from cell occupant lists.
- [NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md](NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md) —
  naval-specific subset.

**What's missing.**
- A unified table: for each (SpeedType × cell-state combination), is it passable? Today
  scattered across SpeedType / Zone / passability docs.
- "Can X stop here" vs. "can X pass through" distinction (terminal vs. transient cells
  in pathfinding).
- How `Crushable=` / `OmniCrusher=` interact with passability.
- Subcell semantics: infantry pass-through-vehicle behavior.
- Tunnel cell legality (CanGetIn / CanGetOut for tunnel entrances).

**Where to start.**
- `/verify-doc UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` first.
- Decompile the central `MapClass::CanFitOn` / `UnitClass::Can_Enter_Cell` family.
- Cross-reference all callers to enumerate the full check matrix.

**Suggested entry point.** `/verify-doc UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md` then
`/plan-investigation cell passability complete decode`.

**Priority justification.** Triggers on every path step (per movement, per tick), every
placement, every spawn. Visibility: high (units choose different routes than gamemd, miss
shortcuts, can't enter cells they should). Impact: pathfinding, placement, scatter, AI
tactics. Tier 1.

---

## 7. Cell-radius enumeration (HIGH priority)

**What it does.** Given an origin cell C and a radius R (cells or leptons), return the
*set* of all cells satisfying a predicate within R. Different from find-nearest (single
result vs. all). Used by ore-scan (harvester finds nearby ore), threat-scan (target
priority), AOE (cell-spread, §2), sensor reveal, garrison capture range, MCV deploy area.

**Why foundational.** Different consumers visit cells in different orders and apply
different predicates, but they share the underlying "iterate cells within R" primitive.
If iteration shape, order, or inclusion rule differs from gamemd, the "first-found"
outcome differs (e.g., harvester picks different ore patch) even when both engines find
the same set eventually.

**What's already known.**
- [TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md](TIBERIUM_QUEUE_SEEDING_AND_TIMING_REPORT.md)
  — ore-cell scan radius (`TiberiumShortScan`, `TiberiumLongScan`).
- [GREATEST_THREAT_SCAN_GHIDRA_REPORT.md](GREATEST_THREAT_SCAN_GHIDRA_REPORT.md) — target
  scan, partial coverage.
- [TARGET_ACQUISITION_GHIDRA_REPORT.md](TARGET_ACQUISITION_GHIDRA_REPORT.md) —
  auto-target scan.
- [SENSOR_CLOAK_DETECTION.md](SENSOR_CLOAK_DETECTION.md) — sensor radius.
- AOE cell walks (covered as §2 separately).

**What's missing.**
- Whether all radius-scans share a common helper or each consumer rolls its own.
- Iteration shape: bbox-and-filter? expanding ring? pre-computed offset table? May
  differ between fast scans (cheap) and thorough scans.
- Ordering: spiral? row-major? by-distance?
- Whether bridge layer is included or scanned separately.
- Scan-radius unit conversion: `TiberiumShortScan=4` (cells) → iteration bound — 4 =
  inclusive Chebyshev? Euclidean cells?

**Where to start.**
- Cross-reference xrefs from `TiberiumShortScan` consumers and `GREATEST_THREAT` scan
  loops — shared helper or independent?
- Search for cell-iteration loops with `dx*dx + dy*dy <= R*R` patterns.

**Suggested entry point.** `/plan-investigation cell-radius scan unification`.

**Priority justification.** Triggers many times per tick (per harvester scan, per
scanning unit, per AOE explosion, per sensor refresh). Visibility: medium (different
first-found target / ore cell, different "auto-target" feel). Impact: harvester
behavior, AI targeting, AOE damage distribution. Tier 1.

---

## 8. Speed → traversal time (HIGH priority)

**What it does.** Given INI `Speed=` (0-100), terrain cost multiplier, slope multiplier,
and per-unit veterancy bonus, compute the per-tick lepton advance. From that, derive
cell-crossing duration in frames. Determines how long every movement actually takes.

**Why foundational.** Every unit's move-from-A-to-B time is built from this conversion.
If Speed-to-leptons is even a few percent off, "rush distance" feel changes, harvest
cycles desync, build orders behave differently. Multiplayer determinism collapses if the
math diverges.

**What's already known.**
- [LOCOMOTION_MATH_AND_CONSTANTS.md §3](LOCOMOTION_MATH_AND_CONSTANTS.md) — speed ramping,
  acceleration, deceleration.
- [TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md](TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md) — Speed
  field offset.
- [TERRAIN_COST_FACTSHEET.md](TERRAIN_COST_FACTSHEET.md) — terrain cost values.
- [DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md](DRIVE_LOCOMOTION_HELPERS_GHIDRA_REPORT.md)
  — partial drive-side coverage.
- Rust uses `SimFixed` per [object_type.rs](../../../ra2-rust-game/src/rules/object_type.rs)
  — already substantial.

**What's missing.**
- The *exact* `INI Speed [0-100] → leptons-per-tick` conversion. The widely-believed
  "Speed × ~5/256 leptons-per-frame" approximation may not be the real formula.
- How veterancy speed bonus stacks with terrain multiplier (multiply or sequential
  clamp?).
- Per-locomotor differences (does Fly use the same conversion as Drive? Hover? Walk?).
- The `ReadSpeed` parser semantics — is `Speed=0` legal (special meaning)? Is 100 the cap?

**Where to start.**
- Decompile `ReadSpeed` (CCINIClass helper) — gives the parse formula.
- Trace `TechnoTypeClass.Speed` field consumers — find the per-tick advance computation
  in each locomotor's `Process`.

**Suggested entry point.** `/plan-investigation speed-to-traversal conversion math`.

**Priority justification.** Triggers per-tick on every moving entity. Visibility: high
when wrong (units arrive at wrong frames; harvester cycle off; rush timings shifted).
Impact: all movement, multiplayer determinism, AI build-order timing. Tier 1.

---

# Tier 2 — Specialized but spatial-foundational

These five primitives have narrower consumer surface than Tier 1 but still qualify as
primitives — math/geometry layers other systems read. Drift here is local to a system
(voxel render, bridge correctness) but still visible.

---

## 9. Voxel chassis tilt on slopes (Tier 2)

**What it does.** Given a vehicle on a sloped cell, compute the chassis tilt matrix from
the cell's slope orientation (NE-up, S-down, etc.) so the voxel renders pitched/banked
along the slope. Independent of body facing; combines with facing matrix to produce final
render matrix.

**Why foundational.** All voxel vehicles tilt visibly. Wrong tilt → vehicles look "wrong"
on slopes (floating, sunk, banking the wrong way). Highly visible visual primitive even
though the logic feeds only render.

**What's already known.**
- [VOXEL_SLOPE_TILT_SYSTEM.md](VOXEL_SLOPE_TILT_SYSTEM.md) — slope-to-tilt mapping.
- [VXL_DRAW_MATRIX_GHIDRA_REPORT.md](VXL_DRAW_MATRIX_GHIDRA_REPORT.md) — matrix builders.

**What's missing.**
- Whether existing docs are HIGH or MEDIUM confidence (verify first).
- How tilt-while-moving-onto-slope is interpolated (snap on cell-cross, smooth over N
  frames?).
- Per-cell slope index → tilt matrix conversion table — 16-entry LUT or computed?
- Interaction with bridge cells (do tanks on a bridge tilt with the bridge? gamemd
  bridges are typically flat decks).

**Where to start.**
- `/verify-doc VOXEL_SLOPE_TILT_SYSTEM.md` first.
- Decompile the chassis matrix builder near `0x55A730`
  (`Matrix3x4_BuildFromRotateXAndFacing`).

**Suggested entry point.** `/verify-doc VOXEL_SLOPE_TILT_SYSTEM.md` then targeted
`/re-investigate` if gaps remain.

**Priority justification.** Triggers per-frame on every visible voxel unit on sloped
terrain (very common). Visibility: high (tilt direction is obvious to the eye). Impact:
render-only — gameplay isn't affected. Tier 2.

---

## 10. Aim leading / target prediction (Tier 2)

**What it does.** When a unit fires at a moving target, does the engine predict where the
target will be when the projectile arrives, or does it shoot at the target's current
position? If predicting: how is the lead computed?

**Why foundational.** Determines whether fast-moving targets (Harriers, chrono-warping
units) take projectiles in the side or escape. Most RA2/YR ranged weapons clearly DON'T
lead (you can dodge by moving), but some homing weapons or AA may. Worth confirming for
parity.

**What's already known.**
- [TARGET_ACQUISITION_GHIDRA_REPORT.md](TARGET_ACQUISITION_GHIDRA_REPORT.md) — target
  selection, not lead.
- [BULLETCLASS_TRAJECTORY_AND_HOMING.md](BULLETCLASS_TRAJECTORY_AND_HOMING.md) — homing
  bullets correct mid-flight (not lead at fire-time).
- [FIRE_AT_ANALYSIS.md](FIRE_AT_ANALYSIS.md) — fire pipeline, doesn't address lead.

**What's missing.**
- Confirm direct-fire weapons fire at *current* target position (the assumption — but
  unverified).
- Whether AA weapons against high-flying aircraft compute any lead.
- Whether the homing seeker's "where am I going?" is purely pursuit (current position)
  or predictive at launch.

**Where to start.**
- Decompile `BulletClass::Init` / `BulletClass::Fire` — see what target position is
  captured at launch.
- Check if any code adds `velocity × time-to-impact` to `target.position` anywhere.

**Suggested entry point.** `/plan-investigation aim leading and target prediction`.

**Priority justification.** Triggers on every shot. Visibility: low-medium (only against
moving targets; only matters if engines disagree). Impact: AA effectiveness, anti-Kirov
balance. Tier 2 — likely confirms "no lead" quickly, then closes.

---

## 11. Bridge geometry primitives (Tier 2)

**What it does.** Define exactly where on a bridge cell the deck Z is, where a ramp Z
gradient transitions to flat-deck, where the span anchors (NS bridges vs EW, low vs high)
align in lepton space. Gives every consumer (movement, render, fire-LOS gate, AI pathing)
the same geometric ground truth.

**Why foundational.** Bridges are a constant pain point — Bridges Tier 2 work in this
project is dedicated to it. The underlying "where is the bridge surface?" math, separated
from state machine and rendering, would let consumers reason consistently. Right now the
geometry is spread across multiple docs.

**What's already known.**
- [BRIDGE_RENDERING_GHIDRA_REPORT.md](BRIDGE_RENDERING_GHIDRA_REPORT.md) — render-side
  geometry.
- [BRIDGE_SYSTEM.md](BRIDGE_SYSTEM.md) — overall behavior.
- [HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md](HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md)
  — damage transitions.
- [SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md](SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md)
  cites `DAT_00B0EB24` (BridgeHeightInLeptons), bridge cell flag `0x100`, layered
  occupancy.
- Bridges Tier 2 work-in-progress per the project tracker — multiple recent commits.

**What's missing.**
- A unified geometry primer separated from the state machine: "given cell C with bridge
  flag, the deck surface Z is X, the ramp gradient at this cell is Y, the anchor for span
  S is Z."
- AI pathfinding's view of bridges (reachable-via-ramp vs blocked).
- Consumer-side reconciliation: render geometry, LOS gate, occupancy, and pathing should
  all agree.

**Where to start.**
- **Coordinate with active Bridges Tier 2 work first** — possibly redundant if their
  plan already covers geometry primitives. Check the latest bridge plan in `docs/plans/`.

**Suggested entry point.** Likely **NOT a separate plan** — fold into Bridges Tier 2's
existing roadmap if not already in scope. If not: `/plan-investigation bridge geometry
primitives`.

**Priority justification.** Triggers per-tick on every entity over a bridge, per LOS
check, per render frame. Visibility: medium-high when wrong (units float above deck,
fall through, miss-targeting). Impact: bridges are a known parity problem area.
Tier 2 here because separate Tier 2 work is already addressing it.

---

## 12. Foundation walk / footprint enumeration (Tier 2)

**What it does.** For a building's footprint (a list of relative cell offsets defining
its shape), enumerate the actual cells occupied at world-position P. Used at placement
preview, occupancy mark/unmark, damage radius, scaffold spawn, debris drop, and bib
rendering.

**Why foundational.** Foundation cells determine where buildings *are*. If the walk order
or relative offsets differ from gamemd, occupancy marks the wrong cells (units phase
through buildings or get blocked from clear cells). Cross-cutting building primitive.

**What's already known.**
- [BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md](BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md) —
  placement-side walk.
- [FOUNDATION_CENTER_INVESTIGATION.md](FOUNDATION_CENTER_INVESTIGATION.md) — center
  computation.
- [BIB_SYSTEM_GHIDRA_REPORT.md](BIB_SYSTEM_GHIDRA_REPORT.md) — bib (sub-foundation) walk.
- [BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md](BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md) —
  per-cell render walk.

**What's missing.**
- A single "Foundation" struct / data document — what's the in-memory representation
  (offset list? bitmask? pre-baked cell deltas?), and how it's derived from the
  `Foundation=` art.ini key.
- Exact iteration order at occupancy mark vs unmark vs draw — if these differ, edge cases
  (mid-construction, mid-destruction) show inconsistencies.
- How non-rectangular foundations (L-shapes, custom Foundation= entries) are walked.

**Where to start.**
- Cross-reference [BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md](BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md)
  with the bib doc to see if they share a walker.
- Look for the parser of `Foundation=` (in art.ini) — gives the internal structure.

**Suggested entry point.** `/plan-investigation foundation walk and footprint enumeration`.

**Priority justification.** Triggers per building lifecycle event (placement, damage,
destruction) and per render frame. Visibility: low per-event but compounds. Impact:
building correctness — placement bugs are one of the loudest parity issues. Tier 2.

---

## 13. Cell-overlap pushing / scatter geometry (Tier 2)

**What it does.** When two entities are forced into overlapping positions (vehicle
crushing infantry, mind-controlled unit displacing original occupant, building collapsing
on units, garrison destroyed and occupants ejecting), how does the engine pick which
sub-cell or neighbor cell each entity ends up in? The geometric resolution of "two
things, one cell."

**Why foundational.** Determines exit positions for every scatter event, every garrison
destruction, every IFV-mount overflow. Cumulative drift across a match is high (every
scatter event ends up in a slightly different cell).

**What's already known.**
- [SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md](SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md) — scatter
  triggers.
- [SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md](SCATTER_TRIGGER_POINTS_GHIDRA_REPORT.md) —
  what causes scatter.
- [UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md](UNIT_COLLISION_AND_REPATH_TRIGGERS_GHIDRA_REPORT.md)
  — collision detection.
- [INFANTRY_SUBCELL_POSITIONING.md](INFANTRY_SUBCELL_POSITIONING.md) — sub-cell rules
  (Tier 1 §5 will cover scatter use of these).
- [CRUSH_SYSTEM_GHIDRA_REPORT.md](CRUSH_SYSTEM_GHIDRA_REPORT.md) — crushing rules.

**What's missing.**
- Geometric resolution: when scatter triggers, in what *order* are neighbor cells tried,
  and how is the chosen cell selected (closest? deterministic-random? predicate-based)?
- Subcell vs cell-level: when an infantryman is bumped, do they stay in the same cell at
  a different subcell, or move to a neighbor?
- Whether mind-controlled units ejecting share the scatter walker with garrison ejection.

**Where to start.**
- Likely shares the Tier 1 §5 (find-nearest spiral) primitive — investigate after §5.

**Suggested entry point.** Possibly fold into Tier 1 §5 as a related variant. If
distinct: `/plan-investigation cell-overlap and scatter resolution geometry`.

**Priority justification.** Triggers on every collision / forced-displacement event.
Visibility: low per-event but accumulates. Impact: scatter pattern feel, garrison eject
positions, crush exit positions. Tier 2.

---

## Suggested order

If tackling sequentially:

**Tier 1 (do these first):**

1. **Facing & rotation** — biggest cross-cutting impact, most isolated to study.
   Plan exists at [docs/plans/2026-05-07-facing-rotation-primitives-investigation-plan.md](../../ra2-rust-game/docs/plans/2026-05-07-facing-rotation-primitives-investigation-plan.md).
2. **AOE / CellSpread** — small, self-contained, consumed by everything combat.
3. **Cell passability** (§6) — read by every path step; verify existing doc first.
4. **Speed → traversal time** (§8) — multiplayer determinism risk; foundational for
   movement parity.
5. **LOS reveal** — verify shroud docs first, then fill cell-walk gap.
6. **Cell-radius enumeration** (§7) — touches many systems; might unify after AOE (§2).
7. **Bullet trajectory** — requires (1) facing for launch direction; verify existing
   trajectory docs first.
8. **Find-nearest** — consumed by fewer systems; lowest Tier 1 urgency.

**Tier 2 (when Tier 1 stable):**

9. **Voxel slope tilt** (§9) — verify-doc first; gameplay-neutral but render-visible.
10. **Foundation walk** (§12) — building-specific; medium reach.
11. **Cell-overlap pushing** (§13) — fold into find-nearest plan if applicable.
12. **Aim leading** (§10) — likely closes quickly with "no lead" confirmation.
13. **Bridge geometry** (§11) — coordinate with Bridges Tier 2 work; may already be in
    that scope.

Each is a candidate for `/plan-investigation` followed by `/re-investigate`. None are
small enough to skip the planning step.

---

## What's NOT on this list (and why)

- **Pathfinding A* internals** — already broadly covered in
  [LOCOMOTION_MATH_AND_CONSTANTS.md §9](LOCOMOTION_MATH_AND_CONSTANTS.md),
  [TERRAIN_COST_FACTSHEET.md](TERRAIN_COST_FACTSHEET.md),
  [ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md](ASTAR_ENTITY_COST_INTEGRATION_GHIDRA_REPORT.md),
  [ZONE_PASSABILITY_VERIFIED.md](ZONE_PASSABILITY_VERIFIED.md). Detail audits welcome
  but not a foundational gap. (Cell passability §6 covers the per-cell predicate
  pathfinding consumes.)
- **Bandbox / drag-rect selection** — covered in
  [BANDBOX_SELECTION_GHIDRA_REPORT.md](BANDBOX_SELECTION_GHIDRA_REPORT.md). UI-specific,
  not a primitive other systems read.
- **Ore growth / spread** — gameplay system layered on cell-radius enumeration (§7).
  Investigate §7 first, then ore growth becomes a consumer audit.
- **Bridge state machine / damage transitions** — separate Tier 2 work in progress.
  Bridge geometry primitives (§11) is the math-level slice that complements that work.
- **Spatial audio panning** — covered in
  [SPATIAL_AUDIO_GHIDRA_REPORT.md](SPATIAL_AUDIO_GHIDRA_REPORT.md). Pure consumer of
  position; no spatial primitive of its own.
- **Tactical autoscroll curve** — covered in
  [TACTICAL_AUTOSCROLL_CURVE_GHIDRA_REPORT.md](TACTICAL_AUTOSCROLL_CURVE_GHIDRA_REPORT.md).
  Camera-input math, not a spatial primitive entities read.
