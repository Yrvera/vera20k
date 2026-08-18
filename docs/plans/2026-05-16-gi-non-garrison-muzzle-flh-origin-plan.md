# GI Non-Garrison Muzzle FLH Origin Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Spawn non-garrison weapon muzzle `Anim=` effects and place `Report=` sound at the documented FLH fire origin when a GI shot actually fires, without changing combat damage timing or garrison muzzle behavior.

**Architecture:** Combat remains authoritative for the fire moment and emits deterministic facts through `SimFireEvent`. App/render/audio resolve art metadata, FLH screen offsets, SHP sprites, and sound screen positions above the sim boundary.

**Design Doc:** `docs/plans/2026-05-16-gi-non-garrison-muzzle-flh-origin-design.md`

---

## Grounding Summary

`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` documents the Fire_At muzzle path: weapon `Anim=` is selected after the shot, `Report=` is played at `muzzleCoords`, and `AnimClass` is constructed at the same coordinates with draw flags `0x600`. Live Ghidra verification of `TechnoClass::Fire_At` at `0x006FDD50` confirmed the same ordering: update burst index, select weapon anim, override with `OccupantAnim` for garrison, play `VocClass__PlayAt` using the muzzle coordinate local, construct `AnimClass`, and attach it to non-building owners.

`ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md` and live Ghidra verification of `TechnoClass::GetFLH` at `0x006F3AD0` confirm FLH selection and rotation: normal weapon slots read FLH from the resolved weapon slot, negative indexes read elite FLH slots, FLH position uses 32-way facing quantization, and `CurrentBurstIndex` can flip the lateral component. GI's retail FLH lateral value is zero, so burst lateral flip is not visible for the GI validation cases.

`GI_GHIDRA_REPORT.md` confirms deployed GI uses `DeployedFire` for visual timing while weapon choice remains target-driven. The previous infantry fire-frame sync commit `1391629` and scan `docs/gap-scans/2026-05-16-disparity-scan-gi-infantry-fire-sync.md` closed standing/prone/deployed damage timing.

Current Rust parses all needed INI data: weapon `Anim=` and `Report=` in `src/rules/weapon_type.rs`, and FLH fields in `src/rules/art_data.rs`. Current Rust does not consume weapon `Anim=` for non-garrison fire; `src/app_building_anim.rs` only spawns garrison `OccupantAnim` muzzle flashes.

Repo pattern to mirror: transient sim events are drained by `src/app_sim_tick.rs`; app-owned one-shot visuals use structs in `src/sim/components.rs`, tick in app code, and render through `src/app_instances/overlays.rs`; effect SHPs are collected in `src/render/sprite_atlas.rs`.

INI keys driving this feature:

- `ini/artmd.ini [GI] PrimaryFireFLH=80,0,105`
- `ini/artmd.ini [GI] SecondaryFireFLH=80,0,90`
- `ini/rulesmd.ini [M60] Report=GIAttack`
- `ini/rulesmd.ini [M60] Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW`
- `ini/rulesmd.ini [M60] OccupantAnim=UCFLASH`
- `ini/rulesmd.ini [Para] Report=GIAttackDeployed`
- `ini/rulesmd.ini [Para] Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW`
- `ini/rulesmd.ini [Para] OccupantAnim=UCFLASH`

Remaining validation after grounding: the plan now adds a 32-way FLH transform to match the binary quantization, but final pixel equivalence still needs live visual comparison against YR.

## Key Technical Decisions

- **Add `weapon_id` to `SelectedWeapon` and `SimFireEvent`:** Render must not infer the selected weapon later from current object state. **Confidence:** high. **Source:** design doc, current `select_weapon_with_ifv`, Fire_At decompile.
- **Move non-garrison `Report=` cue onto `SimFireEvent`:** The same app resolver can place both sound and muzzle visual at FLH origin. Garrison keeps the existing `SimSoundEvent::WeaponFired` cell-position path for this pass. **Confidence:** high. **Source:** Fire_At decompile and design approval.
- **Keep FLH-to-screen math out of `sim/`:** `sim/` emits facing/veterancy/slot/weapon facts only; app resolves pixels. **Confidence:** high. **Source:** `AGENTS.md` and current event architecture.
- **Create a dedicated app-owned non-garrison muzzle flash queue:** This mirrors `GarrisonMuzzleFlash` but uses a fixed spawn screen position from the fire tick. **Confidence:** medium. **Source:** current garrison pattern; docs say non-building anims attach to owner, but GI validation is stationary during fire.
- **Implement weapon `Anim=` selection as a pure helper:** Empty list produces no visual, count 8 uses the documented directional formula, all other positive counts select index 0. **Confidence:** high. **Source:** Fire_At decompile and `ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`.

## Open Questions

### Resolved During Planning

- **Should report sound origin be included now?** Yes. User approved including it, and Fire_At plays `Report=` at the same `muzzleCoords` used by the muzzle anim.
- **Does this add persistent sim state or require state hash changes?** No. `SimFireEvent` is transient and drained each tick.
- **Should garrison `OccupantAnim` change in this pass?** No. Garrison fire ports remain on the existing path; this plan only prevents regressions there.

### Deferred to Implementation

