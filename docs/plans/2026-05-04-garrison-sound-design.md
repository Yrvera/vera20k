# Garrison Sound & EVA Design

## Goal

Wire up the three player-audible cues that gamemd plays around garrison transitions: `EVA_StructureGarrisoned` (first occupant enters), `EVA_StructureAbandoned` (last occupant leaves), and the `BuildingGarrisonedSound` SFX from `[AudioVisual]`.

## Architecture Context

### Existing sound pipeline

The project already has a clean sim/app split for audio:

- **Sim layer** ([src/sim/world/mod.rs:87-114](../../src/sim/world/mod.rs#L87-L114)) emits `SimSoundEvent` enum values into `sim.sound_events: Vec<SimSoundEvent>`. Pure data, no audio dependency. The vec is `#[serde(skip)]` and drained per tick.
- **App layer** ([src/app_sim_tick.rs:282-364](../../src/app_sim_tick.rs#L282-L364)) drains the vec each tick, applies non-deterministic gates (local-player check, EVA registry lookup, screen-space conversion), and converts each entry to a `GameSoundEvent` for the audio backend.

The pattern for owner-bound EVA cues is already established with `SimSoundEvent::BuildingComplete { owner }` → `GameSoundEvent::BuildingReady { sound_id }`. Specifically:

1. Sim emits `SimSoundEvent::Xxx { owner }` whenever the trigger condition fires (no human-player gating in sim — that would desync replays).
2. App layer:
   - Skips if `owner` is not the local human player.
   - Looks up `eva_registry.get(event_name, faction)` for the per-faction sound ID, with a hardcoded fallback string.
   - Returns `GameSoundEvent::Xxx { sound_id }` for the audio backend.

For positional weapon-style SFX, `SimSoundEvent::WeaponFired { report_sound_id, rx, ry }` → `GameSoundEvent::WeaponFired { sound_id, screen_pos }` exists, doing the cell→screen conversion in the app layer.

### Existing garrison entry/exit hooks

[src/sim/passenger.rs:260-396](../../src/sim/passenger.rs#L260-L396) `tick_boarding` already:

- Detects successful boarding (after `cargo.board()` returns true).
- Performs immediate ownership transfer for neutral-civilian → garrisoning-player. Saves the original owner in `t.garrison_original_owner` for revert.

[src/sim/passenger.rs:400-587](../../src/sim/passenger.rs#L400-L587) `tick_unloading` already:

- Pops one passenger per tick.
- On the empty transition (`cargo_empty`), reverts `t.owner` to `t.garrison_original_owner` (or `Neutral` as fallback).

Both points are exactly where new sound events should fire. No new transition hooks needed.

### Existing INI parsing

[src/rules/ruleset.rs:580-615](../../src/rules/ruleset.rs#L580-L615) already reads from the `[AudioVisual]` section (for `ConditionYellow`/`ConditionRed`). Adding `BuildingGarrisonedSound` is one extra `get` call in the same block.

### gamemd reference (verified)

- `BuildingClass::AddGarrisonOccupant` (0x00522910) — fires EVA + SFX on first occupant. Per the audited `GARRISON_SYSTEM_GHIDRA_REPORT.md` §4 Step 5, gated on local human.
- `BuildingClass::CheckAutoSellOrCivilian` (0x00458200) — fires EVA_StructureAbandoned when occupants→0 and the building reverts ownership. Decompilation (verified via Ghidra MCP 2026-05-04) shows `IsHumanPlayer` check **before** `ChangeOwner`, so the EVA fires for the **pre-revert** owner.
- `BuildingGarrisonedSound` is in `[AudioVisual]` (rulesmd.ini:614 — confirmed live by xref from `RulesClass::ReadAudioVisual` at 0x00669bf6).

## Impact Analysis

**Files modified:**

| File | Change | Approx. lines |
|------|--------|--------------|
| [src/sim/world/mod.rs](../../src/sim/world/mod.rs) | 3 new `SimSoundEvent` variants | ~12 |
| [src/sim/passenger.rs](../../src/sim/passenger.rs) | Emit at first-board / last-unload | ~30 |
| [src/rules/ruleset.rs](../../src/rules/ruleset.rs) | Parse `BuildingGarrisonedSound` | ~5 |
| [src/audio/events.rs](../../src/audio/events.rs) | 3 new `GameSoundEvent` variants | ~8 |
| [src/app_sim_tick.rs](../../src/app_sim_tick.rs) | 3 new dispatch arms | ~50 |
| audio backend consumer | 3 new match arms (one-shot SFX) | ~6 |

**Dependencies into this change:** none — adding enum variants is non-breaking until consumed.

**Determinism:** sim emits unconditionally; local-human-player gate happens at app layer. The state hash in [src/sim/world/world_hash.rs](../../src/sim/world/world_hash.rs) does not include `sound_events` (already `#[serde(skip)]` and ephemeral per-tick). Adding variants does not change replay/lockstep behavior.

**Risk:** low. All patterns precedented by existing `BuildingComplete`/`UnitComplete` plus `WeaponFired`. No new abstractions, no refactors.

## Chosen Approach

Three discrete `SimSoundEvent` variants mirroring the existing `XxxComplete { owner }` pattern, three matching `GameSoundEvent` variants. INI in `[AudioVisual]` next to `ConditionYellow`/`ConditionRed`.

Picked over a generic `EvaEvent { event_name: &'static str }` wrapper (introduces a string-typed-event idiom not used elsewhere) and a single combined `GarrisonTransition { kind }` variant (mixes positional SFX and owner-bound EVA into one variant — obscures call sites). Both alternatives saved a few lines at the cost of architectural inconsistency.

## Design

### Components

#### `SimSoundEvent` (sim/world/mod.rs)

```rust
pub enum SimSoundEvent {
    // ... existing variants ...

    /// First occupant entered a CanBeOccupied building (cargo 0→1).
    /// Owner is the post-transfer building owner. App layer plays
    /// EVA_StructureGarrisoned if owner is local human.
    StructureGarrisoned { owner: InternedId },

    /// Last occupant left a garrisoned building (cargo 1→0).
    /// Owner is the **pre-revert** owner — matches gamemd's
    /// CheckAutoSellOrCivilian which fires EVA before ChangeOwner.
    /// App layer plays EVA_StructureAbandoned if owner is local human.
    StructureAbandoned { owner: InternedId },

    /// First-occupant SFX from rulesmd [AudioVisual] BuildingGarrisonedSound.
    /// Positional cue gated on owner == local human (matches gamemd
    /// IsHumanPlayer check in AddGarrisonOccupant).
    BuildingGarrisonedSfx { owner: InternedId, rx: u16, ry: u16 },
}
```

#### `GameSoundEvent` (audio/events.rs)

```rust
pub enum GameSoundEvent {
    // ... existing variants ...

    /// EVA cue: a friendly building was garrisoned.
    StructureGarrisoned { sound_id: String },

    /// EVA cue: a friendly garrison was abandoned.
    StructureAbandoned { sound_id: String },

    /// Positional SFX from [AudioVisual] BuildingGarrisonedSound.
    BuildingGarrisonedSfx { sound_id: String, screen_pos: Option<(f32, f32)> },
}
```

#### `GeneralRules` (rules/ruleset.rs)

```rust
pub struct GeneralRules {
    // ... existing fields ...

    /// [AudioVisual] BuildingGarrisonedSound — SFX played when first occupant
    /// enters a CanBeOccupied building. None = silent.
    pub building_garrisoned_sound: Option<String>,
}
```

### Interfaces / Contracts

**Sim emission contract:** sim **must** push events to `sim.sound_events` deterministically (same input → same events in same order). No conditional logic that depends on the local player or render state. Owner-gating, faction lookup, and screen-space conversion happen in the app layer only.

**App dispatch contract:** for every drained `SimSoundEvent::Xxx` variant the app must either produce exactly one `GameSoundEvent` or skip with `continue` (matches the existing pattern). No silent failures — if the event was emitted but cannot be played (no sound configured, registry empty), the dispatch logs and skips.

### Data Flow

```
First occupant enters:
    Command::EnterTransport → tick_boarding success
    → cargo.board() returns true, cargo.count() == 1
    → ownership transfer (existing code)
    → sim.sound_events.push(StructureGarrisoned { owner })
    → sim.sound_events.push(BuildingGarrisonedSfx { owner, rx, ry })
    → (next tick boundary)
    → app_sim_tick drains: local-player gate → EVA registry/INI lookup
    → state.sound_events.push(GameSoundEvent::StructureGarrisoned { sound_id })
    → state.sound_events.push(GameSoundEvent::BuildingGarrisonedSfx { sound_id, screen_pos })
    → audio backend plays one-shot SFX

Last occupant leaves:
    OrderIntent::Unloading → tick_unloading success on the final pop
    → cargo_empty = true
    → capture pre-revert owner
    → sim.sound_events.push(StructureAbandoned { owner: pre_revert_owner })
    → ownership reverts to garrison_original_owner / Neutral
    → app_sim_tick drains, dispatches as above
```

### Emission Sites (sim/passenger.rs)

**`tick_boarding`** — insert after the existing ownership-transfer block (~line 370), before `pax.passenger_role = PassengerRole::Inside`:

```rust
let first_occupant = sim
    .entities
    .get(transport_id)
    .and_then(|t| t.passenger_role.cargo())
    .map_or(false, |c| c.count() == 1);
if first_occupant
    && rules
        .object(&transport_type_str)
        .map_or(false, |o| o.can_be_occupied)
{
    if let Some(t) = sim.entities.get(transport_id) {
        sim.sound_events.push(SimSoundEvent::StructureGarrisoned { owner: t.owner });
        sim.sound_events.push(SimSoundEvent::BuildingGarrisonedSfx {
            owner: t.owner,
            rx: t.position.rx,
            ry: t.position.ry,
        });
    }
}
```

**`tick_unloading`** — replace the existing cargo-empty branch (~line 565) with:

```rust
if cargo_empty {
    let is_garrison_building = rules
        .object(&transport_type_str)
        .map(|obj| obj.can_be_occupied)
        .unwrap_or(false);
    let neutral_id = sim.interner.intern("Neutral");
    let abandoning_owner = if is_garrison_building {
        sim.entities.get(transport_id).map(|t| t.owner)
    } else {
        None
    };
    if let Some(t) = sim.entities.get_mut(transport_id) {
        t.order_intent = None;
        if is_garrison_building {
            let revert_owner = t.garrison_original_owner.take().unwrap_or(neutral_id);
            t.owner = revert_owner;
            ownership_changed = true;
        }
    }
    if let Some(owner) = abandoning_owner {
        sim.sound_events.push(SimSoundEvent::StructureAbandoned { owner });
    }
}
```

The owner is captured *before* the mut borrow on `t` so the event uses the pre-revert owner without overlapping borrows.

### App Dispatch (app_sim_tick.rs)

Three new match arms in the SimSoundEvent → GameSoundEvent loop, each following the `BuildingComplete` precedent:

```rust
SimSoundEvent::StructureGarrisoned { owner } => {
    let owner_str = sim.interner.resolve(owner);
    if !local_owner_name
        .as_deref()
        .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
    {
        continue;
    }
    let faction = crate::app_building_anim::eva_faction_key(
        owner_str,
        &state.house_roster,
    );
    let sound_id = state
        .eva_registry
        .get("EVA_StructureGarrisoned", faction)
        .unwrap_or("/* fallback resolved at impl time from eva.ini */")
        .to_string();
    GameSoundEvent::StructureGarrisoned { sound_id }
}

SimSoundEvent::StructureAbandoned { owner } => {
    /* same shape, "EVA_StructureAbandoned" */
}

SimSoundEvent::BuildingGarrisonedSfx { owner, rx, ry } => {
    let owner_str = sim.interner.resolve(owner);
    if !local_owner_name
        .as_deref()
        .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
    {
        continue;
    }
    let sound_id = match rules.general.building_garrisoned_sound.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => continue,
    };
    let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
    GameSoundEvent::BuildingGarrisonedSfx {
        sound_id,
        screen_pos: Some((sx, sy)),
    }
}
```

Fallback string IDs for the two EVA cues are resolved at implementation time by reading the `[EVA_StructureGarrisoned]` and `[EVA_StructureAbandoned]` sections in `ini/evamd.ini` and picking the per-faction default.

### Error Handling

- Sim layer: emission is best-effort and infallible. If `entities.get(transport_id)` returns None mid-flow (entity destroyed), simply skip emission — no panic.
- App layer: each new arm uses `continue` for any failure mode (owner not local, no sound configured, registry lookup miss with no fallback). Matches the existing pattern. No silent panics.
- Audio backend: existing pipeline already handles missing sound IDs via warn-log + skip.

### Testing Strategy

**Unit tests (`#[cfg(test)] mod tests` in passenger.rs):**

1. `test_first_occupant_emits_garrisoned_event` — set up CanBeOccupied building (cargo capacity 5) + adjacent Occupier infantry in `BoardingPhase::Entering`, run `tick_boarding`, assert `sim.sound_events` contains exactly one `StructureGarrisoned { owner }` and one `BuildingGarrisonedSfx { owner, rx, ry }` for the building's owner and position.
2. `test_second_occupant_emits_no_event` — pre-populate cargo with one occupant, board a second, assert no `StructureGarrisoned` or `BuildingGarrisonedSfx` is emitted.
3. `test_last_occupant_emits_abandoned_event_with_pre_revert_owner` — set up a 1-occupant garrison owned by a non-Neutral house with `garrison_original_owner = Some(neutral_id)`, set `OrderIntent::Unloading`, tick, assert one `StructureAbandoned { owner }` for the **pre-revert** non-Neutral owner.
4. `test_non_garrison_transport_emits_no_garrison_events` — board into a `Passengers=5` IFV (not `CanBeOccupied`), assert no garrison events.

**INI parsing tests (rules/ruleset.rs):**

5. `test_building_garrisoned_sound_parsed` — INI string with `[AudioVisual] BuildingGarrisonedSound=BuildingGarrisoned`, parse, assert `general.building_garrisoned_sound == Some("BuildingGarrisoned".to_string())`.
6. `test_building_garrisoned_sound_default_none` — INI without the key, assert `None`.

**Integration / determinism:** existing replay tests cover state-hash stability. Sound events are `#[serde(skip)]` so adding variants cannot affect lockstep.

**No app-layer tests** — local-player gating + registry lookup are covered by the existing `BuildingComplete` precedent. If those work, the new arms work.

## Architectural Decisions

**Patterns followed:**

- Owner-bound EVA via `SimSoundEvent { owner }` → app-side local-player gate → faction-keyed EVA registry lookup. Mirrors `BuildingComplete`/`UnitComplete`.
- Positional SFX via `SimSoundEvent { rx, ry }` → app-side iso→screen conversion. Mirrors `WeaponFired`/`EntityDied` (with the addition of `owner` for the IsHumanPlayer gate).
- INI keys grouped by source `.ini` section in the parser (this one goes next to `ConditionRed`/`ConditionYellow` because they all live in `[AudioVisual]`).
- App-layer non-determinism (local-player check, registry lookup, screen-space conversion); sim-layer pure emission.

**Patterns deliberately not introduced:**

- No generic `EvaEvent { event_name: String }` variant. The codebase uses specific variants per cue and we don't want to fork the convention.
- No refactor of `WeaponFired` into a generic `PositionalSfx`. Out of scope for this brainstorm; if multiple `[AudioVisual]` SFX get wired up later (UpgradeVeteranSound, BuildingRepairedSound, BaseUnderAttackSound), refactor then.
- No event when an occupant dies inside a destroyed building. Garrison destruction is currently silent because all occupants die — that's tracked as G1 in the disparity scan and will need a re-evaluation when ejection-on-destruction lands.

**Tech debt introduced:** none material. Three small new variants in two enums; one INI field. No new patterns, no shims, no fallback layers.

## Alternatives Considered

**Approach 2 — generic `EvaEvent { event_name }`.** Rejected: introduces a string-typed event idiom that the codebase does not currently use. Would force every future EVA cue to choose between styles, or trigger a follow-up convergence refactor.

**Approach 3 — single `GarrisonTransition { kind: Entered|Abandoned }`.** Rejected: mixes positional SFX (`BuildingGarrisonedSfx`) and owner-only EVA (`StructureGarrisoned`/`Abandoned`) into one variant, obscuring at the call site whether the event will produce SFX, EVA, or both. Saves a few lines but reduces clarity.

**Reusing `WeaponFired` for the SFX (option A.3.ii in the brainstorm).** Rejected: routing a non-weapon sound through a variant called `WeaponFired` is the kind of small naming drift that's easy to add and hard to clean up later.

**Generalising `WeaponFired` → `PositionalSfx` now (option A.3.iii).** Rejected for this design: would touch every existing `WeaponFired` call site for no functional gain at this point. Worth doing as a separate small refactor when there are 3+ positional SFX consumers.

## Out of Scope

- **Eject occupants on destruction** (G1 in the garrison disparity scan). Separate brainstorm.
- **Assaulter path** (G4). Confirmed TS-legacy ghost — no unit in vanilla YR or known mods has `Assaulter=yes`. Marked deferred indefinitely.
- **Veterancy gain on garrison kills** (G3). Blocked on the system-wide veterancy gain implementation. Note that when that lands, kill credit needs to route to the round-robin occupant per `GARRISON_SYSTEM_GHIDRA_REPORT.md` §15d.
- **Generalising the `[AudioVisual]` SFX dispatch** to cover other `[AudioVisual]` sounds (UpgradeVeteranSound, BuildingRepairedSound, BaseUnderAttackSound). Wait until there are 3+ consumers, then refactor.
