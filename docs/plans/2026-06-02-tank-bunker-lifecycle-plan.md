# Tank Bunker Lifecycle (Slice 7b) — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. Build + test gate every task. `cargo` package is **`vera20k`** (`cargo test -p vera20k …`); read the literal `test result:` line before reporting pass/fail.

**Goal:** Build the complete stock-`NATBNK` tank-bunker lifecycle — radio admission, the facing-driven 6-state install machine, hide, the three distinct exit/teardown helpers, the reciprocal link, and wall sounds/anims — on the Slice 0–7 mission/radio substrate.

**Architecture:** New sim state (`BunkerLink` on the unit, `BunkerRuntime` on the building) + a `sim/docking/bunker_link.rs` helper module + a Bunker branch in the radio bus + two new player Commands + sim→app wall sound/anim events. `sim/` stays free of render/ui/audio/net.

**Design Doc:** `docs/plans/2026-06-02-tank-bunker-lifecycle-design.md` (Approach A)
**RE backing:** `docs/research/TANK_BUNKER_INSTALL_MICROSTATES_GHIDRA_REPORT.md`, `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md`, `BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md`, `BUNKER_SERVICEDEPOT_0X2E4_RECIPROCAL_LINK_TEARDOWN_GHIDRA_REPORT.md`.

---

## Grounding Summary