- **Exact pixel equivalence for FLH transform:** Unit tests can lock the Rust transform behavior, but final parity needs live visual comparison against YR for standing/prone/deployed GI.
- **Owner-attached non-infantry muzzle flashes:** Docs say non-building muzzle anims attach to the firing object. GI validation is stationary at the fire frame, so fixed spawn-position flashes are acceptable for this GI-focused pass. Vehicle/aircraft movement during muzzle flash lifetime should be scanned later.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/combat/combat_weapon.rs` | Carry selected weapon section id through weapon selection. |
| Modify | `src/sim/world/mod.rs` | Extend `SimFireEvent` with firing-tick facts and optional non-garrison report sound id. |
| Modify | `src/sim/combat/mod.rs` | Populate extended fire events and route non-garrison report sounds through fire events. |
| Modify | `src/sim/components.rs` | Add app-owned active non-garrison muzzle flash data. |
| Modify | `src/app.rs` | Store active non-garrison muzzle flashes on `AppState`. |
| Create | `src/app_fire_effects.rs` | Resolve non-garrison fire events into muzzle flashes and report sounds; tick active flashes. |
| Modify | `src/app_sim_tick.rs` | Call the non-garrison fire resolver after sim fire events are drained. |
| Modify | `src/app_instances/overlays.rs` | Build sprite instances for active non-garrison muzzle flashes. |
| Modify | `src/app_render/build_instances.rs` | Add non-garrison muzzle flash instance build call. |
| Modify | `src/render/sprite_atlas.rs` | Load weapon `Anim=` SHPs into the effect atlas. |
| Modify | Tests near touched modules | Add combat/event, helper, atlas, and app resolver regressions. |

## Interface Changes

- `SelectedWeapon<'a>` gains `weapon_id: &'a str`.
- `SimFireEvent` gains `attacker_type_ref: InternedId`, `weapon_id: InternedId`, `facing: u8`, `veterancy: u16`, and `report_sound_id: Option<InternedId>`.
- `AppState` gains `weapon_muzzle_flashes: Vec<WeaponMuzzleFlash>`.
- New app helper API:

```rust
pub(crate) fn tick_weapon_muzzle_flashes(state: &mut AppState, dt_ms: u32);
pub(crate) fn spawn_non_garrison_fire_effects(state: &mut AppState, events: &[SimFireEvent]);
pub(crate) fn build_weapon_muzzle_flash_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
);
```

## Sim Checklist

- [x] All new sim fields are integers or interned ids; no f32/f64 in gameplay logic.
- [x] New fire event data is transient; no state hash update is required.
- [x] `sim/` receives no dependency on render/ui/sidebar/audio/net.
- [x] Tick ordering is preserved: event is emitted on the same combat fire tick.
- [x] EntityStore iteration order is not changed.

## Risk Areas

- **Weapon identity drift:** Using only `WeaponSlot` would fail for IFV/garrison/override paths. The plan carries `weapon_id`.
- **Borrowing in `app_sim_tick`:** Drained fire events must be collected into a local `Vec<SimFireEvent>` so app helpers can consume them without fighting the mutable sim borrow.
- **Sound duplication:** Non-garrison report sounds must not be emitted through both `SimSoundEvent::WeaponFired` and `SimFireEvent::report_sound_id`.
- **Garrison regression:** Garrison continues using `occupant_anim` and current sound path in this pass.
- **Effect atlas misses:** Loading only `OccupantAnim` keeps non-garrison flashes invisible; load all weapon `Anim=` entries.
- **Facing formula:** The 8-way helper must be centralized and tested.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| 2 | Fire event emitted only on the actual fire tick | GI muzzle flash must occur on `FireUp`, `FireProne`, or `DeployedFire` discharge frame | Combat tests inspect fire events after existing delayed-fire tests |
| 3 | Event carries selected weapon id | Deployed GI visual timing must not overwrite target-driven weapon choice | Deployed GI secondary test checks `weapon_id == Para` |
| 5 | 8-way `Anim=` selection | Wrong directional MGUN sprite is visible on every shot while facing changes | Unit tests for 8 facings and live visual check |
| 6 | 32-way FLH transform | Muzzle origin must use gamemd's quantized fire-origin rotation, not continuous render rotation | Unit tests compare quantized facing buckets |
| 7 | FLH primary/secondary origin | GI standing/prone and deployed muzzle positions differ by `105` vs `90` height | Resolver tests with `[GI]` art values |
| 8 | Report sound uses FLH origin | Sound should spatialize from muzzle, not unit cell origin | App resolver test checks screen pos differs from cell origin when FLH nonzero |
| 12 | Garrison remains separate | Garrison fire must keep `UCFLASH` and building fire ports | Regression test on garrison event path |
| 11 | Atlas loads weapon `Anim=` SHPs | Without this, event spawning succeeds but nothing renders | Atlas collection test includes `MGUN-N` and `MGUN-NW` |

---

## Tasks

### Task 1: Carry Selected Weapon Id Through Weapon Selection

**Why:** Later app/render code must know the exact weapon that fired without re-running combat selection.

**Files:**
- Modify: `src/sim/combat/combat_weapon.rs`

**Pattern:** Existing `SelectedWeapon` return type; add one borrowed field.

**Step 1: Extend `SelectedWeapon`**

```rust
pub(crate) struct SelectedWeapon<'a> {
    pub weapon_id: &'a str,
    pub weapon: &'a WeaponType,
    pub warhead: &'a WarheadType,
    pub verses_pct: u8,
    pub slot: WeaponSlot,
}
```

**Step 2: Populate `weapon_id` in `try_weapon`**

```rust
Some(SelectedWeapon {
    weapon_id,
    weapon,
    warhead,
    verses_pct,
    slot,
})
```

**Step 3: Add/adjust unit tests**

Add assertions to existing weapon selection tests:

```rust
let selected = select_weapon_with_ifv(rules, attacker, target_category, target_armor, None)
    .expect("weapon selected");
assert_eq!(selected.weapon_id, "M60");
```

For secondary fallback cases, assert the secondary id. For IFV override tests, assert the weapon list id.

**Step 4: Verify**

Run: `cargo test combat_weapon --lib`

Expected: all combat weapon tests pass.

**Step 5: Commit**

Commit message: `Track selected weapon id in combat selection`.

### Task 2: Extend `SimFireEvent` And Update The Combat Producer

**Why:** The fire event is the contract between deterministic combat and presentation. The struct and its producer must change together so the checkpoint compiles.

**Files:**
- Modify: `src/sim/world/mod.rs`
- Modify: `src/sim/combat/combat_targeting.rs`
- Modify: `src/sim/combat/mod.rs`

**Pattern:** Existing transient `SimFireEvent` near `SimSoundEvent`.

**Step 1: Extend the struct**

```rust
#[derive(Debug, Clone)]
pub struct SimFireEvent {
    pub attacker_id: u64,
    pub attacker_type_ref: InternedId,
    pub weapon_slot: WeaponSlot,
    pub weapon_id: InternedId,
    pub facing: u8,
    pub veterancy: u16,
    pub target: crate::sim::combat::TargetKind,
    pub report_sound_id: Option<InternedId>,
    pub garrison_muzzle_index: Option<u8>,
    pub occupant_anim: Option<InternedId>,
}
```

Keep the doc comment explicit: non-garrison fields are a firing-tick snapshot; garrison fields are still fire-port/occupant-specific.

**Step 2: Add firing facts to attacker snapshots**

`AttackerSnapshot` already carries `type_id`. Add `facing: u8` and `veterancy: u16` to `src/sim/combat/combat_targeting.rs`, populate them from `entity.facing` and `entity.veterancy` in the snapshot build tuple, and destructure them into `snap.facing` and `snap.veterancy`.

**Step 3: Replace non-garrison report sound emission**

Current shape:

```rust
if let Some(ref report_id) = weapon.report {
    fire_sounds.push((interner.intern(report_id), snap.pos_rx, snap.pos_ry));
}
```

Change it so garrison keeps existing `fire_sounds`, while non-garrison carries the report on `SimFireEvent`:

```rust
let report_sound_id = weapon.report.as_ref().map(|report_id| interner.intern(report_id));
if is_garrison {
    if let Some(report_id) = report_sound_id {
        fire_sounds.push((report_id, snap.pos_rx, snap.pos_ry));
    }
}
```

**Step 4: Populate the extended event**

```rust
fire_events.push(SimFireEvent {
    attacker_id: snap.stable_id,
    attacker_type_ref: snap.type_id,
    weapon_slot: selected.slot,
    weapon_id: interner.intern(selected.weapon_id),
    facing: snap.facing,
    veterancy: snap.veterancy,
    target: snap.target,
    report_sound_id: if is_garrison { None } else { report_sound_id },
    garrison_muzzle_index: snap.garrison.as_ref().map(|gs| gs.fire_index),
    occupant_anim: if is_garrison {
        weapon.occupant_anim.as_ref().map(|s| interner.intern(s))
    } else {
        None
    },
});
```

Use `rg -n "SimFireEvent \\{" src` to verify no other struct literals need the new fields.

**Step 5: Verify**

Run: `cargo check --lib`

Expected: the library compiles. This checkpoint must not leave missing `SimFireEvent` fields for a later task.

**Step 6: Commit**

Commit message: `Extend combat fire events with firing facts`.

### Task 3: Add Combat Event Regression Tests

**Why:** Combat owns the exact fire moment and selected weapon; tests lock the new event contract before app/render consumes it.

**Files:**
- Modify: `src/sim/combat/combat_tests.rs`

**Pattern:** Existing GI fire-frame sync tests.

**Step 1: Add combat event tests**

In `src/sim/combat/combat_tests.rs`, extend existing GI delayed-fire tests:

```rust
assert_eq!(sim.fire_events.len(), 1);
let ev = &sim.fire_events[0];
assert_eq!(sim.interner.resolve(ev.weapon_id), "M60");
assert_eq!(ev.weapon_slot, WeaponSlot::Primary);
assert_eq!(ev.report_sound_id.map(|id| sim.interner.resolve(id)), Some("GIAttack"));
assert_eq!(ev.garrison_muzzle_index, None);
assert_eq!(ev.occupant_anim, None);
```

For deployed GI versus heavy armor, assert:

```rust
assert_eq!(sim.interner.resolve(ev.weapon_id), "Para");
assert_eq!(ev.weapon_slot, WeaponSlot::Secondary);
assert_eq!(ev.report_sound_id.map(|id| sim.interner.resolve(id)), Some("GIAttackDeployed"));
```

**Step 2: Verify**

Run: `cargo test combat --lib`

Expected: combat tests pass and non-garrison report sounds are no longer duplicated in `sim.sound_events`.

**Step 3: Commit**

Commit message: `Test combat fire event facts`.

### Task 4: Add Active Non-Garrison Muzzle Flash State

**Why:** Non-garrison muzzle flashes need app-owned one-shot runtime state parallel to garrison flashes.

**Files:**
- Modify: `src/sim/components.rs`
- Modify: `src/app.rs`

**Pattern:** `GarrisonMuzzleFlash` and `AppState.garrison_muzzle_flashes`.

**Step 1: Define `WeaponMuzzleFlash`**

Add after `GarrisonMuzzleFlash`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeaponMuzzleFlash {
    pub attacker_id: u64,
    pub shp_name: String,
    pub screen_x: f32,
    pub screen_y: f32,
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
    pub frame: u16,
    pub total_frames: u16,
    pub rate_ms: u32,
    pub elapsed_ms: u32,
}
```