- **Docs** verify the full lifecycle (entry radio 0x0F/0x15, install state-5 writes, the 3 distinct teardown helpers, sounds/anims) and the install micro-states (facing-driven, no magic timers; `0x8000`=South; install tracks `0x43–0x46` by octant; exit `0x47`; hide `+0x150` is light but 7b uses full conceal).
- **Ghidra (this session):** install `0x00458E50` (sole caller `MissionRepairAndProduce 0x0044B780`, Bunker-gated), `CanAutoDeployHere 0x0070FB50`, exit `0x004595C0`/`0x004593A0`; FacingClass timer trio `0x004c9220/80/d0`.
- **Repo patterns to mirror:** radio bus `transmit`/`receive_radio` (`src/sim/radio/`); `building_gate: Option<BuildingGateRuntime>` runtime field (`game_entity.rs`); refinery dock FSM (`miner_dock_sequence.rs` — `start_refinery_exit_force_track`, `forced_drive_track`, `facing_target` pivot); `EnterTransport` command (`world_commands.rs:845`); garrison teardown (`production_sell.rs` `eject_garrison_occupants`/`eject_destruction_garrison`); sim→app event vec `bale_events` → `consume_bale_events` (`app_building_anim.rs:420`); `SimSoundEvent` + `app_sim_tick.rs` mapping.
- **Body facing** = `entity.facing: u8` + `facing_target: Option<u8>` (movement tick turns toward it, clears on arrival); South = `0x80`. **NOT** the 16-bit FacingClass (that's turret/`barrel_facing`).
- **INI:** `[NATBNK] Bunker=yes`/`Foundation=2x2`/`NumberOfDocks=1`/`Strength=1000`; `[AudioVisual] BunkerWallsUpSound=TankBunkerUp`/`BunkerWallsDownSound=TankBunkerDown`; `[General] ConditionRed`. Art `[NATBNK]` `SpecialAnim`/`Two`/`Three`/`Four`(+`Damaged`).
- **Still unknown (deferred, non-blocking):** `unit+0x214`→-1 reader (model as clearing pending-nav); COL-verified `+0x150`/`+0x480` decompile (behaviors confirmed from call shape); install tracks `0x43–0x46` raw-track tables presence in `drive_track.rs` (Task 8 confirms/extracts).

## Key Technical Decisions

- **Reciprocal link = `BunkerLink` enum on the unit + `bunker_occupant: Option<u64>` on the building.** — folds approach + installed states into one hashed field; mirrors `PassengerRole`'s enum shape. **Confidence:** high. **Source:** design D1 + `BUNKER_SERVICEDEPOT_0X2E4` Handoff 1.
- **Install is facing-driven (no `MissionTimer` for waits).** State transitions gate on `facing_target.is_none()` (turn done) and `forced_drive_track.is_none()` (track-step done). **Confidence:** high. **Source:** `TANK_BUNKER_INSTALL_MICROSTATES` §3.1 [GHIDRA].
- **Hide = full `conceal`+`remove_entity_occupancy`; each release `reveal`+places.** Output-equivalent to gamemd's light hide; combat/render slice revisits. **Confidence:** high (output), medium (mechanism divergence is documented). **Source:** design D2 + RE report OQ5.
- **`release_clear` (FUN_00459470) is implemented but its triggers are deferred** — super/temporal aren't in sim and the concealed unit isn't damageable in 7b, so no live trigger exists. Keeps the 3-helper contract intact without inventing call sites. **Confidence:** high. **Source:** agent scan (super/temporal NOT FOUND in `src/sim`) + design D3 (unit not damageable while hidden).
- **Wall anims travel sim→app via a new `bunker_wall_events` vec** (mirror `bale_events`), consumed app-side to create `BuildingAnimOverlays` from the building's `kind == Special` art entries (document order: 0/1 = up, 2/3 = down). **Confidence:** high (verified: `SpecialAnim` suffixes parse to `BuildingAnimKind::Special` in order; Task 11 adds the missing `"Four"` suffix at `art_data.rs:1082`). **Source:** `art_data.rs:1082/1120` + `app_building_anim.rs` `consume_bale_events`.
- **Bunker vs refinery routing in the radio bus uses `bunker_runtime.is_some()`, NOT rules.** Verified: `receive_radio` has no `rules` param and `Simulation` owns no `RuleSet`. The spawn-seeded `bunker_runtime` is `Some` only on `Bunker=yes` buildings and is already hashed, so it is the routing key. The rules-gated admission check (`can_auto_deploy_here`) runs at command time (Task 12), never on the bus. **Confidence:** high. **Source:** `radio/receive.rs:36`, `object_type.rs:214/643`.
- **Eject targets the bunker (`EjectBunker { bunker_id }`), not the hidden unit.** Forced by the unit not being rendered/selectable in 7b; behavior identical to gamemd's unit-deploy trigger. **Confidence:** high. **Source:** design D3.

## Open Questions

### Resolved During Planning
- *Do super/temporal teardown triggers exist to hook?* No — not implemented in `src/sim` (agent scan); `release_clear` ships without live call sites (deferred, prerequisite-blocked).
- *How does a stationary unit turn to a facing + signal done?* `entity.facing_target = Some(f)`; movement tick clears it on arrival; done = `facing_target.is_none()` (MCV-deploy/miner-pivot pattern).
- *Building-death teardown helper?* `release_sell_destroy` (gamemd ReceiveDamage case 4 → UndockUnit), not `release_clear`.

### Deferred to Implementation
- Install tracks `0x43–0x46` raw-track tables: confirm present in `drive_track.rs` else extract the 4 TurnTrack entries (siblings of track 15) from Ghidra `g_DriveTrackIndex_Table` (Task 8).
- `unit+0x214`→-1 reader identity (model as clearing the unit's pending-nav/`facing_target`/`movement_target` on hide; verify no regression).
- Exact octant→8-bit-facing boundaries for the install track choice (the gamemd map is on 16-bit facing `>>7`; Rust body facing is 8-bit — Task 7 derives the equivalent 8-bit octant→track map and tests it).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/game_entity.rs` | add `bunker_link: BunkerLink`, `bunker_runtime: Option<BunkerRuntime>`; init |
| Create | `src/sim/docking/bunker_link.rs` | install/break + 3 release helpers + despawn safety net + `can_auto_deploy_here` |
| Create | `src/sim/docking/bunker_install.rs` | `BunkerRuntime`/`BunkerState` + `tick_bunker_install` 6-state machine |
| Modify | `src/sim/docking/mod.rs` | `pub mod bunker_link; pub mod bunker_install;` |
| Modify | `src/sim/radio/receive.rs` | Structure branch → bunker vs refinery; `bunker_receive` |
| Modify | `src/sim/command.rs` | `EnterBunker`, `EjectBunker` variants |
| Modify | `src/sim/world/world_commands.rs` | dispatch + validation for the two commands |
| Modify | `src/sim/world/mod.rs` | `SimSoundEvent::BunkerWallsUp/Down`; `bunker_wall_events` vec; tick call; uninit hook; world_hash fold |
| Modify | `src/sim/world/world_hash.rs` | fold `bunker_link` + `bunker_runtime` |
| Modify | `src/sim/production/production_sell.rs` | sell teardown hook |
| Modify | `src/rules/ruleset.rs` | parse `BunkerWallsUpSound` |
| Modify | `src/audio/events.rs` | `GameSoundEvent::BunkerWallsUp/Down` |
| Modify | `src/app_sim_tick.rs` | map the two sound events |
| Modify | `src/app_building_anim.rs` | consume `bunker_wall_events` → overlays |
| Modify | `src/sim/movement/drive_track.rs` | confirm/extract install tracks `0x43–0x46` |
| Modify | `src/rules/art_data.rs` | parse `SpecialAnimFour` (add `"Four"` suffix at `:1082`) |

## Interface Changes

- **`GameEntity`** gains two hashed fields — anything iterating/serializing entities is unaffected (additive, `#[serde(default)]`).
- **`Command`** gains two variants — every `match` on `Command` must add arms (the dispatch in `world_commands.rs`; any exhaustive match in tests/replay). Grep `Command::` matches before finalizing.
- **`SimSoundEvent`/`GameSoundEvent`** gain two variants each — the `app_sim_tick.rs` drain match must handle them.
- **`radio::receive_radio` Structure branch** changes from refinery-only to bunker-vs-refinery — refinery behavior must stay bit-identical (Task 6 guards this).

## Sim Checklist

- [ ] All math integer/`u8`/`u16` facing + cell coords — no f32/f64 in sim logic (atan2 facing uses the existing integer `facing_from_delta`/lepton helper, not float).
- [x] `bunker_link` + `bunker_runtime` folded into `world_hash` (Task 3). No re-baseline needed (no absolute golden-hash constant; determinism tests are relative — verified).
- [ ] No `sim/` dependency on render/ui/sidebar/audio/net (wall anims/sounds go out via event vecs).
- [ ] Tick ordering: `tick_bunker_install` runs in the docks sub-phase (alongside `building_dock::tick_building_docks`, `world/mod.rs:~2289`); teardown hooks fire in the existing sell/death paths.
- [ ] BTreeMap iteration: `tick_bunker_install` walks `keys_sorted()` (deterministic).

## Risk Areas

- **The passability gate goes live.** `bunker_occupant` was read-but-never-set; install/release now write it → occupied bunkers block cells. Regression: `cell_entry.rs:948` covers both states; add an integration test (Task 12).
- **Refinery dock must not regress** when the Structure receiver branches (Task 6) — run the full `miner::` suite.
- **One-sided link** = stuck/ghost unit. Every teardown clears both sides; despawn safety net catches stragglers (Tasks 5, 9).
- **Hash fold** (Task 3): the two new fields enter the entity fold. Verified there is NO absolute golden-hash baseline — every determinism test is relative — so no re-baseline is needed; just confirm the relative + replay tests stay green.
- **Command match exhaustiveness** — adding variants breaks any non-`_` match; grep first.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 4 | Admission gate (`Bunker=yes` ∧ own-owner ∧ `Bunkerable` ∧ has-weapon ∧ not-occupied) | wrong gate lets infantry/enemy/NABNKR in or rejects valid tanks — every bunker use | `bunker_entry_requires_bunker_flag_and_bunkerable_vehicle`; [GHIDRA 0x0070FB50/0x0043C2D0] |
| 5 | **3 distinct teardown helpers, both-sides clear, sound matrix** (up=install; down=normal+clear, NOT sell_destroy) | collapsing them duplicates/loses/traps units; wrong sound | `bunker_clear_path_dispatch_correct_by_trigger`; [doc 2026-06-02 §5] |
| 5/9 | `release_normal` places at `Find_Nearby_Passable_Cell` from building-NW `(-1,+1)` + Move; `release_sell_destroy` places at the building cell, no Move/sound | wrong exit position/idle vs move is visible every eject/sell | [GHIDRA 0x004595C0/0x004593A0] |
| 7 | Install state-5 write order + South-facing + entry anim before hide | walls-up→tank-gone sequence + facing the player sees | [GHIDRA 0x00458E50 case 4/5] |
| 7/8 | Install force-track `0x43–0x46` by octant (target=building coord, NO `±0x80`) | sub-cell entry step; wrong track = wrong entry curve | [GHIDRA 0x00458E50 state 2] + drive_track data |
| 10 | Wall sounds: up on install only; down on normal+clear; play only if id≠empty; positional at building | audible every entry/exit | [GHIDRA + rulesmd.ini:719/720] |
| 11 | Entry anims = Special order 0/1 (SpecialAnim/Two), exit = 2/3 (Three/Four), health-gated on ConditionRed; **`SpecialAnimFour` parse added** (was dropped) | the walls-up/down visual | [GHIDRA case 4 / 0x004595C0] + art `[NATBNK]` + `art_data.rs:1082` |

---

## Tasks

### Task 1: `BunkerLink` enum + GameEntity fields

**Why:** the reciprocal-link data model; everything else writes/reads it. Types first.

**Files:** Modify `src/sim/game_entity.rs`

**Pattern:** mirrors `PassengerRole` (enum field) + `building_gate: Option<BuildingGateRuntime>` (Option runtime field).

**Step 1 — define `BunkerLink`** (new module-level enum near the other `game_entity` enums, or top of the file):
```rust
/// The unit side of the tank-bunker reciprocal link (gamemd `TechnoClass+0x2E4`
/// plus the pre-install approach state, folded into one field — distinct from
/// `PassengerRole` cargo: a bunker is a single reciprocal link, never cargo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum BunkerLink {
    /// Not heading to or inside any bunker.
    #[default]
    None,
    /// Ordered into bunker `id`, still approaching (pre-install). Cleared on any
    /// retask → lets the building install machine reset. (gamemd has no field for
    /// this — it tracks the candidate via the building's destination; the explicit
    /// unit-side marker is the Rust-native abort signal.)
    Approaching(u64),
    /// Installed inside bunker `id` (reciprocal of `building.bunker_occupant`).
    Installed(u64),
}

impl BunkerLink {
    /// The bunker this unit is installed in, if any.
    pub fn installed_in(self) -> Option<u64> {
        match self { BunkerLink::Installed(id) => Some(id), _ => None }
    }
    /// The bunker this unit is approaching, if any.
    pub fn approaching(self) -> Option<u64> {
        match self { BunkerLink::Approaching(id) => Some(id), _ => None }
    }
}
```

**Step 2 — add the field** to `GameEntity`, immediately after `bunker_occupant: Option<u64>,` (verified `game_entity.rs:427`):
```rust
    /// Unit side of the bunker reciprocal link (approach + installed states).
    #[serde(default)]
    pub bunker_link: BunkerLink,
```

**Step 3 — add the building-side runtime field** after `building_gate: Option<BuildingGateRuntime>,` (verified `:433`):
```rust
    /// Tank-bunker install state machine. `Some` on `Bunker=yes` buildings from
    /// spawn (state `Idle` when empty). Drives entry admission → install.
    #[serde(default)]
    pub bunker_runtime: Option<crate::sim::docking::bunker_install::BunkerRuntime>,
```

**Step 4 — init in `new()`** (verified `:626` `bunker_occupant: None,` and `:627` `building_gate: None,`):
```rust
            bunker_occupant: None,
            bunker_link: BunkerLink::None,
            building_gate: None,
            bunker_runtime: None,
```
(`bunker_runtime` is seeded to `Some(BunkerRuntime::idle())` for `Bunker=yes` buildings at spawn — wired in Task 7's spawn hook; default `None` here is correct for all non-bunker entities.)

**Step 5 — verify:** `cargo check -p vera20k` (will fail until Task 7 creates `bunker_install`; if so, temporarily comment the `bunker_runtime` field type to `Option<()>` is NOT allowed — instead do Task 7's module stub first if `check` blocks. Prefer: create the empty `bunker_install.rs` stub with just `pub struct BunkerRuntime;` + `impl BunkerRuntime { pub fn idle() -> Self { Self } }` now, fleshed out in Task 7).

**Step 6 — commit.**

---

### Task 2: `bunker_install.rs` skeleton — `BunkerRuntime` + `BunkerState`

**Why:** the building-side state type that Task 1's field references; defining it now unblocks `cargo check`. The tick logic lands in Task 7.

**Files:** Create `src/sim/docking/bunker_install.rs`; Modify `src/sim/docking/mod.rs`.

**Pattern:** `BuildingGateRuntime` (a small serde runtime struct on the building).

**Step 1 — module file:**
```rust
//! Tank-bunker install state machine (building side).
//!
//! Models gamemd's `Bunker=yes` mission helper (`0x00458E50`): a facing-driven
//! 6-state machine that, once a candidate unit is on the footprint, shoves
//! blockers, turns the unit to face the building, force-tracks it onto the
//! building cell, turns it South, plays entry anims, then installs (hide +
//! reciprocal link + up sound). Waits are turn/track completions — NOT timers.
//!
//! sim/ only — never render/ui/sidebar/audio/net.
use serde::{Deserialize, Serialize};

/// Install progress (maps 1:1 to gamemd `BuildingClass+0x718`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum BunkerState {
    /// Empty / not installing (gamemd +0x718 == 0 with no candidate).
    #[default]
    Idle,
    /// Candidate admitted; waiting for it to arrive on the footprint + stop, then shove blockers.
    ArriveWait,
    /// Waiting for the footprint to clear of other objects, then face the building.
    ClearWait,
    /// Turning the unit to face the building.
    TurnToBuilding,
    /// Force-track sub-cell step onto the building cell in progress.
    TrackStep,
    /// Turning the unit to South (gamemd desired facing 0x8000 → Rust 0x80).
    TurnSouth,
    /// Installed (gamemd +0x718 == 6).
    Occupied,
}

/// Building-side bunker runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BunkerRuntime {
    pub state: BunkerState,
    /// Candidate unit during ArriveWait..TurnSouth; `None` when Idle/Occupied
    /// (Occupied tracks the occupant via `GameEntity.bunker_occupant`).
    pub installing_unit: Option<u64>,
}

impl BunkerRuntime {
    pub fn idle() -> Self { Self { state: BunkerState::Idle, installing_unit: None } }
}
```

**Step 2 — register module** in `src/sim/docking/mod.rs`:
```rust
pub mod bunker_install;
pub mod bunker_link;
```
(create `bunker_link.rs` as an empty file now to satisfy the `mod` — fleshed out in Tasks 4/5; or add `pub mod bunker_link;` in Task 4. If adding now, create a one-line `bunker_link.rs` with `//! Tank-bunker reciprocal link helpers.`).

**Step 3 — verify:** `cargo check -p vera20k` → PASS (with Task 1's field now resolving).

**Step 4 — commit.**

---

### Task 3: World-hash fold for the two new fields

**Why:** lockstep determinism — new persistent state must be hashed. Do it early so the re-baseline is a single known point.

**Files:** Modify `src/sim/world/world_hash.rs` (the per-entity fold, near the existing `entity.bunker_occupant.hash(hasher)` at `:497`).

**Step 1 — fold** immediately after the existing `entity.bunker_occupant.hash(hasher);` line:
```rust
        entity.bunker_occupant.hash(hasher);
        // Slice 7b: reciprocal link + install machine are authoritative lifecycle state.
        entity.bunker_link.hash(hasher);
        entity.bunker_runtime.hash(hasher);
```
(`BunkerLink` and `BunkerRuntime` both derive `Hash` — Tasks 1/2.)

**Step 2 — verify:** `cargo test -p vera20k --lib state_hash` → PASS. **Verified during planning:** every hash test in the suite is *relative* (run-twice-compare, or live-vs-replay in `tests/determinism_replay.rs`). There is **no absolute golden-hash constant anywhere**, so adding the two default-valued fields breaks nothing and needs **no re-baseline** — at fold level OR globally. Both compared runs carry identical defaults.

**Step 3 — commit.**

---

### Task 4: `bunker_link.rs` — `install_bunker_link` + `break_bunker_link` + `can_auto_deploy_here`

**Why:** the core reciprocal-link writes (the install commit) + the shared link-clear primitive + the admission predicate. Foundation for the machine (Task 7) and the radio branch (Task 6).

**Files:** Modify `src/sim/docking/bunker_link.rs`.

**Pattern:** free functions on `&mut Simulation` (like `radio::receive` and `mission::retask`); reuse `conceal`/`remove_entity_occupancy`/`transmit`.

**Step 1 — `can_auto_deploy_here`** (ledger item 1; `[GHIDRA 0x0070FB50]`):
```rust
use crate::sim::world::Simulation;
use crate::sim::game_entity::BunkerLink;
use crate::sim::mission::{verb, MissionType};
use crate::rules::RuleSet;

/// gamemd `TechnoClass::CanAutoDeployHere @ 0x0070FB50`: the per-unit half of the
/// bunker admission gate. Requires a Bunkerable vehicle with a primary weapon.
/// (Movement-zone/busy-guard sub-checks at 0x0070FB50 are not reproduced here —
/// they exclude no stock bunkerable vehicle; see RE report ledger item 1.)
pub fn can_auto_deploy_here(sim: &Simulation, unit_id: u64, rules: &RuleSet) -> bool {
    let Some(unit) = sim.substrate.entities.get(unit_id) else { return false };
    let Some(obj) = sim.object_type(unit.type_ref, rules) else { return false };
    obj.bunkerable && obj.primary.is_some()
}
```
*(Verified: `ObjectType` exposes `pub primary: Option<String>` (`object_type.rs:214`) = the primary weapon ID, and `bunkerable: bool` (`:643`). Do NOT use `weapon_list` — that is IFV-Gunner-only (`:645`) and empty for normal tanks. Gate = `bunkerable && primary.is_some()`. This is called from the Task 12 command dispatch (which has `rules`), NOT from the radio bus.)*

**Step 2 — `install_bunker_link`** (ledger item 6; `[GHIDRA 0x00458E50 case 5]`):
```rust
/// Install state 5: write both reciprocal links, clear the unit's pending nav,
/// hide it (full limbo — combat/render is deferred), Guard mission, and signal
/// the wall-up sound. Entry anims are emitted by the machine just before this.
pub fn install_bunker_link(sim: &mut Simulation, building_id: u64, unit_id: u64) {
    // Write both sides (gamemd order: building+0x2E4, then unit+0x2E4, then unit+0x214=-1).
    if let Some(b) = sim.substrate.entities.get_mut(building_id) {
        b.bunker_occupant = Some(unit_id);
        if let Some(rt) = b.bunker_runtime.as_mut() {
            rt.state = crate::sim::docking::bunker_install::BunkerState::Occupied;
            rt.installing_unit = None;
        }
    }
    let now = sim.binary_frame;
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        u.bunker_link = BunkerLink::Installed(building_id);
        // unit+0x214 = -1 analogue: clear any pending navigation/turn so the
        // hidden unit holds no stale destination (RE OQ4, behavioral).
        u.movement_target = None;
        u.facing_target = None;
        u.forced_drive_track = None;
        verb::assign_mission(&mut u.mission, MissionType::Guard, now);
    }
    // Hide: remove from cell occupancy + leave the active set (full conceal).
    sim.remove_entity_occupancy(unit_id);
    sim.conceal(unit_id);
    // Wall-up sound (Task 10 adds the event; emit here).
    emit_bunker_wall_sound(sim, building_id, /*up=*/true);
}
```

**Step 3 — `break_bunker_link`** (core both-sides clear + BREAK; `[GHIDRA, doc §3]`):
```rust
/// Clear BOTH sides of the link and send the radio BREAK. Returns the unit id
/// that was installed (for callers that re-place it). Does NOT reveal/place/anim.
pub fn break_bunker_link(sim: &mut Simulation, building_id: u64) -> Option<u64> {
    let unit_id = sim.substrate.entities.get(building_id)?.bunker_occupant?;
    // BREAK over the bus (clears the radio contact both ways; gamemd vtable+0x274(3)).
    crate::sim::radio::transmit(
        sim, building_id, unit_id,
        crate::sim::radio::RadioMessage::Break,
        crate::sim::radio::RadioPayload::default(),
    );
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        u.bunker_link = BunkerLink::None;
    }
    if let Some(b) = sim.substrate.entities.get_mut(building_id) {
        b.bunker_occupant = None;
    }
    Some(unit_id)
}
```

**Step 4 — `emit_bunker_wall_sound` placeholder shim** so this compiles before Task 10:
```rust
/// Emits the positional wall sound event. Real body lands in Task 10 (sound events).
fn emit_bunker_wall_sound(sim: &mut Simulation, building_id: u64, up: bool) {
    let Some(b) = sim.substrate.entities.get(building_id) else { return };
    let (rx, ry) = (b.position.rx, b.position.ry);
    sim.sound_events.push(if up {
        crate::sim::world::SimSoundEvent::BunkerWallsUp { rx, ry }
    } else {
        crate::sim::world::SimSoundEvent::BunkerWallsDown { rx, ry }
    });
}
```
*(This references `SimSoundEvent::BunkerWallsUp/Down` — add them in Task 10 BEFORE compiling this, or do Task 10's enum-variant step first. Ordering note: do Task 10 Step 1 (enum variants) before Task 4 Step 4 to keep `cargo check` green.)*

**Step 5 — unit tests** (`#[cfg(test)] mod tests` in `bunker_link.rs`): build a sim with a `Bunker=yes` building + a bunkerable vehicle; assert `install_bunker_link` sets `building.bunker_occupant == Some(unit)`, `unit.bunker_link == Installed(building)`, unit is concealed (`!in_logic_vector`), unit mission == Guard, and one `BunkerWallsUp` sound event; assert `break_bunker_link` clears both sides and returns the unit. (Mirror the spawn helpers in `radio/receive.rs` tests.)

**Step 6 — verify:** `cargo test -p vera20k bunker_link` → PASS.

**Step 7 — commit.**

---

### Task 5: `bunker_link.rs` — the three release helpers + despawn safety net

**Why:** the exit/teardown contract — three distinct behaviors that must NOT be collapsed (ledger items 7/8/9).

**Files:** Modify `src/sim/docking/bunker_link.rs`.

**Step 1 — `release_normal`** (`[GHIDRA 0x004595C0]`): clear entry anim → down sound → exit anim (health-gated) → `break_bunker_link` → `reveal` + place at `find_nearby_passable_cell(building_NW + (-1,+1))` + `add_entity_occupancy` → unit Move mission → reset `bunker_runtime` to Idle.
```rust
pub fn release_normal(sim: &mut Simulation, building_id: u64, rules: &RuleSet) {
    emit_bunker_wall_anim(sim, building_id, /*up=*/false); // exit anims (Task 11); also emits down sound
    emit_bunker_wall_sound(sim, building_id, /*up=*/false);
    let Some(unit_id) = break_bunker_link(sim, building_id) else {
        reset_bunker_idle(sim, building_id);
        return;
    };
    let cell = bunker_exit_search_cell(sim, building_id); // building NW + (-1,+1), then nearby-passable
    let now = sim.binary_frame;
    if let Some((rx, ry)) = cell {
        if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
            u.position.rx = rx; u.position.ry = ry;
        }
        sim.reveal(unit_id);
        sim.add_entity_occupancy(unit_id);
    }
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        verb::assign_mission(&mut u.mission, MissionType::Move, now);
    }
    reset_bunker_idle(sim, building_id);
    let _ = rules;
}
```

**Step 2 — `release_sell_destroy`** (`[GHIDRA 0x004593A0]`): NO sound/anims; `break_bunker_link`; reveal + place at the **building cell** (gamemd Head_To SE half-cell, no nearby-passable search, no Move mission). Does **not** reset state via the normal path (gamemd doesn't clear +0x718 here, but the building is about to despawn — reset is harmless; place the unit so it survives the despawn).
```rust
pub fn release_sell_destroy(sim: &mut Simulation, building_id: u64) {
    let Some((brx, bry)) = sim.substrate.entities.get(building_id).map(|b| (b.position.rx, b.position.ry)) else { return };
    let Some(unit_id) = break_bunker_link(sim, building_id) else { return };
    if let Some(u) = sim.substrate.entities.get_mut(unit_id) {
        u.position.rx = brx; u.position.ry = bry; // gamemd leaves it at the building cell, idle (no Move)
        u.facing = 0x80; // South per facing convention (0x80=S); verify exact UndockUnit head vs RE report before ship
    }
    sim.reveal(unit_id);
    sim.add_entity_occupancy(unit_id);
    // No Move mission, no sound, no anims (matches UndockUnit).
}
```
*(Mechanism note: gamemd never re-places here because its hide is light; 7b's full-conceal model requires the reveal+place to reproduce the visible "unit at the building cell, idle" result. Documented divergence — design D2.)*

**Step 3 — `release_clear`** (`[GHIDRA 0x00459470]`): clear anim → down sound if occupied → exit anims → `break_bunker_link` → reset Idle. **No reveal/place/Move** (the unit is dead or warped). **No live trigger in 7b** (super/temporal absent; concealed unit not damageable) — ship it; call sites deferred.
```rust
/// Clear-only teardown (super / temporal-non-building / unit-death). Clears both
/// links + plays down sound/anims, but does NOT reposition the unit. Implemented
/// for contract completeness; no live trigger exists in 7b (prerequisite systems
/// absent + the concealed unit is not damageable until the combat slice lands).
pub fn release_clear(sim: &mut Simulation, building_id: u64) {
    if sim.substrate.entities.get(building_id).and_then(|b| b.bunker_occupant).is_some() {
        emit_bunker_wall_anim(sim, building_id, false);
        emit_bunker_wall_sound(sim, building_id, false);
        break_bunker_link(sim, building_id);
    }
    reset_bunker_idle(sim, building_id);
}
```

**Step 4 — helpers** `reset_bunker_idle` (set `bunker_runtime` to Idle + building Guard mission), `bunker_exit_search_cell` (building NW `(-1,+1)` then nearby-passable — reuse the passenger 8-neighbor/`Find_Nearby_Passable_Cell` search; grounding: mirror `passenger.rs` `NEIGHBORS` or `production_sell.rs` perimeter), and a `emit_bunker_wall_anim` shim (real body Task 11).

**Step 5 — `break_links_on_despawn`** (safety net, called from `uninit` Task 9):
```rust
/// Despawn safety net: if `id` is a bunker with an occupant, or a unit installed
/// in a bunker, clear the reciprocal side. No anims/sound/placement.
pub fn break_links_on_despawn(sim: &mut Simulation, id: u64) {
    let Some(e) = sim.substrate.entities.get(id) else { return };
    if let Some(unit_id) = e.bunker_occupant {
        if let Some(u) = sim.substrate.entities.get_mut(unit_id) { u.bunker_link = BunkerLink::None; }
    }
    if let Some(building_id) = e.bunker_link.installed_in() {
        if let Some(b) = sim.substrate.entities.get_mut(building_id) {
            b.bunker_occupant = None;
            if let Some(rt) = b.bunker_runtime.as_mut() { *rt = crate::sim::docking::bunker_install::BunkerRuntime::idle(); }
        }
    }
}
```

**Step 6 — tests:** `bunker_clear_path_dispatch_correct_by_trigger` — install, then each release; assert: `release_normal` → both cleared, unit revealed at a passable cell near the building, Move mission, one down sound; `release_sell_destroy` → both cleared, unit at building cell, NO Move/sound; `release_clear` → both cleared, down sound, unit NOT revealed/repositioned. Plus `break_links_on_despawn` from each side.

**Step 7 — verify:** `cargo test -p vera20k bunker_link` → PASS.

**Step 8 — commit.**

---

### Task 6: Radio bus bunker admission branch

**Why:** entry admission is gamemd's actual mechanism (HELLO→CAN_ENTER→DockNow). Reuses the Slice-4 bus; refinery must stay bit-identical.

**Files:** Modify `src/sim/radio/receive.rs`.

**Step 1 — branch the Structure receiver.** Replace the `Structure => refinery_receive(...)` arm (`receive.rs:49`) with a bunker-vs-refinery dispatch. **Verified: `receive_radio(sim, target_sid, sender_sid, msg, payload)` has NO `rules` param and `Simulation` owns no `RuleSet`** (`receive.rs:36`; rules is threaded as a param everywhere else). So do NOT read `Bunker=yes` here. Use the spawn-seeded `bunker_runtime`: it is `Some` only on `Bunker=yes` buildings (Task 7 seed), so `bunker_runtime.is_some()` IS "is this a tank bunker" — no rules, no new field, already hashed (Task 3).
```rust
EntityCategory::Structure => {
    if is_bunker_building(sim, target_sid) {
        bunker_receive(sim, target_sid, sender_sid, msg)
    } else {
        refinery_receive(sim, target_sid, sender_sid, msg)
    }
}
```
```rust
/// A tank bunker is any structure seeded with a `bunker_runtime` (Bunker=yes at spawn).
fn is_bunker_building(sim: &Simulation, sid: u64) -> bool {
    sim.substrate.entities.get(sid).is_some_and(|b| b.bunker_runtime.is_some())
}
```

**Step 2 — `bunker_receive`:**
```rust
fn bunker_receive(sim: &mut Simulation, bld: u64, sender: Option<u64>, msg: RadioMessage) -> RadioResponse {
    let Some(unit) = sender else { return RadioResponse::None };
    match msg {
        RadioMessage::CanEnter => {
            // own-owner ∧ alive ∧ not occupied/installing ∧ CanAutoDeployHere
            if bunker_admits(sim, bld, unit) { RadioResponse::Roger } else { RadioResponse::Negatory }
        }
        RadioMessage::DockNow => {
            // Commit: start the install machine (gamemd case 0x15 → building mission 0x14).
            if let Some(b) = sim.substrate.entities.get_mut(bld) {
                if let Some(rt) = b.bunker_runtime.as_mut() {
                    if rt.state == BunkerState::Idle {
                        rt.state = BunkerState::ArriveWait;
                        rt.installing_unit = Some(unit);
                    }
                }
            }
            RadioResponse::Roger
        }
        RadioMessage::Break => { // teardown handled by bunker_link helpers; clear contact only
            if let Some(b) = sim.substrate.entities.get_mut(bld) { b.radio_contacts.remove(unit); }
            RadioResponse::None
        }
        _ => RadioResponse::None,
    }
}
```
`bunker_admits` = own-owner (owner equality, matching `refinery_hello`) ∧ `!dying && health>0` ∧ `bunker_occupant.is_none()` ∧ `bunker_runtime.state == Idle`. **Sim-state only — no rules.** The rules-gated `can_auto_deploy_here` (Bunkerable + has-weapon) is checked once at command time (Task 12 dispatch, which has `rules`) BEFORE the handshake transmit, so the bus never needs rules.

**Step 3 — tests:** `bunker_bus_routes_and_admits_by_sim_state` — for a seeded bunker, CanEnter from an own-owner sender with the bunker Idle/empty → Roger; enemy-owner / already-occupied / non-Idle → Negatory; a refinery (no `bunker_runtime`) still routes to `refinery_receive` (HELLO Roger — refinery-unchanged assertion). The Bunkerable + has-weapon rejection (infantry / `Bunkerable=no`) is rules-gated and tested at the command level (Task 12 `enter_bunker_rejects_non_bunkerable`), not on the bus.

**Step 4 — verify:** `cargo test -p vera20k radio` AND `cargo test -p vera20k miner` (refinery regression) → PASS.

**Step 5 — commit.**

---

### Task 7: Install state machine `tick_bunker_install` + spawn seed

**Why:** the 6-state facing-driven install (ledger item 3). The behavioral heart.

**Files:** Modify `src/sim/docking/bunker_install.rs`; Modify `src/sim/world/mod.rs` (tick call + spawn seed).

**Step 1 — `tick_bunker_install(sim)`** walking `keys_sorted()` over buildings with `bunker_runtime.state != Idle/Occupied`. Per the candidate `installing_unit`, run the state per ledger item 3. Use:
- arrival/stopped check: candidate on the building footprint (`entity_occupancy_cells(building)` contains the unit's cell) AND not moving (`movement_target.is_none() && forced_drive_track.is_none()`).
- **ArriveWait:** if candidate's `bunker_link != Approaching(this)` → abort (reset Idle). If on footprint + stopped → shove every other unit off the footprint (reuse the existing scatter — grep `scatter`/`blocked_scatter`; mirror `production`/movement scatter), then → ClearWait.
- **ClearWait:** if footprint has any other live unit → stay; else compute `f = facing_toward(unit→building)` (use `facing_from_delta` on the cell delta), set `unit.facing_target = Some(f)`; → TurnToBuilding.
- **TurnToBuilding:** if `unit.facing_target.is_some()` (still turning) → stay; else pick the install track from `unit.facing` octant (Task 8 map) and start the force-track via `start_bunker_install_force_track(unit, track, building_coord)`; → TrackStep.
- **TrackStep:** if `unit.forced_drive_track.is_some()` or moving → stay; else set `unit.facing_target = Some(0x80)` (South); → TurnSouth.
- **TurnSouth:** if `unit.facing_target.is_some()` → stay; else emit entry-anim event (health-gated; Task 11) and call `bunker_link::install_bunker_link(sim, building, unit)`; the install sets state to Occupied.

**Step 2 — `start_bunker_install_force_track`** in `drive_track`/`miner_dock_sequence` style but with **no `±0x80` offset** (target = the building cell coord), track ∈ `0x43..=0x46`. Mirror `start_refinery_exit_force_track` (`miner_dock_sequence.rs:540`).

**Step 3 — spawn seed:** where buildings are finalized at spawn (grep the building-spawn finalizer that sets `building_gate` for `Gate=yes`), set `bunker_runtime = Some(BunkerRuntime::idle())` when `object_type(...).bunker`. Mirror the `building_gate` seed.

**Step 4 — tick wiring:** call `bunker_install::tick_bunker_install(self)` in `advance_tick`'s docks sub-phase, immediately after `building_dock::tick_building_docks(self, rules)` (verified `world/mod.rs:2289`).

**Step 5 — tests:** state-progression test — admit (set `Approaching` + `ArriveWait` + place unit on footprint stopped), advance ticks, assert the machine walks ArriveWait→…→Occupied and `install_bunker_link` fired (occupant set, unit concealed); abort test — retask the unit (clear `bunker_link`) mid-approach → machine resets to Idle.

**Step 6 — verify:** `cargo test -p vera20k bunker_install` → PASS.

**Step 7 — commit.**

---

### Task 8: Confirm/extract install force-track tables `0x43–0x46`

**Why:** the install force-track (Task 7 Step 2) needs raw-track point data for tracks `0x43–0x46`, like `0x47`/track-15 already in `drive_track.rs`.

**Files:** Modify `src/sim/movement/drive_track.rs` (only if missing).

**Step 1 — grounding:** read `drive_track.rs`; check whether TurnTrack indices `0x43..=0x46` (67–70) and their raw tracks are present (the `0x47`/raw-15 mapping exists per `miner_tests.rs:3883`). 

**Step 2 — if present:** no change; record the raw-track indices each maps to (for Task 7's octant map). **If missing:** extract the 4 TurnTrack entries from Ghidra `g_DriveTrackIndex_Table + index*12` (`0x007E7E7C` is index 71; indices 67–70 are at `0x007E7E7C − 4*12 .. −12`) and their raw-track point tables, exactly as track-15 was extracted. State the addresses read in the commit message (kept out of code comments per project rule).

**Step 3 — verify:** `cargo test -p vera20k drive_track` → PASS (add a data-presence assertion for the 4 indices).

**Step 4 — commit.**

---

### Task 9: Teardown hooks — sell + building death + despawn safety net

**Why:** wire the live triggers (`release_sell_destroy` on sell/death) and the `uninit` safety net.

**Files:** Modify `src/sim/production/production_sell.rs`, `src/sim/world/mod.rs`.

**Step 1 — sell hook:** in `sell_building` (`production_sell.rs:691`), **before** `eject_garrison_occupants` (`:713`) / `uninit` (`:716`), add:
```rust
    if sim.substrate.entities.get(stable_id).and_then(|b| b.bunker_occupant).is_some() {
        crate::sim::docking::bunker_link::release_sell_destroy(sim, stable_id);
    }
```

**Step 2 — death hook:** in `world/mod.rs` where destroyed buildings are uninit'd (`:2075-2080`, the `combat_result.immediate_uninit_ids` loop), **before** `self.uninit(dead_id)`, add the same occupied-bunker → `release_sell_destroy(self, dead_id)` guard. (Mirrors the existing `eject_destruction_garrison` placement.)

**Step 3 — despawn safety net:** in `uninit` (`world/mod.rs:1010`), after `self.clear_radio_contacts_for(stable_id);` (`:1025`) and before `self.conceal(stable_id);` (`:1026`), add:
```rust
        crate::sim::docking::bunker_link::break_links_on_despawn(self, stable_id);
```

**Step 4 — tests:** `occupied_bunker_sell_clears_links_unit_at_building_cell`; `occupied_bunker_death_clears_links`; `despawn_clears_bunker_back_link` (limbo a bunker with an occupant directly → occupant's `bunker_link` cleared).

**Step 5 — verify:** `cargo test -p vera20k bunker` AND `cargo test -p vera20k production` (sell regression) → PASS.

**Step 6 — commit.**

---

### Task 10: Wall sounds — parse + events + app mapping

**Why:** the audible up/down cues (ledger items 10/12).

**Files:** Modify `src/rules/ruleset.rs`, `src/sim/world/mod.rs`, `src/audio/events.rs`, `src/app_sim_tick.rs`.

**Step 1 — `SimSoundEvent` variants** (`world/mod.rs`, after `RefineryExitSfx` at `:181`):
```rust
    /// Tank-bunker walls-up — install. [AudioVisual] BunkerWallsUpSound.
    BunkerWallsUp { rx: u16, ry: u16 },
    /// Tank-bunker walls-down — normal exit / clear teardown. BunkerWallsDownSound.
    BunkerWallsDown { rx: u16, ry: u16 },
```
(Do this BEFORE Task 4 Step 4 compiles — see that note.)

**Step 2 — parse `BunkerWallsUpSound`** in `ruleset.rs` beside `bunker_walls_down_sound` (`:286` field, `:932` parse): add `pub bunker_walls_up_sound: Option<String>` + parse from `[AudioVisual] BunkerWallsUpSound` + default `None`.

**Step 3 — `GameSoundEvent` variants** in `audio/events.rs` (mirror `RefineryExitSfx` at `:140`): `BunkerWallsUp { sound_id: String, screen_pos: Option<(f32,f32)> }` and `BunkerWallsDown { … }`.

**Step 4 — map** in `app_sim_tick.rs` drain loop (mirror the `RefineryExitSfx` arm at `:551`): `BunkerWallsUp` → resolve `rules.general.bunker_walls_up_sound`, skip if empty (the `≠ -1` guard); `BunkerWallsDown` → `bunker_walls_down_sound`. Positional via `iso_to_screen`.

**Step 5 — retire the stale mapping:** the `RefineryExitSfx`-as-bunker-down provisional usage had no producer; leave `RefineryExitSfx` for the refinery non-event test but stop treating it as bunker-down (the bunker now uses its own events). Update the stale comment at `audio/events.rs:140` per the design's stale-doc note.

**Step 6 — tests:** `bunker_up_sound_emitted_on_install` (already covered by Task 4 test — extend to assert the app maps it); `bunker_down_on_normal_and_clear_not_sell` (the sound matrix — assert no down event from `release_sell_destroy`).

**Step 7 — verify:** `cargo test -p vera20k bunker` + `cargo test -p vera20k ruleset` → PASS.

**Step 8 — commit.**

---

### Task 11: Wall anims — sim event + app overlay creation

**Why:** the visible walls-up/down (ledger items 4/5/11). Sim emits; app creates the overlay (the one render-touching task — kept in 7b per the design's wall-anim scope).

**Files:** Modify `src/rules/art_data.rs` (parse `SpecialAnimFour`), `src/sim/world/mod.rs` (event vec), `src/sim/docking/bunker_link.rs` (`emit_bunker_wall_anim` body), `src/app_building_anim.rs` (consumer), `src/sim/game_entity.rs` (health-state at emit).

**Step 1 — parser fix (verified gap).** `BUILDING_ANIM_KEYS` (`art_data.rs:1082`) lists `("SpecialAnim", &["", "Two", "Three"])` — only THREE suffixes, so `SpecialAnimFour` (NATBNK `Four=NATBNK_B2/B2D`) is currently **dropped**. Add `"Four"`:
```rust
    ("SpecialAnim", &["", "Two", "Three", "Four"]),
```
All four parse to `BuildingAnimKind::Special` with `is_primary = suffix.is_empty()`; there is **no slot-index field** — they are distinguished only by document order within `kind == Special`. So the consumer (Step 4) selects by that order: 0/1 = up pair (SpecialAnim/Two), 2/3 = down pair (Three/Four). The `…Damaged` keys are already parsed (`art_data.rs:1122`).

**Step 2 — event vec** (mirror `bale_events`, `world/mod.rs:344`):
```rust
    #[serde(skip)] pub bunker_wall_events: Vec<BunkerWallAnimEvent>,
```
with `struct BunkerWallAnimEvent { building_id: u64, up: bool, damaged: bool }` (in `components.rs` beside `BaleDepositEvent`).

**Step 3 — `emit_bunker_wall_anim` body** (in `bunker_link.rs`): push a `BunkerWallAnimEvent` with `damaged = health_ratio <= rules.general.condition_red` (ledger item 5 health gate). Up emits slots 10/11 intent; down emits 12/13 intent (the `up` bool selects the slot pair app-side).

**Step 4 — consumer** in `app_building_anim.rs` (mirror `consume_bale_events:420`): for each `bunker_wall_events`, resolve the building art entry, collect its `kind == BuildingAnimKind::Special` configs in document order, then pick indices `[0,1]` (up) or `[2,3]` (down), using each config's `damaged_variant` when `damaged`, and push `AnimOverlayState`s into `BuildingAnimOverlays`. On exit (down), clear the up overlays first (per the exit helper's anim-clear-then-set order, `0x004595C0`).

**Step 5 — drain** `bunker_wall_events` each frame (alongside `consume_bale_events`).

**Step 6 — verify:** `cargo test -p vera20k bunker` (sim-side: assert events emitted up-on-install, down-on-normal/clear, with correct `damaged` flag at low health) → PASS. App-side overlay creation is verified by running the game (Task 13).

**Step 7 — commit.**

---

### Task 12: Commands — `EnterBunker` + `EjectBunker`

**Why:** the playable entry + exit (design decision D1 "full playable").

**Files:** Modify `src/sim/command.rs`, `src/sim/world/world_commands.rs`.

**Step 1 — variants** in `command.rs` `Command` enum (mirror `EnterTransport`):
```rust
    /// Order a bunkerable vehicle into a friendly Bunker=yes building.
    EnterBunker { unit_id: u64, bunker_id: u64 },
    /// Eject the occupant of a friendly occupied bunker (targets the bunker —
    /// the hidden occupant is not selectable in 7b).
    EjectBunker { bunker_id: u64 },
```

**Step 2 — grep + patch every exhaustive `match Command`** (tests, replay, any dispatch) to add arms. `grep -rn "Command::" src/` for non-`_` matches.

**Step 3 — `EnterBunker` dispatch** (mirror `EnterTransport:845`): validate `command_owner` owns `unit_id`; not deployed; `bunker_id` is own `Bunker=yes`, not occupied/installing; run the admission handshake over the bus — `transmit(unit, bunker, Hello)` then `transmit(unit, bunker, CanEnter)`; on `Roger`: `transmit(unit, bunker, DockNow)`, set `unit.bunker_link = Approaching(bunker_id)`, `assign_mission_with_teardown(unit, Enter, DockTeardown::None)`, and issue an approach move to the bunker cell (reuse the `EnterTransport` move-issuance block). On `Negatory`: no-op (return false).

**Step 4 — `EjectBunker` dispatch:** validate owner owns `bunker_id` and it has an occupant; call `bunker_link::release_normal(self, bunker_id, rules)`.

**Step 5 — tests:** `enter_bunker_admits_and_starts_approach` (link = Approaching, mission Enter, move issued, `bunker_runtime` = ArriveWait); `enter_bunker_rejects_non_bunkerable`; `eject_bunker_releases_occupant` (release_normal fired); `eject_empty_bunker_noop`.

**Step 6 — verify:** `cargo test -p vera20k bunker` + `cargo test -p vera20k world_commands` → PASS.

**Step 7 — commit.**

---

### Task 13: Passability-gate-live test + global hash re-baseline + end-to-end

**Why:** the gate that was dead (`bunker_occupant` read-but-unset) is now live; confirm the full loop and re-baseline the lockstep hash once.

**Files:** Modify `src/sim/movement/` test module (or `pathfinding/cell_entry.rs` tests); `src/sim/world/global_parity_harness_tests.rs` (baseline).

**Step 1 — passability test:** spawn `NATBNK`, assert an empty bunker's footprint is row-exempt (not a blocker) and an occupied bunker (after `install_bunker_link`) blocks — exercising `movement_occupancy.rs:332` / `cell_entry.rs:345` with `bunker_occupant` now set.

**Step 2 — end-to-end integration test:** `EnterBunker` a tank → tick the machine to Occupied (unit concealed, occupant set, up sound, footprint blocks) → `EjectBunker` → unit revealed at a nearby passable cell, links cleared, down sound, footprint no longer blocks. One test, the full lifecycle.

**Step 3 — determinism check (no re-baseline):** run `cargo test -p vera20k --test determinism_replay`; all relative determinism tests must stay PASS. **Verified during planning: there is no absolute golden-hash constant in the suite** (the `GLOBAL_HARNESS_FINAL_HASH` / `global_skirmish_replay_is_deterministic_and_baseline_stable` names in the original plan do not exist) — every determinism test is run-twice-compare or live-vs-replay, so the new default-valued fields need no re-baseline. Confirm run-twice hashes equal and live timeline == replay timeline.

**Step 4 — full regression:** `cargo test -p vera20k` → read the literal `test result:` line; all PASS.

**Step 5 — commit.**

---

### Task 14: Verification against gamemd.exe

**Why:** confirm observable parity.

**Verify (in-game / `/fidelity-check`):**
- Order a tank onto own `NATBNK`: it drives in, turns, walls rise (`TankBunkerUp`), tank disappears inside. Infantry / `Bunkerable=no` / enemy bunker reject. (gamemd: same.)
- Eject: walls fall (`TankBunkerDown`), tank reappears on a nearby passable cell SW of the bunker and moves out. (gamemd `0x004595C0`: nearby-passable from NW+(-1,+1) + Move.)
- Sell an occupied bunker: tank reappears at the bunker cell, idle, **no** down sound. (gamemd `0x004593A0`: UndockUnit, no sound, no Move.) Destroy an occupied bunker: same teardown.
- Occupied bunker blocks pathing through its footprint; empty does not.
- **Known 7b gap (expected):** no tank rendered *inside* the walls and the bunkered tank does not fire — deferred to the combat/render slice (design D2/D3). The walls-up/down anim + sounds + lifecycle ARE present.

## Sources & References

- **Design doc:** `docs/plans/2026-06-02-tank-bunker-lifecycle-design.md`
- **Ghidra reports:** `TANK_BUNKER_INSTALL_MICROSTATES_GHIDRA_REPORT.md`, `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md`, `BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md`, `BUNKER_SERVICEDEPOT_0X2E4_RECIPROCAL_LINK_TEARDOWN_GHIDRA_REPORT.md`, `NATBNK_BUNKER_0X2E4_ROW_HELPER_PATH_GHIDRA_REPORT.md`, `CHRONO_MINER_FORCE_TRACK_0X47_REFINERY_EXIT_GHIDRA_REPORT.md`, `RADIO_INFERRED_CODES_AND_DOCK_HANDSHAKE_ORDER_GHIDRA_REPORT.md`.
- **gamemd.exe addresses:** install `0x00458E50` (caller `0x0044B780`); `CanAutoDeployHere 0x0070FB50`; exits `0x004595C0`/`0x004593A0`; clear `0x00459470`; FacingClass trio `0x004c9220/80/d0`; admission `Receive_Radio 0x0043C2D0` cases 0x0F/0x15.
- **INI:** `rulesmd.ini` `[NATBNK]`/`[AudioVisual] BunkerWallsUpSound/DownSound`/`[General] ConditionRed`; `artmd.ini` `[NATBNK]` SpecialAnim slots.
- **Related code:** `src/sim/radio/{mod,receive}.rs`, `src/sim/mission/{verb,retask}.rs`, `src/sim/docking/building_dock.rs`, `src/sim/miner/miner_dock_sequence.rs` (force-track + facing pivot pattern), `src/sim/production/production_sell.rs` (teardown hooks), `src/app_building_anim.rs` (`consume_bale_events`), `src/sim/movement/facing_class.rs`, `src/sim/movement/drive_track.rs`.