This struct is app-owned presentation state despite living in `sim/components.rs`, matching the existing `GarrisonMuzzleFlash` placement.

**Step 2: Add `AppState` field**

Add near `garrison_muzzle_flashes`:

```rust
pub(crate) weapon_muzzle_flashes: Vec<crate::sim::components::WeaponMuzzleFlash>,
```

Initialize it with `Vec::new()` in `AppState::default`.

**Step 3: Verify**

Run: `cargo check --lib`

Expected: the library still compiles. This task only introduces app-owned state and must not depend on later wiring to type-check.

**Step 4: Commit**

Commit message: `Add non-garrison muzzle flash state`.

### Task 5: Add Weapon Muzzle Anim Selection Helper

**Why:** Directional `Anim=` selection is a pure rule and should be tested before render wiring.

**Files:**
- Create: `src/app_fire_effects.rs`

**Pattern:** App helper modules such as `app_chute_anim.rs` and `app_building_anim.rs`.

**Step 1: Create module doc comment and imports**

```rust
//! App-owned weapon fire presentation: non-garrison muzzle flashes and
//! FLH-positioned weapon report sounds.
//!
//! The sim emits deterministic fire facts. This module resolves rules/art
//! metadata into screen-space visuals and audio cues above the sim boundary.
```

**Step 2: Add helper**

```rust
pub(crate) fn select_weapon_muzzle_anim<'a>(anims: &'a [String], facing: u8) -> Option<&'a str> {
    match anims.len() {
        0 => None,
        8 => {
            let idx = ((((facing as u16) << 8) >> 12) + 1) >> 1;
            let idx = ((idx & 7) + 1) & 7;
            anims.get(idx as usize).map(String::as_str)
        }
        _ => anims.first().map(String::as_str),
    }
}
```

This implements the decompiled `((*dir >> 12) + 1 >> 1 & 7) + 1 & 7` formula after converting Rust's 8-bit facing into the 16-bit facing space used in the decompile.

**Step 3: Add tests**

```rust
#[test]
fn selects_none_for_empty_weapon_anim_list() {
    assert_eq!(select_weapon_muzzle_anim(&[], 0), None);
}

#[test]
fn selects_first_for_non_directional_list() {
    let anims = vec!["GUNFIRE".to_string(), "ALT".to_string()];
    assert_eq!(select_weapon_muzzle_anim(&anims, 64), Some("GUNFIRE"));
}

#[test]
fn selects_documented_8way_indices() {
    let anims = vec![
        "N".to_string(), "NE".to_string(), "E".to_string(), "SE".to_string(),
        "S".to_string(), "SW".to_string(), "W".to_string(), "NW".to_string(),
    ];
    assert_eq!(select_weapon_muzzle_anim(&anims, 0), Some("NE"));
    assert_eq!(select_weapon_muzzle_anim(&anims, 32), Some("E"));
    assert_eq!(select_weapon_muzzle_anim(&anims, 64), Some("SE"));
    assert_eq!(select_weapon_muzzle_anim(&anims, 128), Some("SW"));
    assert_eq!(select_weapon_muzzle_anim(&anims, 192), Some("NW"));
}
```

These expected values follow the decompiled formula directly. If visual validation later proves Rust facing semantic labels are rotated relative to gamemd labels, update this centralized helper and its tests together.

**Step 4: Expose module**

Add `pub mod app_fire_effects;` in `src/lib.rs`, next to the other app sibling modules such as `app_sim_tick`, `app_building_anim`, and `app_chute_anim`. Do not register it as a private nested module inside `src/app.rs`; callers use `crate::app_fire_effects`.

**Step 5: Verify**

Run: `cargo test app_fire_effects --lib`

Expected: helper tests pass.

**Step 6: Commit**

Commit message: `Add weapon muzzle anim selector`.

### Task 6: Add Gamemd 32-Way FLH Screen Transform

**Why:** `TechnoClass::GetFLH` quantizes facing to 32 directions before rotating FLH. A continuous render-angle transform can place muzzle flashes and report sounds between gamemd's fire-origin buckets.

**Files:**
- Modify: `src/util/flh_transform.rs`

**Pattern:** Existing `flh_to_screen_offset` helper and tests in the same file.

**Step 1: Add quantized helper**

Keep the existing helper available for any current callers, and add a gamemd-specific helper:

```rust
pub fn flh_to_screen_offset_32way(
    forward: i32,
    lateral: i32,
    height: i32,
    facing: u8,
) -> (f32, f32) {
    let facing_16: u16 = (facing as u16) << 8;
    let bucket: i16 = ((((facing_16 >> 10) + 1) >> 1) & 0x1f) as i16 - 8;
    let quantized_facing: u8 = (((bucket + 8) as u16 * 8) & 0xff) as u8;
    flh_to_screen_offset(forward, lateral, height, quantized_facing)
}
```

The bucket expression mirrors the verified binary shape. If the existing helper's zero-facing axis convention needs an offset to match current Rust facing semantics, apply that offset inside this helper and keep it covered by tests.

**Step 2: Add tests**

```rust
#[test]
fn flh_32way_quantizes_small_facing_changes_to_same_offset() {
    let a = flh_to_screen_offset_32way(80, 0, 105, 0);
    let b = flh_to_screen_offset_32way(80, 0, 105, 3);
    assert_eq!(a, b);
}

#[test]
fn flh_32way_changes_after_bucket_boundary() {
    let a = flh_to_screen_offset_32way(80, 0, 105, 0);
    let b = flh_to_screen_offset_32way(80, 0, 105, 8);
    assert_ne!(a, b);
}

#[test]
fn flh_32way_preserves_zero_flh() {
    assert_eq!(flh_to_screen_offset_32way(0, 0, 0, 123), (0.0, 0.0));
}
```

**Step 3: Verify**

Run: `cargo test flh_transform --lib`

Expected: FLH transform tests pass.

**Step 4: Commit**

Commit message: `Add quantized FLH screen transform`.

### Task 7: Resolve Non-Garrison FLH Fire Origin

**Why:** Both muzzle visual and report sound need one authoritative app-layer origin resolver.

**Files:**
- Modify: `src/app_fire_effects.rs`

**Pattern:** Rules image fallback from `tick_garrison_muzzle_flashes`; FLH resolver from `src/rules/flh.rs`.

**Step 1: Add resolved origin struct**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FireOrigin {
    pub screen_x: f32,
    pub screen_y: f32,
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
}
```

Do not derive `Eq` because screen coordinates are floats in app/render.

**Step 2: Add resolver**

```rust
pub(crate) fn resolve_non_garrison_fire_origin(
    state: &AppState,
    ev: &SimFireEvent,
) -> Option<FireOrigin> {
    if ev.garrison_muzzle_index.is_some() {
        return None;
    }
    let sim = state.simulation.as_ref()?;
    let rules = state.rules.as_ref()?;
    let art_reg = state.art_registry.as_ref()?;
    let entity = sim.entities.get(ev.attacker_id)?;
    let etype_str = sim.interner.resolve(ev.attacker_type_ref);
    let rules_image = rules
        .object(etype_str)
        .map(|o| o.image.clone())
        .unwrap_or_else(|| etype_str.to_string());
    let art = art_reg.resolve_metadata_entry(etype_str, &rules_image)?;
    let flh = crate::rules::flh::resolve_flh(
        art.primary_fire_flh,
        art.secondary_fire_flh,
        art.elite_primary_fire_flh,
        art.elite_secondary_fire_flh,
        matches!(ev.weapon_slot, WeaponSlot::Primary),
        ev.veterancy,
    );
    let (dx, dy) = crate::util::flh_transform::flh_to_screen_offset_32way(
        flh.forward,
        flh.lateral,
        flh.height,
        ev.facing,
    );
    Some(FireOrigin {
        screen_x: entity.position.screen_x + dx,
        screen_y: entity.position.screen_y + dy,
        rx: entity.position.rx,
        ry: entity.position.ry,
        z: entity.position.z,
    })
}
```

**Step 3: Add tests**

Add a smaller pure helper that takes `Position`, `ArtEntry`, `WeaponSlot`, `veterancy`, and `facing`, and have `resolve_non_garrison_fire_origin` call it after app lookups:

```rust
pub(crate) fn resolve_fire_origin_from_art(
    position: &Position,
    art: &ArtEntry,
    slot: WeaponSlot,
    veterancy: u16,
    facing: u8,
) -> FireOrigin {
    let flh = crate::rules::flh::resolve_flh(
        art.primary_fire_flh,
        art.secondary_fire_flh,
        art.elite_primary_fire_flh,
        art.elite_secondary_fire_flh,
        matches!(slot, WeaponSlot::Primary),
        veterancy,
    );
    let (dx, dy) = crate::util::flh_transform::flh_to_screen_offset_32way(
        flh.forward,
        flh.lateral,
        flh.height,
        facing,
    );
    FireOrigin {
        screen_x: position.screen_x + dx,
        screen_y: position.screen_y + dy,
        rx: position.rx,
        ry: position.ry,
        z: position.z,
    }
}
```

Unit-test this helper:

```rust
let primary = resolve_fire_origin_from_art(&position, &art, WeaponSlot::Primary, 0, 0);
let secondary = resolve_fire_origin_from_art(&position, &art, WeaponSlot::Secondary, 0, 0);
assert_ne!(primary.screen_y, secondary.screen_y);
```

**Step 4: Verify**

Run:

```powershell
cargo test flh_transform --lib
cargo test app_fire_effects --lib
```

Expected: origin resolver tests pass.

**Step 5: Commit**

Commit message: `Resolve FLH fire origins for app effects`.

### Task 8: Spawn and Tick Non-Garrison Muzzle Flashes and Report Sounds

**Why:** Convert drained fire events into visible muzzle flashes and FLH-positioned sound events.

**Files:**
- Modify: `src/app_fire_effects.rs`

**Pattern:** `tick_garrison_muzzle_flashes` phase split: spawn from pending events, then advance active entries.

**Step 1: Add spawn function**

```rust
pub(crate) fn spawn_non_garrison_fire_effects(state: &mut AppState, events: &[SimFireEvent]) {
    let Some(sim) = state.simulation.as_ref() else { return; };
    let Some(rules) = state.rules.as_ref() else { return; };

    let mut spawned = Vec::new();
    let mut sounds = Vec::new();

    for ev in events {
        if ev.garrison_muzzle_index.is_some() {
            continue;
        }
        let Some(origin) = resolve_non_garrison_fire_origin(state, ev) else { continue; };
        if let Some(report_id) = ev.report_sound_id {
            sounds.push(GameSoundEvent::WeaponFired {
                sound_id: sim.interner.resolve(report_id).to_string(),
                screen_pos: Some((origin.screen_x, origin.screen_y)),
            });
        }
        let Some(weapon) = rules.weapon(sim.interner.resolve(ev.weapon_id)) else { continue; };
        let Some(anim_name) = select_weapon_muzzle_anim(&weapon.anim, ev.facing) else { continue; };
        let total_frames = sim
            .interner
            .get(anim_name)
            .and_then(|anim_id| sim.effect_frame_counts.get(&anim_id).copied())
            .unwrap_or(1);
        spawned.push(WeaponMuzzleFlash {
            attacker_id: ev.attacker_id,
            shp_name: anim_name.to_string(),
            screen_x: origin.screen_x,
            screen_y: origin.screen_y,
            rx: origin.rx,
            ry: origin.ry,
            z: origin.z,
            frame: 0,
            total_frames,
            rate_ms: 67,
            elapsed_ms: 0,
        });
    }

    state.weapon_muzzle_flashes.extend(spawned);
    for sound in sounds {
        state.sound_events.push(sound);
    }
}
```

**Step 2: Add tick function**

```rust
pub(crate) fn tick_weapon_muzzle_flashes(state: &mut AppState, dt_ms: u32) {
    state.weapon_muzzle_flashes.retain_mut(|flash| {
        flash.elapsed_ms += dt_ms;
        while flash.elapsed_ms >= flash.rate_ms && flash.rate_ms > 0 {
            flash.elapsed_ms -= flash.rate_ms;
            flash.frame += 1;
        }
        flash.frame < flash.total_frames
    });
}
```

**Step 3: Add tests**

Unit-test tick removal:

```rust
state.weapon_muzzle_flashes.push(WeaponMuzzleFlash {
    attacker_id: 1,
    shp_name: "MGUN-N".to_string(),
    screen_x: 100.0,
    screen_y: 200.0,
    rx: 10,
    ry: 12,
    z: 0,
    frame: 0,
    total_frames: 1,
    rate_ms: 67,
    elapsed_ms: 0,
});
tick_weapon_muzzle_flashes(&mut state, 67);
assert!(state.weapon_muzzle_flashes.is_empty());
```

Add a spawn test that supplies a non-garrison event with `report_sound_id` and verifies one `GameSoundEvent::WeaponFired` is pushed with `screen_pos` equal to resolved FLH origin.

**Step 4: Verify**

Run: `cargo test app_fire_effects --lib`

Expected: spawn/tick tests pass.

**Step 5: Commit**

Commit message: `Spawn non-garrison muzzle fire effects`.

### Task 9: Wire Fire Effect Processing Into App Tick

**Why:** The app must process fire events after simulation advances and before sound dispatch drains the app sound queue.

**Files:**
- Modify: `src/app_sim_tick.rs`

**Pattern:** Existing `pending_fire_effects` drain and app sound event conversion.

**Step 1: Drain into a local vector and process after the sim mutable borrow**

Introduce a local vector before the `if let Some(sim) = &mut state.simulation` block:

```rust
let mut drained_fire_events: Vec<SimFireEvent> = Vec::new();
```

Inside the sim block, replace:

```rust
state.pending_fire_effects.extend(sim.fire_events.drain(..));
```

with:

```rust
drained_fire_events.extend(sim.fire_events.drain(..));
state
    .pending_fire_effects
    .extend(drained_fire_events.iter().cloned());
```

After the sim block ends and the mutable borrow of `state.simulation` is released, call:

```rust
crate::app_fire_effects::spawn_non_garrison_fire_effects(state, &drained_fire_events);
```

Place the call before the next fixed step can clear `state.pending_fire_effects`.

**Step 2: Tick active flashes in the visual update phase**

Near the existing call to `tick_garrison_muzzle_flashes`, add:

```rust
crate::app_fire_effects::tick_weapon_muzzle_flashes(
    state,
    sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32,
);
```

**Step 3: Verify**

Run: `cargo check --lib`

Expected: no borrow errors remain.

**Step 4: Commit**

Commit message: `Wire non-garrison fire effects into app tick`.

### Task 10: Render Non-Garrison Muzzle Flash Instances

**Why:** Active muzzle flashes need sprite instances to appear on screen.

**Files:**
- Modify: `src/app_instances/overlays.rs`
- Modify: `src/app_render/build_instances.rs`

**Pattern:** `build_garrison_muzzle_flash_instances`.

**Step 1: Add builder**

```rust
pub(crate) fn build_weapon_muzzle_flash_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
) {
    let (sim, atlas) = match (&state.simulation, &state.sprite_atlas) {
        (Some(s), Some(a)) => (s, a),
        _ => return,
    };
    let z = state.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.camera_x,
        state.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let (origin_y, world_height) = state
        .terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));

    for flash in &state.weapon_muzzle_flashes {
        if !in_view(flash.screen_x, flash.screen_y, 96.0, 96.0, cam_x, cam_y, sw, sh, 96.0) {
            continue;
        }
        let key = ShpSpriteKey {
            type_id: flash.shp_name.clone(),
            facing: 0,
            frame: flash.frame,
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else { continue; };
        let tint = state
            .lighting_grid
            .get(&(flash.rx, flash.ry))
            .copied()
            .unwrap_or(lighting::DEFAULT_TINT);
        let depth = compute_sprite_depth_params(origin_y, world_height, flash.screen_y, flash.z);
        paged[entry.page as usize].push(SpriteInstance {
            position: [flash.screen_x + entry.offset_x, flash.screen_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha: 1.0,
            ..Default::default()
        });
    }
}
```

Adjust imports to match the garrison builder's existing imports.

**Step 2: Wire builder into render**

In `src/app_render/build_instances.rs`, call the new builder next to garrison muzzle flashes:

```rust
app_instances::build_weapon_muzzle_flash_instances(state, &mut shp_paged);
app_instances::build_garrison_muzzle_flash_instances(state, &mut shp_paged);
```

Keep both in the same render layer as existing muzzle flashes.

**Step 3: Add tests**

Add a focused unit test around a private helper that builds the `ShpSpriteKey` from `WeaponMuzzleFlash`:

```rust
fn weapon_muzzle_flash_key(flash: &WeaponMuzzleFlash) -> ShpSpriteKey {
    ShpSpriteKey {
        type_id: flash.shp_name.clone(),
        facing: 0,
        frame: flash.frame,
        house_color: HouseColorIndex(0),
    }
}
```

Assert the helper returns `type_id == "MGUN-N"` and `frame == flash.frame`.

**Step 4: Verify**

Run: `cargo check --lib`

Expected: render instance build compiles.

**Step 5: Commit**

Commit message: `Render non-garrison muzzle flashes`.

### Task 11: Load Weapon `Anim=` SHPs Into the Atlas

**Why:** Spawned `MGUN-*` flashes will not render unless the atlas has their frames.

**Files:**
- Modify: `src/render/sprite_atlas.rs`

**Pattern:** Existing `effect_names` collection for warhead `AnimList` and weapon `OccupantAnim`.

**Step 1: Extend weapon effect collection**

Replace the current occupant-only loop with:

```rust
for weapon in r.weapons_iter() {
    for anim_name in &weapon.anim {
        if !effect_names.iter().any(|n| n.eq_ignore_ascii_case(anim_name)) {
            effect_names.push(anim_name.clone());
        }
    }
    if let Some(ref anim_name) = weapon.occupant_anim {
        if !effect_names.iter().any(|n| n.eq_ignore_ascii_case(anim_name)) {
            effect_names.push(anim_name.clone());
        }
    }
}
```

**Step 2: Add or extend atlas tests**

Add a test that builds a minimal ruleset with a weapon containing:

```ini
[TestWeapon]
Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW
```

Extract a private helper:

```rust
fn collect_effect_names(r: &RuleSet) -> Vec<String>
```

Unit-test that helper and verify the collected names include `MGUN-N` and `MGUN-NW`.

**Step 3: Verify**

Run: `cargo test sprite_atlas --lib`

Expected: atlas tests pass.

**Step 4: Commit**

Commit message: `Load weapon muzzle anim SHPs`.

### Task 12: Preserve Garrison Fire Behavior

**Why:** The new non-garrison path must not steal `OccupantAnim` or sound behavior from garrisoned buildings.

**Files:**
- Modify: `src/sim/combat/combat_tests.rs`
- Modify: `src/app_fire_effects.rs` tests

**Pattern:** Existing garrison fire event tests and `tick_garrison_muzzle_flashes`.

**Step 1: Add combat assertion**

For a garrison fire fixture, assert:

```rust
let ev = &sim.fire_events[0];
assert!(ev.garrison_muzzle_index.is_some());
assert_eq!(ev.occupant_anim.map(|id| sim.interner.resolve(id)), Some("UCFLASH"));
assert_eq!(ev.report_sound_id, None);
```

Also assert `sim.sound_events` still includes one `SimSoundEvent::WeaponFired` for garrison fire.

**Step 2: Add app helper assertion**

Pass a garrison `SimFireEvent` to `spawn_non_garrison_fire_effects` and assert:

```rust
assert!(state.weapon_muzzle_flashes.is_empty());
assert!(state.sound_events.is_empty());
```

**Step 3: Verify**

Run: `cargo test garrison --lib`

Expected: garrison tests pass.

**Step 4: Commit**

Commit message: `Guard garrison muzzle behavior`.

### Task 13: End-to-End Regression Tests for GI

**Why:** The validation case is player-visible GI standing/prone/deployed fire output.

**Files:**
- Modify: `src/sim/combat/combat_tests.rs`
- Modify: `src/app_fire_effects.rs` tests

**Pattern:** Existing GI fire-frame sync tests.

**Step 1: Standing GI**

Extend the standing GI delayed-fire test:

```rust
assert_eq!(sim.fire_events.len(), 1);
let ev = &sim.fire_events[0];
assert_eq!(sim.interner.resolve(ev.weapon_id), "M60");
assert_eq!(ev.weapon_slot, WeaponSlot::Primary);
assert_eq!(ev.report_sound_id.map(|id| sim.interner.resolve(id)), Some("GIAttack"));
```

**Step 2: Prone GI**

Extend the prone GI delayed-fire test with the same primary weapon assertions and confirm the event exists only on the prone fire frame.

**Step 3: Deployed GI**

Extend the deployed GI heavy-target test:

```rust
assert_eq!(sim.interner.resolve(ev.weapon_id), "Para");
assert_eq!(ev.weapon_slot, WeaponSlot::Secondary);
assert_eq!(ev.report_sound_id.map(|id| sim.interner.resolve(id)), Some("GIAttackDeployed"));
```

**Step 4: App origin fixture**

Add an app-level test that creates a non-garrison GI fire event for primary and secondary slots and verifies:

```rust
let primary = resolve_non_garrison_fire_origin(&state, &primary_event).unwrap();
let secondary = resolve_non_garrison_fire_origin(&state, &secondary_event).unwrap();
assert_ne!(primary.screen_y, secondary.screen_y);
```

The exact expected delta should be based on `PrimaryFireFLH=80,0,105` and `SecondaryFireFLH=80,0,90` after applying the helper transform.

**Step 5: Verify**

Run:

```powershell
cargo test combat --lib
cargo test app_fire_effects --lib
```

Expected: all GI event and origin tests pass.

**Step 6: Commit**

Commit message: `Test GI muzzle fire event parity`.

### Task 14: Full Verification

**Why:** This touches sim event contracts, app presentation, atlas loading, and audio placement.

**Files:** No source edits.

**Step 1: Run focused checks**

```powershell
cargo check --lib
cargo test combat --lib
cargo test app_fire_effects --lib
cargo test sprite_atlas --lib
cargo test animation --lib
```

Expected: all pass.

**Step 2: Run broader checks**

```powershell
cargo test --lib
```

Expected: pass unless unrelated dirty worktree changes have introduced branch-wide failures. If unrelated failures appear, record them with file/test names and do not fix them in this task.

**Step 3: Manual visual check**

Run the game and test:

- standing GI firing infantry: `MGUN-*` appears on rifle fire frame;
- prone GI firing infantry: `MGUN-*` appears on prone fire frame;
- deployed GI firing heavy armor: `MGUN-*` appears on `DeployedFire` and uses Para report sound;
- deployed GI firing infantry: `MGUN-*` appears on `DeployedFire` while weapon choice remains target-driven;
- garrison GI fire still uses `UCFLASH` at building fire ports.

**Step 4: Commit**

Commit message: `Verify GI muzzle FLH fire origin parity`.

## Sources & References

- **Design doc:** `docs/plans/2026-05-16-gi-non-garrison-muzzle-flh-origin-design.md`
- **Prior scan:** `docs/gap-scans/2026-05-16-disparity-scan-gi-muzzle-flh-origin.md`
- **Fire-frame scan:** `docs/gap-scans/2026-05-16-disparity-scan-gi-infantry-fire-sync.md`
- **Ghidra reports:** `docs/research/GI_GHIDRA_REPORT.md`
- **Ghidra reports:** `docs/research/ANIMCLASS_SPAWN_PATHS_GHIDRA_REPORT.md`
- **Ghidra reports:** `docs/research/WEAPONTYPECLASS_RUST_VS_FIRE_AT_TRACE.md`
- **Ghidra reports:** `docs/research/WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`
- **Live Ghidra verification:** `TechnoClass::Fire_At` at `0x006FDD50`
- **Live Ghidra verification:** `TechnoClass::GetFLH` at `0x006F3AD0`
- **INI keys:** `ini/artmd.ini [GI] PrimaryFireFLH=80,0,105`, `SecondaryFireFLH=80,0,90`
- **INI keys:** `ini/rulesmd.ini [M60] Report=GIAttack`, `Anim=MGUN-*`, `OccupantAnim=UCFLASH`
- **INI keys:** `ini/rulesmd.ini [Para] Report=GIAttackDeployed`, `Anim=MGUN-*`, `OccupantAnim=UCFLASH`
- **Related code:** `src/sim/combat/mod.rs`
- **Related code:** `src/sim/combat/combat_weapon.rs`
- **Related code:** `src/sim/world/mod.rs`
- **Related code:** `src/app_sim_tick.rs`
- **Related code:** `src/app_building_anim.rs`
- **Related code:** `src/app_instances/overlays.rs`
- **Related code:** `src/render/sprite_atlas.rs`
- **Related code:** `src/rules/flh.rs`
- **Related code:** `src/util/flh_transform.rs`
- **Prior commit:** `1391629 Sync infantry fire to animation frames`
