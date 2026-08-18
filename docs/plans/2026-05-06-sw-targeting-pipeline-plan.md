# Superweapon Click → Target → Fire UI Pipeline — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Wire the UI side of superweapon launch — clicking a charged SW
cameo enters a targeting cursor mode; clicking a tactical-map cell emits
`Command::LaunchSuperWeapon`; right-click and Esc cancel.

**Architecture:** Mirrors the existing building-placement pipeline at every
layer (state field, sidebar action, input handler, cursor feedback,
per-tick sync). Replaces `armed_building_placement: Option<String>` with a
unified `targeting_mode: Option<TargetingMode>` enum that carries either
`BuildingPlacement(String)` or `SuperWeapon(String)` — mutual exclusion by
construction. The sim-side `Command::LaunchSuperWeapon` dispatch is
already wired and tested; this plan only adds the UI layer.

**Design Doc:** docs/plans/2026-05-06-sw-targeting-pipeline-design.md

---

## Grounding Summary

- **Docs (`ra2-rust-game-docs/`):** SUPERCLASS_SYSTEM_GHIDRA_REPORT.md +
  SUPERWEAPON_LAUNCH_HANDLERS_REPORT.md +
  SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md cover SuperWeaponTypeClass
  layout (Action= at 0xBC, Range= at 0xF8, PreClick/PostClick at
  0xED/0xEE, AuxBuilding at 0xC8). Confidence HIGH; all active in YR.
- **Ghidra verification:** sim-side dispatch at
  `src/sim/world/world_commands.rs:917-1020` validates
  `is_active && is_ready` before per-kind handler; `reset_after_fire()`
  runs on success. No additional Ghidra work needed.
- **Repo pattern mirrored:** building placement —
  `armed_building_placement` field, `ArmPlacement`/`ClearPlacementMode`
  SidebarActions, `place_ready_building_at_cursor`,
  `sync_armed_building_placement`, placement cursor branch, per-tick
  preview update.
- **INI keys driving behavior:** `[<SW>] Action=` in `rulesmd.ini` —
  values used in YR: Nuke, ChronoSphere, ChronoWarp, IronCurtain,
  LightningStorm, ParaDrop, AmerParaDrop, PsychicDominator, SpyPlane,
  GeneticConverter, ForceShield, PsychicReveal. `IonCannon` is TS-legacy.
  Already parsed into `SuperWeaponType::action`.
- **Cursor frames already loaded** at `src/render/cursor_atlas.rs:250-340`
  for every relevant CursorId variant (`Nuke`, `Chronosphere`,
  `IronCurtain`, `LightningStorm`, `Paradrop`, `ForceShield`,
  `GeneticMutator`, `AirStrike`, `PsychicDominator`, `PsychicReveal`,
  `SpyPlane`).
- **Still unknown:** cursor sprite while hovering the cameo of an *armed*
  SW (default-cursor fallback chosen as documented parity drift); EVA
  voice cue on arm/cancel (deferred per design doc).

## Key Technical Decisions

- **Unified `TargetingMode` enum** replaces the parallel
  `armed_building_placement` field. **Confidence:** high. **Source:**
  design doc Approach 2; mutual-exclusion-by-construction win.
- **`Command::LaunchSuperWeapon` carries the SW INI section name interned**
  (e.g., `"LightningStormSpecial"`), NOT the SidebarImage SHP name (which
  collides — INTICON is shared by multiple SWs). **Confidence:** high.
  **Source:** `src/sim/world/world_commands.rs:925-948` resolves type via
  `interner.resolve(*sw_type_id) → rules.super_weapon()`.
- **`SuperWeaponView::display_name` is the SW INI section name**, not a
  CSF-localized string. **Confidence:** high. **Source:**
  `src/sim/superweapon/mod.rs:163-165` —
  `display_name: type_id_str.to_string()`. Plan adds an explicit
  `super_weapon_section` field on `SidebarItem`/`BuildEntry` so the
  dispatch never relies on `display_name` semantics.
- **No range circle drawn on the tactical map** during targeting. Cursor
  reticle only. **Confidence:** high. **Source:** gamemd reference
  behavior (Q2 (a) in design doc).
- **Cursor sprite over an armed-SW cameo falls back to default**.
  **Confidence:** medium — gamemd reference behavior UNKNOWN.
  **Source:** none — flagged as accepted parity drift; needs RE if
  proven visible.

## Open Questions

### Resolved During Planning

- **How is `display_name` populated for SW views?** —
  `superweapon_views_for_owner` sets it to the resolved interned section
  name. So today it carries the section name, not the localized name.
  The plan adds `super_weapon_section` explicitly to decouple.
- **Multiple SWs sharing `SidebarImage=INTICON`** — confirmed in
  rulesmd.ini. The dispatch must use section name (unique), not type_id.

### Deferred to Implementation

- **Will any test that constructs `SidebarItem` literally need updating?**
  Grep at execution time before Task 5; expected zero matches outside
  builder code, but verify.
- **Two-click Chrono behavior** — out of scope; Chrono cameos arm and
  fire on single click, sim hits `other =>` warn. Address when Chrono
  launch handlers ship sim-side.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_types.rs` | Add `TargetingMode` enum + helpers; extend `CursorFeedbackKind` |
| Modify | `src/app.rs` | Replace `armed_building_placement` with `targeting_mode`; add accessor helpers |
| Modify | `src/sidebar/mod.rs` | Extend `SidebarAction`; extend `SidebarItem`; update `hit_test` |
| Modify | `src/sidebar/sidebar_view.rs` | Extend `BuildEntry`; populate SW fields; signature change for `armed` parameter |
| Modify | `src/app_input.rs` | New SidebarAction handlers; SW launch wiring; cancel paths |
| Modify | `src/app_commands.rs` | Add `launch_super_weapon_at_cursor`; migrate writers to `targeting_mode` |
| Modify | `src/app_sidebar_render.rs` | Replace `sync_armed_building_placement` with `sync_targeting_mode`; pass-through update |
| Modify | `src/app_cursor.rs` | Add `super_weapon_cursor_id` table; SW cursor branch |
| Modify | `src/app_sim_tick.rs` | Migrate `update_building_placement_preview` reader |
| Modify | `src/app_render_tests.rs` | Rewrite sync tests for `TargetingMode` |

## Interface Changes

- **`AppState.armed_building_placement`** → removed; replaced by
  `AppState.targeting_mode: Option<TargetingMode>`. 24 call sites updated.
- **`SidebarAction`** — adds `ArmSuperWeapon(String)` and
  `ClearSuperWeaponMode`. Consumers: `apply_sidebar_action`.
- **`SidebarItem`** — adds `is_superweapon: bool` and
  `super_weapon_section: Option<String>`. Consumers: `hit_test`,
  `app_sidebar_build.rs` (cameo render — additive only, doesn't read
  the new fields).
- **`BuildEntry`** (private) — same two new fields.
- **`build_sidebar_view_with_spec`** — `armed_building: Option<&str>`
  parameter swaps to `armed: Option<&TargetingMode>`. Single caller in
  `app_sidebar_render.rs`.
- **`sync_armed_building_placement`** → renamed to `sync_targeting_mode`;
  signature changes from `(&mut Option<String>, &mut Option<...>, &[Ready])`
  to `(&mut Option<TargetingMode>, &mut Option<...>, &[Ready], &[SuperWeaponView])`.
- **`CursorFeedbackKind`** — adds `SuperWeaponTarget(CursorId)` variant.
  Consumer: `cursor_id_for_feedback`.
- **`Command::LaunchSuperWeapon`** — already exists, no change.

## Sim Checklist

This plan does NOT modify `sim/`. The only sim-touching code is the new
`launch_super_weapon_at_cursor` in `app_commands.rs`, which calls the
existing `schedule_command()` helper to enqueue `Command::LaunchSuperWeapon`
via the existing `pending_commands` queue.

- [x] No new sim state introduced
- [x] No new tick-order changes
- [x] `Command::LaunchSuperWeapon` execute_tick uses `input_delay_ticks`
      via `schedule_command` (already enforced)
- [x] No `f32`/`f64` in sim logic introduced
- [x] No new dependencies from sim/ on render/ui/sidebar/audio/net

## Risk Areas

- **AppState field rename touches 24 sites across 9 files.** Task 7
  is large but mechanical. Build must be re-validated after each step
  within the task to catch missed sites early.
- **`SidebarItem` shape change**. Any literal constructor must add the
  two new fields. Grep before editing (execution-time check in Task 5).
- **Sidebar `hit_test` is the entry point for every cameo click.** Tests
  must cover all three new outcomes (`ArmSuperWeapon`, `ClearSuperWeaponMode`,
  `None` for not-ready/right-click) AND verify existing build-cameo
  behavior unchanged.
- **`sync_targeting_mode` is called every render frame.** Must remain
  cheap and deterministic. Test invariant: building-placement validation
  unchanged from prior `sync_armed_building_placement`.
- **`current_cursor_feedback_kind` ordering.** SW branch must come before
  the building-placement branch but after sidebar/minimap/edge-scroll
  branches. Wrong ordering masks the SW reticle while hovering UI chrome
  (intended) or, if too early, would mask edge-scroll arrows (bug).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | `Action=` → `CursorId` mapping for all 12 YR-active strings | Wrong cursor sprite per SW = visible parity break every match. Lightning Storm reticle vs Iron Curtain reticle vs Paradrop reticle are all distinct in the original. | Table unit test in Task 2; in-game verification in Task 14 |
| Task 6 | `is_armed` toggle on second click of same cameo | Player clicks cameo, decides not to fire, clicks the same cameo again to cancel. Common interaction pattern. | hit_test test in Task 8; in-game verification in Task 14 |
| Task 8 | Right-click on SW cameo returns `None` (NOT `CancelBuild`) | SWs have no queue. Right-click on a charging Lightning Storm must NOT cancel anything. | hit_test test in Task 8 |
| Task 9 | Auto-cancel when granting building dies (`is_active=false`) | Common scenario: player arms Lightning Storm, ConYard with the radar dies, the SW silently goes inactive. UI must reflect this. | sync test in Task 9 |
| Task 10 | Mutual exclusion: arming SW clears building placement (and vice versa) | Player arms a refinery, then clicks Lightning Storm cameo. The refinery ghost must vanish, the reticle must take over. | hit_test + apply_sidebar_action tests; in-game in Task 14 |
| Task 11 | `Command::LaunchSuperWeapon` carries section name (not SidebarImage) | Multiple SWs share `SidebarImage=INTICON`. Wrong key = sim dispatches wrong SW. | launch_super_weapon_at_cursor uses `intern_type` on the section name passed in by `apply_sidebar_action(ArmSuperWeapon(section))` |
| Task 12 | Click on tactical fires; click on sidebar/minimap does not | Click on the sidebar after arming must NOT fire the SW into the off-map cell behind the panel. **Critical:** the RELEASE of the arming click itself lands on the cameo. Without the sidebar/minimap guard in `launch_super_weapon_at_cursor`, the arming click would self-fire the SW at a bogus cell. | Sidebar/minimap guard at top of `launch_super_weapon_at_cursor` (Task 12 step 2) returns early WITHOUT clearing `targeting_mode`. Manual click-on-sidebar test in Task 14. |
| Task 12 | Right-click and Esc both cancel; sim command NOT emitted on cancel | Player must be able to back out without firing. Common interaction. | Task 14 in-game verification |
| Task 13 | No range circle drawn on tactical map during targeting | gamemd doesn't draw one; we shouldn't either. | Visual inspection in Task 14 — no extra render layer added |

---

## Tasks

### Task 1: Add `TargetingMode` enum to `app_types.rs`

**Why:** Foundation type with zero dependents. Adding it first lets every
later task reference it.

**Files:**
- Modify: `src/app_types.rs` (after the `OrderMode` enum at line 30-34)

**Pattern:** Sibling to existing `OrderMode` enum.

**Step 1: Append the enum and helpers**

Add at the end of `src/app_types.rs`:

```rust
/// Mutually-exclusive cursor-on-tactical-map targeting modes.
///
/// Building placement and superweapon targeting cannot both be active at
/// once. Arming one clears the other; right-click and Esc clear both.
/// The variant payload is the type_id (interned section name) the
/// targeting refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetingMode {
    /// Ready building waiting to be placed on the tactical map.
    /// Payload: building INI section name (e.g., "GAPOWR").
    BuildingPlacement(String),
    /// Charged superweapon waiting for a target cell.
    /// Payload: SW INI section name (e.g., "LightningStormSpecial").
    SuperWeapon(String),
}

impl TargetingMode {
    pub fn as_building_placement(&self) -> Option<&str> {
        match self {
            Self::BuildingPlacement(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_super_weapon(&self) -> Option<&str> {
        match self {
            Self::SuperWeapon(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn is_building_placement(&self) -> bool {
        matches!(self, Self::BuildingPlacement(_))
    }

    pub fn is_super_weapon(&self) -> bool {
        matches!(self, Self::SuperWeapon(_))
    }
}
```

**Step 2: Verify**

Run: `cargo check`
Expected: PASS (no callers yet — this is pure addition).

**Step 3: Commit**

Message: `app_types: add TargetingMode enum for unified arm-state`

---

### Task 2: Add `super_weapon_cursor_id` table in `app_cursor.rs`

**Why:** Foundation lookup table. Pure-function with zero dependents — testable
in isolation. Required by Task 11 (cursor branch).

**Files:**
- Modify: `src/app_cursor.rs` (append after the existing module body)

**Pattern:** Same shape as `scroll_dir_to_cursor_id` at
`src/app_cursor.rs:399-410` — a `match` over a string/enum returning
`CursorId`.

**Step 1: Append the lookup function**

Add to `src/app_cursor.rs`:

```rust
/// Map a SuperWeaponType `Action=` INI string to its targeting cursor.
///
/// Action strings come from `[SWType] Action=` in rulesmd.ini. Cursor
/// frame ranges are pre-loaded in `render/cursor_atlas.rs`.
///
/// Returns `None` for `IonCannon` (TS-legacy, no YR SW uses it) and any
/// unrecognized string. Caller should fall back to `CursorId::Default`.
pub(crate) fn super_weapon_cursor_id(action: &str) -> Option<CursorId> {
    match action {
        "Nuke" => Some(CursorId::Nuke),
        "ChronoSphere" => Some(CursorId::Chronosphere),
        "ChronoWarp" => Some(CursorId::Chronosphere),
        "IronCurtain" => Some(CursorId::IronCurtain),
        "LightningStorm" => Some(CursorId::LightningStorm),
        "ParaDrop" => Some(CursorId::Paradrop),
        "AmerParaDrop" => Some(CursorId::Paradrop),
        "PsychicDominator" => Some(CursorId::PsychicDominator),
        "SpyPlane" => Some(CursorId::SpyPlane),
        "GeneticConverter" => Some(CursorId::GeneticMutator),
        "ForceShield" => Some(CursorId::ForceShield),
        "PsychicReveal" => Some(CursorId::PsychicReveal),
        // IonCannon is TS-legacy — no YR superweapon uses this Action.
        _ => None,
    }
}
```

**Step 2: Add table tests**

Append to `src/app_cursor.rs` (or extend an existing `#[cfg(test)] mod tests`
if present — there is none at end of file currently):

```rust
#[cfg(test)]
mod tests {
    use super::super_weapon_cursor_id;
    use crate::app_types::CursorId;

    #[test]
    fn maps_every_yr_active_action() {
        assert_eq!(super_weapon_cursor_id("Nuke"), Some(CursorId::Nuke));
        assert_eq!(super_weapon_cursor_id("ChronoSphere"), Some(CursorId::Chronosphere));
        assert_eq!(super_weapon_cursor_id("ChronoWarp"), Some(CursorId::Chronosphere));
        assert_eq!(super_weapon_cursor_id("IronCurtain"), Some(CursorId::IronCurtain));
        assert_eq!(super_weapon_cursor_id("LightningStorm"), Some(CursorId::LightningStorm));
        assert_eq!(super_weapon_cursor_id("ParaDrop"), Some(CursorId::Paradrop));
        assert_eq!(super_weapon_cursor_id("AmerParaDrop"), Some(CursorId::Paradrop));
        assert_eq!(super_weapon_cursor_id("PsychicDominator"), Some(CursorId::PsychicDominator));
        assert_eq!(super_weapon_cursor_id("SpyPlane"), Some(CursorId::SpyPlane));
        assert_eq!(super_weapon_cursor_id("GeneticConverter"), Some(CursorId::GeneticMutator));
        assert_eq!(super_weapon_cursor_id("ForceShield"), Some(CursorId::ForceShield));
        assert_eq!(super_weapon_cursor_id("PsychicReveal"), Some(CursorId::PsychicReveal));
    }

    #[test]
    fn returns_none_for_ts_legacy_and_unknown() {
        assert_eq!(super_weapon_cursor_id("IonCannon"), None);
        assert_eq!(super_weapon_cursor_id(""), None);
        assert_eq!(super_weapon_cursor_id("BogusAction"), None);
    }
}
```

**Step 3: Verify**

Run: `cargo test super_weapon_cursor_id`
Expected: 2 tests pass.

**Step 4: Commit**

Message: `app_cursor: add super_weapon_cursor_id Action=→CursorId table`

---

### Task 3: Extend `SidebarAction` with two new variants

**Why:** Required by `apply_sidebar_action` (Task 10) and `hit_test`
(Task 8). Adding the variants first means downstream tasks compile
incrementally.

**Files:**
- Modify: `src/sidebar/mod.rs:127-141`

**Pattern:** Sibling variants to existing `ArmPlacement(String)` /
`ClearPlacementMode`.

**Step 1: Add variants**

In `src/sidebar/mod.rs`, replace the `SidebarAction` enum (line 126-141) with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarAction {
    None,
    SelectTab(SidebarTab),
    BuildType(String),
    ArmPlacement(String),
    ClearPlacementMode,
    /// Arm the targeting cursor for a charged superweapon.
    /// Payload: SW INI section name (e.g., "LightningStormSpecial").
    ArmSuperWeapon(String),
    /// Clear the SW targeting cursor (toggle off / second click on cameo).
    ClearSuperWeaponMode,
    TogglePauseQueue(ProductionCategory),
    CycleProducer(ProductionCategory),
    CancelBuild(String),
    CancelLastBuild,
    CycleOwner,
    PlaceStarterBase,
    SpawnTestUnits,
    Deploy,
}
```

**Step 2: Verify**

Run: `cargo check`
Expected: PASS. `apply_sidebar_action`'s `match` is non-exhaustive on
`SidebarAction` because it uses an explicit pattern list, but the
compiler will warn if any variant is unhandled. Existing `match` in
`apply_sidebar_action` uses arm-by-arm without a wildcard; the new
variants will produce a non-exhaustive match warning. **Expected
warning** about missing arms — Task 10 fills them in.

If `cargo check` errors out (rather than warns), it means
`apply_sidebar_action`'s match is `#[deny(non_exhaustive_patterns)]` or
similar. In that case, add temporary stubs:

```rust
SidebarAction::ArmSuperWeapon(_) => {}
SidebarAction::ClearSuperWeaponMode => {}
```

to `apply_sidebar_action` to keep compile green; Task 10 will replace.

**Step 3: Commit**

Message: `sidebar: add ArmSuperWeapon / ClearSuperWeaponMode actions`

---

### Task 4: Extend `SidebarItem` and `BuildEntry` with SW fields

**Why:** `hit_test` (Task 8) needs `is_superweapon` to branch correctly.
`super_weapon_section` carries the unique INI section name (the unique
key — `display_name` and `type_id` are not unique across SWs sharing
INTICON).

**Files:**
- Modify: `src/sidebar/mod.rs:143-165` (`SidebarItem` struct + impl)
- Modify: `src/sidebar/sidebar_view.rs:277-288` (`BuildEntry` struct)
- Modify: `src/sidebar/sidebar_view.rs:142-175` (`SidebarItem` constructor)
- Modify: `src/sidebar/sidebar_view.rs:309-326, 343-413` (`BuildEntry`
  constructor sites)

**Pattern:** Additive struct fields with `Default`/literal initialization
in every constructor.

**Step 1: Grep for `SidebarItem {` literal constructors**

```
grep -rn "SidebarItem {" src/
```

Expected: only `src/sidebar/sidebar_view.rs:155` and possibly tests.
If any other site exists outside this file, list it in step-3 substep
to update.

**Step 2: Add fields to `SidebarItem`**

In `src/sidebar/mod.rs`, replace the `SidebarItem` struct (line 143-158):

```rust
#[derive(Debug, Clone)]
pub struct SidebarItem {
    pub rect: Rect,
    pub type_id: String,
    pub display_name: String,
    pub cost: Option<i32>,
    pub has_cameo_art: bool,
    pub queue_category: ProductionCategory,
    pub enabled: bool,
    pub progress: f32,
    pub queued_count: usize,
    /// True when this type is the one actively being produced in its category.
    pub is_building_this_type: bool,
    pub is_ready: bool,
    pub is_armed: bool,
    /// True if this cameo represents a superweapon (not a buildable).
    pub is_superweapon: bool,
    /// Unique SW INI section name (e.g., "LightningStormSpecial"). Set
    /// only when `is_superweapon=true`. Multiple SWs may share `type_id`
    /// (SidebarImage), but section names are unique.
    pub super_weapon_section: Option<String>,
}
```

**Step 3: Add fields to `BuildEntry`**

In `src/sidebar/sidebar_view.rs`, replace the `BuildEntry` struct
(line 277-288):

```rust
struct BuildEntry {
    type_id: String,
    display_name: String,
    cost: Option<i32>,
    enabled: bool,
    progress: f32,
    queued_count: usize,
    /// True when this type is the one actively being produced in its category.
    is_building_this_type: bool,
    is_ready: bool,
    is_armed: bool,
    is_superweapon: bool,
    super_weapon_section: Option<String>,
}
```

**Step 4: Populate the SW branch**

In `src/sidebar/sidebar_view.rs`, replace the SW entry constructor
(line 314-324) with:

```rust
sw_entries.push(BuildEntry {
    type_id,
    display_name: sw.display_name.clone(),
    cost: None,
    enabled: sw.is_online,
    progress: sw.progress,
    queued_count: 0,
    is_building_this_type: !sw.is_ready && sw.is_online && sw.progress > 0.0,
    is_ready: sw.is_ready,
    is_armed: false, // Recomputed in Task 7 once `armed` parameter changes.
    is_superweapon: true,
    super_weapon_section: Some(sw.display_name.clone()),
});
```

**Step 5: Populate the build-cameo branches**

In `src/sidebar/sidebar_view.rs`, every `BuildEntry { ... }` literal in
`collect_build_entries` (the three sites in the build-options loop and
the ready-buildings append loop) needs the two new fields. Add to each
existing literal:

```rust
            is_superweapon: false,
            super_weapon_section: None,
```

Sites (current line numbers):
- Line 343-355 (the `if is_ready` ready-building branch)
- Line 374-385 (the `else` not-ready branch)
- Line 402-413 (the ready-buildings append loop)

**Step 6: Populate the `SidebarItem` constructor**

In `src/sidebar/sidebar_view.rs:155-173` (the `entries.drain(..)` map
closure), pass the new fields through:

```rust
SidebarItem {
    rect: Rect { x, y, w: layout_spec.cameo_width.round(), h: layout_spec.cameo_height.round() },
    type_id: entry.type_id,
    display_name: entry.display_name,
    cost: entry.cost,
    has_cameo_art: false,
    queue_category: selected_category,
    enabled: entry.enabled,
    progress: entry.progress,
    queued_count: entry.queued_count,
    is_building_this_type: entry.is_building_this_type,
    is_ready: entry.is_ready,
    is_armed: entry.is_armed,
    is_superweapon: entry.is_superweapon,
    super_weapon_section: entry.super_weapon_section,
}
```

**Step 7: Verify**

Run: `cargo check`
Expected: PASS. Existing `SidebarItem`-reading code is unaffected (the
new fields are unread for now).

Run: `cargo test sidebar::`
Expected: existing tests still pass (the tests construct `SidebarView`
through the public builder, not literal `SidebarItem`).

**Step 8: Commit**

Message: `sidebar: add is_superweapon + super_weapon_section to SidebarItem/BuildEntry`

---

### Task 5: Extend `CursorFeedbackKind` with `SuperWeaponTarget(CursorId)`

**Why:** Required by Task 11 (cursor branch). Carrying `CursorId` lets
`cursor_id_for_feedback` pass through without re-doing the Action=
lookup.

**Files:**
- Modify: `src/app_types.rs:156-181` (`CursorFeedbackKind` enum)
- Modify: `src/app_cursor.rs:377-397` (`cursor_id_for_feedback`)

**Pattern:** Sibling variants to existing `Scroll(ScrollDir)` —
data-carrying enum variant where the payload is a `Copy` type.

**Step 1: Add the variant**

In `src/app_types.rs`, replace the `CursorFeedbackKind` enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorFeedbackKind {
    Move,
    AttackMove,
    Guard,
    FriendlyUnit,
    FriendlyStructure,
    EnemyUnit,
    EnemyStructure,
    EnemyOutOfRange,
    Invalid,
    PlaceValid,
    PlaceInvalid,
    /// Edge-scroll arrow — shown when cursor is near a screen edge.
    Scroll(ScrollDir),
    /// Move cursor minimap variant (frames 42–51) — shown when hovering over the minimap.
    MinimapMove,
    /// Deploy/undeploy cursor — shown when a Deployer unit hovers over itself.
    Deploy,
    /// Enter cursor — garrison, capture, board transport, sabotage.
    Enter,
    /// Engineer repair cursor — engineer hovering a damaged friendly building.
    EngineerRepair,
    /// Pan cursor — shown while middle-mouse dragging to scroll the map.
    Pan,
    /// Superweapon targeting reticle — shown while a charged SW is armed
    /// and the cursor is over the tactical map. Payload is the per-SW
    /// CursorId resolved from the `Action=` INI string.
    SuperWeaponTarget(CursorId),
}
```

The existing `derive(Copy)` is preserved because `CursorId` is `Copy`.

**Step 2: Pass through in `cursor_id_for_feedback`**

In `src/app_cursor.rs`, update the match in `cursor_id_for_feedback`
(line 377-397). Add this arm before the closing brace of the match:

```rust
        CursorFeedbackKind::SuperWeaponTarget(id) => Some(id),
```

**Step 3: Verify**

Run: `cargo check`
Expected: PASS.

**Step 4: Commit**

Message: `app_types: add CursorFeedbackKind::SuperWeaponTarget(CursorId)`

---

### Task 6: AppState atomic migration to `targeting_mode`

**Why:** Replace `armed_building_placement: Option<String>` with
`targeting_mode: Option<TargetingMode>` across all 24 call sites.
Behavior is unchanged for building placement — this is a pure
state-shape refactor that unlocks the SW path. **This task is large but
mechanical.** Each step is one-file local; the build only goes green
again at the end.

**Files:**
- Modify: `src/app.rs:219, 618` (field decl + init)
- Modify: `src/app_input.rs:50, 61, 148-149, 247, 250, 298-299` (8 sites)
- Modify: `src/app_commands.rs:259, 538` (2 writers)
- Modify: `src/app_cursor.rs:40` (1 reader)
- Modify: `src/app_sim_tick.rs:751` (1 reader)
- Modify: `src/app_sidebar_render.rs:48-49, 113, 133, 148-164` (sync
  function + pass-through)
- Modify: `src/app_render_tests.rs:7, 395, 418` (test imports + calls)

**Pattern:** Building-placement state shape, generalized via
`TargetingMode`.

**Step 1: Replace the field in `AppState`**

In `src/app.rs`, change line 219:
- Before: `pub(crate) armed_building_placement: Option<String>,`
- After:
```rust
/// Mutually-exclusive cursor-on-tactical-map targeting mode (building
/// placement OR superweapon). Right-click and Esc clear; arming one
/// kind clears the other.
pub(crate) targeting_mode: Option<TargetingMode>,
```

Add the import at the top of `src/app.rs` (find the `app_types` use
line, extend it):
```rust
use crate::app_types::{..., TargetingMode};
```

(If `app_types` is not currently imported by name, add a fresh line.)

**Step 2: Update the init in `AppState::new` / equivalent**

In `src/app.rs:618`, change:
- Before: `armed_building_placement: None,`
- After: `targeting_mode: None,`

**Step 3: Add accessor helpers on `AppState`**

In `src/app.rs`, find the `impl AppState` block (or add one if absent
near the struct). Add:

```rust
impl AppState {
    /// Return the building-placement section name if the targeting mode
    /// is set to `BuildingPlacement`, else `None`.
    pub(crate) fn armed_building_type(&self) -> Option<&str> {
        self.targeting_mode
            .as_ref()
            .and_then(crate::app_types::TargetingMode::as_building_placement)
    }

    /// Return the SW section name if the targeting mode is set to
    /// `SuperWeapon`, else `None`.
    pub(crate) fn armed_super_weapon_type(&self) -> Option<&str> {
        self.targeting_mode
            .as_ref()
            .and_then(crate::app_types::TargetingMode::as_super_weapon)
    }
}
```

If an `impl AppState` already exists with other helpers, append to that.
Otherwise add a new block adjacent to the struct definition.

**Step 4: Migrate `app_input.rs` sites**

- Line 50: `if state.armed_building_placement.is_some() {` →
  `if state.armed_building_type().is_some() {`
- Line 61: `if let Some(type_id) = state.armed_building_placement.clone() {` →
  `if let Some(type_id) = state.armed_building_type().map(str::to_owned) {`
- Lines 148-149 (right-click cancel):
  ```rust
  if state.armed_building_placement.is_some() {
      state.armed_building_placement = None;
  ```
  →
  ```rust
  if state.targeting_mode.is_some() {
      state.targeting_mode = None;
  ```
  (The `state.building_placement_preview = None` line below stays.)
- Line 247 (`SidebarAction::ArmPlacement(type_id) =>`):
  ```rust
  state.armed_building_placement = Some(type_id);
  ```
  →
  ```rust
  state.targeting_mode = Some(crate::app_types::TargetingMode::BuildingPlacement(type_id));
  ```
- Line 250 (`SidebarAction::ClearPlacementMode =>`):
  ```rust
  state.armed_building_placement = None;
  ```
  →
  ```rust
  state.targeting_mode = None;
  ```
- Lines 298-299 (Esc handler):
  ```rust
  } else if state.armed_building_placement.is_some() {
      state.armed_building_placement = None;
  ```
  →
  ```rust
  } else if state.targeting_mode.is_some() {
      state.targeting_mode = None;
  ```

**Step 5: Migrate `app_commands.rs` sites**

- Line 259 (in `place_ready_building_at_cursor`):
  ```rust
  state.armed_building_placement = None;
  ```
  →
  ```rust
  state.targeting_mode = None;
  ```
- Line 538 (in `cycle_local_owner`):
  ```rust
  state.armed_building_placement = None;
  ```
  →
  ```rust
  state.targeting_mode = None;
  ```

**Step 6: Migrate `app_cursor.rs` site**

- Line 40:
  ```rust
  if state.armed_building_placement.is_some() {
  ```
  →
  ```rust
  if state.armed_building_type().is_some() {
  ```

**Step 7: Migrate `app_sim_tick.rs` site**

- Line 751 (in `update_building_placement_preview`):
  ```rust
  let Some(type_id) = state.armed_building_placement.as_deref() else {
  ```
  →
  ```rust
  let Some(type_id) = state.armed_building_type() else {
  ```

**Step 8: Migrate `app_sidebar_render.rs` — `sync_armed_building_placement`
signature**

This function gets renamed and extended in Task 9. For now (Task 6),
just change its **first parameter type** to keep the build green:

In `src/app_sidebar_render.rs:148-164`:
```rust
pub(crate) fn sync_armed_building_placement(
    targeting_mode: &mut Option<crate::app_types::TargetingMode>,
    building_placement_preview: &mut Option<crate::sim::production::BuildingPlacementPreview>,
    ready_buildings: &[production::ReadyBuildingView],
    interner: Option<&crate::sim::intern::StringInterner>,
) {
    let still_valid = targeting_mode
        .as_ref()
        .and_then(crate::app_types::TargetingMode::as_building_placement)
        .map_or(true, |armed| {
            ready_buildings.iter().any(|ready| {
                interner.map_or(false, |i| {
                    i.resolve(ready.type_id).eq_ignore_ascii_case(armed)
                })
            })
        });
    if !still_valid {
        *targeting_mode = None;
        *building_placement_preview = None;
    }
}
```

This preserves prior behavior: only `BuildingPlacement` is checked; if
the future SW variant is set, `still_valid` is `true` (no reader yet —
SW validation is added in Task 9). The function name is kept the same
in this task; Task 9 renames to `sync_targeting_mode` and adds SW
validation.

**Step 9: Update the caller in `app_sidebar_render.rs`**

Line 48-49:
```rust
sync_armed_building_placement(
    &mut state.armed_building_placement,
```
→
```rust
sync_armed_building_placement(
    &mut state.targeting_mode,
```

Lines 113 and 133 (pass-through to `build_sidebar_view_with_spec` —
the `armed_building` parameter currently takes `Option<&str>`).
Replace each:
```rust
state.armed_building_placement.as_deref(),
```
→
```rust
state.armed_building_type(),
```

(The full signature change of `build_sidebar_view_with_spec` to take
`Option<&TargetingMode>` happens in Task 7. For Task 6 we keep the
existing `Option<&str>` parameter and feed it via the accessor.)

**Step 10: Update `app_render_tests.rs`**

Line 7 (import — keep as-is, function name unchanged in this task):
```rust
use crate::app_sidebar_render::sync_armed_building_placement;
```
(no change)

Line 395 — `test_ready_buildings_do_not_auto_arm_placement`:
```rust
let mut armed = None;
let mut preview = None;
...
sync_armed_building_placement(&mut armed, &mut preview, &ready, None);
```
The `armed` declared as `Option<String>` literal — the new signature
expects `Option<TargetingMode>`. Replace:
```rust
let mut armed: Option<crate::app_types::TargetingMode> = None;
```
The `sync_armed_building_placement(&mut armed, ...)` call is unchanged.

Line 405-418 — `test_invalid_armed_building_clears_when_not_ready`:
```rust
let mut armed = Some("GAPOWR".to_string());
```
→
```rust
let mut armed = Some(crate::app_types::TargetingMode::BuildingPlacement(
    "GAPOWR".to_string(),
));
```

**Step 11: Verify**

Run: `cargo check`
Expected: PASS.

Run: `cargo test app_render_tests::test_ready_buildings_do_not_auto_arm_placement app_render_tests::test_invalid_armed_building_clears_when_not_ready`
Expected: 2 tests pass.

Run: `cargo test sidebar::`
Expected: existing sidebar tests still pass.

**Step 12: Sanity-grep for orphans**

```
grep -rn "armed_building_placement" src/
```
Expected: zero matches in src/ (the field name is gone).

If any matches remain, update them following the pattern above.

**Step 13: Commit**

Message: `app: replace armed_building_placement with TargetingMode field`

---

### Task 7: `build_sidebar_view_with_spec` signature: armed parameter swap

**Why:** Lets `collect_build_entries` compute `is_armed` for both
building cameos and SW cameos uniformly. Without this, SW
`is_armed` is hard-coded false (set in Task 4 step 4).

**Files:**
- Modify: `src/sidebar/sidebar_view.rs:17-51` (`build_sidebar_view`
  wrapper)
- Modify: `src/sidebar/sidebar_view.rs:53-70` (`build_sidebar_view_with_spec`
  signature)
- Modify: `src/sidebar/sidebar_view.rs:290-298` (`collect_build_entries`
  signature)
- Modify: `src/sidebar/sidebar_view.rs:299-301` (`armed_id` derivation)
- Modify: `src/sidebar/sidebar_view.rs:340-413` (the two `is_armed`
  computations)
- Modify: `src/app_sidebar_render.rs:113, 133` (pass-through)

**Pattern:** Parameter type widening; existing `Option<&str>` →
`Option<&TargetingMode>`.

**Step 1: Update `build_sidebar_view_with_spec` parameter**

In `src/sidebar/sidebar_view.rs:53-70`, replace the parameter
`armed_building: Option<&str>` with:
```rust
armed: Option<&crate::app_types::TargetingMode>,
```

Cascade to the call to `collect_build_entries` inside the function:
```rust
let mut all_entries = collect_build_entries(
    selected_category,
    queue_items,
    build_options,
    ready_buildings,
    armed,
    interner,
    sw_views,
);
```

**Step 2: Update `build_sidebar_view` wrapper**

The wrapper at line 17-51 currently passes `armed_building` through.
Update it to also accept `Option<&TargetingMode>`:
```rust
pub fn build_sidebar_view(
    screen_w: f32,
    screen_h: f32,
    active_tab: SidebarTab,
    credits: i32,
    power_produced: i32,
    power_drained: i32,
    tab_button_size: Option<[f32; 2]>,
    queue_items: &[QueueItemView],
    build_options: &[BuildOption],
    ready_buildings: &[ReadyBuildingView],
    armed: Option<&crate::app_types::TargetingMode>,
    producer_focus: &[ProducerFocusView],
    scroll_rows: usize,
    interner: Option<&crate::sim::intern::StringInterner>,
) -> SidebarView {
    build_sidebar_view_with_spec(
        SidebarChromeLayoutSpec::stock(),
        screen_w, screen_h, active_tab, credits, power_produced, power_drained,
        tab_button_size, queue_items, build_options, ready_buildings,
        armed, producer_focus, scroll_rows, interner,
        &[],
    )
}
```

**Step 3: Update `collect_build_entries` signature and `armed_id` derivation**

Line 290-301 — replace:
```rust
fn collect_build_entries(
    category: ProductionCategory,
    queue_items: &[QueueItemView],
    build_options: &[BuildOption],
    ready_buildings: &[ReadyBuildingView],
    armed_building: Option<&str>,
    interner: Option<&crate::sim::intern::StringInterner>,
    sw_views: &[SuperWeaponView],
) -> Vec<BuildEntry> {
    let armed_id: Option<InternedId> = armed_building.and_then(|s| interner.and_then(|i| i.get(s)));
```
with:
```rust
fn collect_build_entries(
    category: ProductionCategory,
    queue_items: &[QueueItemView],
    build_options: &[BuildOption],
    ready_buildings: &[ReadyBuildingView],
    armed: Option<&crate::app_types::TargetingMode>,
    interner: Option<&crate::sim::intern::StringInterner>,
    sw_views: &[SuperWeaponView],
) -> Vec<BuildEntry> {
    // Building-placement is_armed: matched by interned type_id (existing logic).
    let armed_building_id: Option<InternedId> = armed
        .and_then(crate::app_types::TargetingMode::as_building_placement)
        .and_then(|s| interner.and_then(|i| i.get(s)));
    // SW is_armed: matched by section name (string compare).
    let armed_sw_section: Option<&str> = armed
        .and_then(crate::app_types::TargetingMode::as_super_weapon);
```

**Step 4: Update SW `is_armed` computation**

In the SW branch (around line 314-326), set:
```rust
is_armed: armed_sw_section
    .map_or(false, |s| s.eq_ignore_ascii_case(&sw.display_name)),
```

(Replaces the hard-coded `is_armed: false` from Task 4 step 4.)

**Step 5: Update building-placement `is_armed` computation**

In `collect_build_entries`, every existing reference to `armed_id` (as
written today) is replaced with `armed_building_id`. Sites:
- Line 341: `let is_armed = is_ready && armed_id == Some(opt.type_id);`
  → `let is_armed = is_ready && armed_building_id == Some(opt.type_id);`
- Line 401: `let is_armed = armed_id == Some(r.type_id);`
  → `let is_armed = armed_building_id == Some(r.type_id);`

**Step 6: Update callers in `app_sidebar_render.rs`**

Line 113 and 133 (pass-through to `build_sidebar_view_with_spec`):
```rust
state.armed_building_type(),
```
→
```rust
state.targeting_mode.as_ref(),
```

**Step 7: Update existing `build_sidebar_view` test calls**

In `src/sidebar/sidebar_view.rs::tests` (line 425+), every call to
`build_sidebar_view(..., None, ...)` keeps `None` (still typechecks as
`Option<&TargetingMode>`). No behavioral change.

**Step 8: Verify**

Run: `cargo check`
Expected: PASS.

Run: `cargo test sidebar::`
Expected: existing sidebar tests pass.

**Step 9: Commit**

Message: `sidebar: build_sidebar_view armed parameter takes Option<&TargetingMode>`

---

### Task 8: `hit_test` branching on `is_superweapon` + tests

**Why:** Cameo-click dispatch — the hot path that turns a SW cameo
click into a `SidebarAction`. Required by Task 10 (apply_sidebar_action).

**Files:**
- Modify: `src/sidebar/mod.rs:293-349` (`hit_test`)
- Modify: `src/sidebar/mod.rs:352-365` (`#[cfg(test)] mod tests`)

**Pattern:** Sibling branch to existing build-cameo dispatch.

**Step 1: Replace `hit_test` body**

In `src/sidebar/mod.rs`, replace the `for item in &view.items` loop in
`hit_test` (lines 304-325) with:

```rust
    for item in &view.items {
        if item.rect.contains(x, y) {
            return hit_test_item(item, right_click);
        }
    }
```

Add a helper function above `hit_test`:

```rust
fn hit_test_item(item: &SidebarItem, right_click: bool) -> SidebarAction {
    if right_click {
        // SW cameos have no queue → right-click does nothing.
        if item.is_superweapon {
            return SidebarAction::None;
        }
        // Build cameo right-click: cancel one queued (or ready) item.
        return if item.queued_count > 0 || item.is_ready {
            SidebarAction::CancelBuild(item.type_id.clone())
        } else {
            SidebarAction::None
        };
    }
    // Left-click branch.
    if item.is_superweapon {
        if !item.is_ready {
            return SidebarAction::None;
        }
        return if item.is_armed {
            SidebarAction::ClearSuperWeaponMode
        } else {
            // Section name (unique). Fall back to display_name for safety;
            // they currently coincide for SW views.
            let section = item
                .super_weapon_section
                .clone()
                .unwrap_or_else(|| item.display_name.clone());
            SidebarAction::ArmSuperWeapon(section)
        };
    }
    // Build cameo branch (unchanged).
    if item.is_ready {
        if item.is_armed {
            SidebarAction::ClearPlacementMode
        } else {
            SidebarAction::ArmPlacement(item.type_id.clone())
        }
    } else if item.enabled {
        SidebarAction::BuildType(item.type_id.clone())
    } else {
        SidebarAction::None
    }
}
```

**Step 2: Add hit_test tests**

Append to `#[cfg(test)] mod tests` in `src/sidebar/mod.rs`:

```rust
    use super::SidebarItem;
    use super::Rect;
    use crate::sim::production::ProductionCategory;

    fn make_sw_item(is_ready: bool, is_armed: bool) -> SidebarItem {
        SidebarItem {
            rect: Rect { x: 0.0, y: 0.0, w: 60.0, h: 48.0 },
            type_id: "INTICON".to_string(),
            display_name: "LightningStormSpecial".to_string(),
            cost: None,
            has_cameo_art: true,
            queue_category: ProductionCategory::Defense,
            enabled: true,
            progress: if is_ready { 1.0 } else { 0.5 },
            queued_count: 0,
            is_building_this_type: !is_ready,
            is_ready,
            is_armed,
            is_superweapon: true,
            super_weapon_section: Some("LightningStormSpecial".to_string()),
        }
    }

    #[test]
    fn sw_ready_left_click_arms() {
        let item = make_sw_item(true, false);
        let action = super::hit_test_item(&item, false);
        assert_eq!(
            action,
            super::SidebarAction::ArmSuperWeapon("LightningStormSpecial".to_string())
        );
    }

    #[test]
    fn sw_ready_armed_left_click_clears() {
        let item = make_sw_item(true, true);
        let action = super::hit_test_item(&item, false);
        assert_eq!(action, super::SidebarAction::ClearSuperWeaponMode);
    }

    #[test]
    fn sw_charging_left_click_does_nothing() {
        let item = make_sw_item(false, false);
        let action = super::hit_test_item(&item, false);
        assert_eq!(action, super::SidebarAction::None);
    }

    #[test]
    fn sw_right_click_does_nothing() {
        for ready in [false, true] {
            for armed in [false, true] {
                let item = make_sw_item(ready, armed);
                let action = super::hit_test_item(&item, true);
                assert_eq!(action, super::SidebarAction::None,
                    "ready={} armed={}", ready, armed);
            }
        }
    }
```

**Step 3: Verify**

Run: `cargo test sidebar::tests::sw_`
Expected: 4 tests pass.

Run: `cargo test sidebar::`
Expected: all sidebar tests pass.

**Step 4: Commit**

Message: `sidebar: hit_test branches ArmSuperWeapon / ClearSuperWeaponMode`

---

### Task 9: `sync_armed_building_placement` → `sync_targeting_mode`

**Why:** Per-frame validation of the armed state must also handle
`SuperWeapon` mode — clearing it when the SW becomes inactive (granting
building destroyed) or not-ready.

**Files:**
- Modify: `src/app_sidebar_render.rs:148-164` (the function)
- Modify: `src/app_sidebar_render.rs:48-53` (caller)
- Modify: `src/app_render_tests.rs:7, 380-422` (import + tests)

**Pattern:** Extend existing `still_valid` check to include the SW
variant.

**Step 1: Rename and extend the function**

Note on `SuperWeaponView`: it has `is_ready` and `is_online`, NOT
`is_active`. Views are only emitted for `is_active=true` instances per
the filter at `src/sim/superweapon/mod.rs:156-158`, so a view existing
in `super_weapons` already implies the SW is active. The match arm
checks `is_ready` only; if the granting building is destroyed the SW
deactivates and the view disappears entirely, making `iter().any()`
return `false` → still_valid=false → cleared.

Replace the entire `sync_armed_building_placement` function in
`src/app_sidebar_render.rs`:

```rust
pub(crate) fn sync_targeting_mode(
    targeting_mode: &mut Option<crate::app_types::TargetingMode>,
    building_placement_preview: &mut Option<crate::sim::production::BuildingPlacementPreview>,
    ready_buildings: &[production::ReadyBuildingView],
    super_weapons: &[crate::sim::superweapon::SuperWeaponView],
    interner: Option<&crate::sim::intern::StringInterner>,
) {
    let still_valid = match targeting_mode.as_ref() {
        None => true,
        Some(crate::app_types::TargetingMode::BuildingPlacement(armed)) => {
            ready_buildings.iter().any(|ready| {
                interner.map_or(false, |i| {
                    i.resolve(ready.type_id).eq_ignore_ascii_case(armed)
                })
            })
        }
        Some(crate::app_types::TargetingMode::SuperWeapon(section)) => {
            super_weapons.iter().any(|sw| {
                sw.is_ready && sw.display_name.eq_ignore_ascii_case(section)
            })
        }
    };
    if !still_valid {
        *targeting_mode = None;
        *building_placement_preview = None;
    }
}
```

**Step 2: Update caller in `current_sidebar_view`**

In `src/app_sidebar_render.rs:48-53`, replace:
```rust
sync_armed_building_placement(
    &mut state.armed_building_placement,
    &mut state.building_placement_preview,
    &ready_buildings,
    state.simulation.as_ref().map(|s| &s.interner),
);
```
with:
```rust
sync_targeting_mode(
    &mut state.targeting_mode,
    &mut state.building_placement_preview,
    &ready_buildings,
    &sw_views,
    state.simulation.as_ref().map(|s| &s.interner),
);
```

**Note on ordering:** the call must come AFTER `sw_views` is built.
Currently `sw_views` is built at line 95-100, before the existing
`sync_armed_building_placement` call at line 48. **The call needs to
move below `sw_views` construction.** Move the entire `sync_*` block
(currently lines 48-53) to immediately after the `sw_views` assignment
(currently line 100). The data dependency is straightforward: the sync
call needs `ready_buildings` (built at line 35) and `sw_views`
(built at line 95-100).

**Step 3: Rewrite tests in `app_render_tests.rs`**

Line 7 — update import:
```rust
use crate::app_sidebar_render::sync_targeting_mode;
```

Lines 385-402 — `test_ready_buildings_do_not_auto_arm_placement`:
```rust
#[test]
fn test_ready_buildings_do_not_auto_arm_placement() {
    let mut armed: Option<crate::app_types::TargetingMode> = None;
    let mut preview = None;
    let ready = vec![ReadyBuildingView {
        type_id: crate::sim::intern::test_intern("GAPOWR"),
        display_name: "Power Plant".to_string(),
        queue_category: crate::sim::production::ProductionCategory::Building,
    }];

    sync_targeting_mode(&mut armed, &mut preview, &ready, &[], None);

    assert!(armed.is_none(), "ready building should not auto-arm placement");
    assert!(preview.is_none());
}
```

Lines 404-422 — `test_invalid_armed_building_clears_when_not_ready`:
```rust
#[test]
fn test_invalid_armed_building_clears_when_not_ready() {
    let mut armed = Some(crate::app_types::TargetingMode::BuildingPlacement(
        "GAPOWR".to_string(),
    ));
    let mut preview = Some(crate::sim::production::BuildingPlacementPreview {
        type_id: crate::sim::intern::test_intern("GAPOWR"),
        rx: 5,
        ry: 5,
        width: 2,
        height: 2,
        valid: false,
        reason: None,
        cell_valid: vec![false; 4],
    });

    sync_targeting_mode(&mut armed, &mut preview, &[], &[], None);

    assert!(armed.is_none());
    assert!(preview.is_none());
}
```

**Step 4: Add three new tests for SW validation**

Append to the same test module:

```rust
#[test]
fn test_sw_armed_preserved_when_ready() {
    use crate::sim::superweapon::SuperWeaponView;
    let mut armed = Some(crate::app_types::TargetingMode::SuperWeapon(
        "LightningStormSpecial".to_string(),
    ));
    let mut preview = None;
    let sw = SuperWeaponView {
        type_id: crate::sim::intern::test_intern("LightningStormSpecial"),
        display_name: "LightningStormSpecial".to_string(),
        progress: 1.0,
        is_ready: true,
        is_online: true,
        sidebar_image: Some("INTICON".to_string()),
        kind: crate::rules::superweapon_type::SuperWeaponKind::LightningStorm,
    };

    sync_targeting_mode(&mut armed, &mut preview, &[], &[sw], None);

    assert!(armed.is_some(), "armed SW should be preserved while ready");
}

#[test]
fn test_sw_armed_cleared_when_not_ready() {
    use crate::sim::superweapon::SuperWeaponView;
    let mut armed = Some(crate::app_types::TargetingMode::SuperWeapon(
        "LightningStormSpecial".to_string(),
    ));
    let mut preview = None;
    let sw = SuperWeaponView {
        type_id: crate::sim::intern::test_intern("LightningStormSpecial"),
        display_name: "LightningStormSpecial".to_string(),
        progress: 0.5,
        is_ready: false,  // Charging, not yet ready.
        is_online: true,
        sidebar_image: Some("INTICON".to_string()),
        kind: crate::rules::superweapon_type::SuperWeaponKind::LightningStorm,
    };

    sync_targeting_mode(&mut armed, &mut preview, &[], &[sw], None);

    assert!(armed.is_none(), "armed SW should clear when not ready");
}

#[test]
fn test_sw_armed_cleared_when_view_gone() {
    let mut armed = Some(crate::app_types::TargetingMode::SuperWeapon(
        "LightningStormSpecial".to_string(),
    ));
    let mut preview = None;

    // No SW views — granting building destroyed.
    sync_targeting_mode(&mut armed, &mut preview, &[], &[], None);

    assert!(armed.is_none(), "armed SW should clear when view disappears");
}
```

**Step 5: Verify**

Run: `cargo test app_render_tests::test_`
Expected: 5 tests pass (2 existing rewritten + 3 new).

Run: `cargo build`
Expected: PASS.

**Step 6: Commit**

Message: `sidebar_render: rename sync_armed_building_placement to sync_targeting_mode + SW validation`

---

### Task 10: `apply_sidebar_action` handlers for new variants

**Why:** Wires the SidebarAction variants into AppState mutations.
Required by Task 12 (which depends on the action being handled).

**Files:**
- Modify: `src/app_input.rs:236-278` (`apply_sidebar_action`)

**Pattern:** Sibling arms to existing `ArmPlacement` / `ClearPlacementMode`.

**Step 1: Add the two new arms**

In `src/app_input.rs:236-278`, locate the `apply_sidebar_action` match.
Add two new arms after `SidebarAction::ClearPlacementMode`:

```rust
        SidebarAction::ClearPlacementMode => {
            state.targeting_mode = None;
            state.building_placement_preview = None;
        }
        SidebarAction::ArmSuperWeapon(section) => {
            state.targeting_mode = Some(
                crate::app_types::TargetingMode::SuperWeapon(section),
            );
            // Mutual exclusion: clear any pending building-placement preview.
            state.building_placement_preview = None;
            log::info!("SuperWeapon armed: type={}", state.armed_super_weapon_type().unwrap_or(""));
        }
        SidebarAction::ClearSuperWeaponMode => {
            state.targeting_mode = None;
            log::info!("SuperWeapon targeting cleared");
        }
```

If Task 3 added temporary stub arms, replace them with the above.

**Step 2: Verify**

Run: `cargo check`
Expected: PASS, no non-exhaustive match warning on `SidebarAction`.

**Step 3: Commit**

Message: `app_input: handle ArmSuperWeapon / ClearSuperWeaponMode actions`

---

### Task 11: Add SW cursor branch in `current_cursor_feedback_kind`

**Why:** Player feedback — when targeting mode is SuperWeapon and the
cursor is over the tactical map, show the per-SW reticle from
`Action=` → `CursorId`.

**Files:**
- Modify: `src/app_cursor.rs:17-42` (`current_cursor_feedback_kind` —
  insert a new branch after edge-scroll/sidebar/minimap, before
  building-placement)

**Pattern:** Sibling branch to existing building-placement at
`src/app_cursor.rs:33-42`.

**Step 1: Insert the SW branch**

In `src/app_cursor.rs`, in `current_cursor_feedback_kind`. The insertion
point is **between** `if current_sidebar_view_hit(state) { return None; }`
(line 30-32) and `if let Some(preview) = state.building_placement_preview...`
(line 33).

Add:

```rust
    // Superweapon targeting cursor takes precedence over building
    // placement. Returns None over sidebar/minimap (the chrome hit checks
    // already short-circuited above), so the SW reticle only renders on
    // the tactical map.
    if let Some(section) = state.armed_super_weapon_type() {
        let cursor_id = state
            .rules
            .as_ref()
            .and_then(|r| r.super_weapon(section))
            .and_then(|sw| sw.action.as_deref())
            .and_then(super_weapon_cursor_id)
            .unwrap_or(CursorId::Default);
        return Some(CursorFeedbackKind::SuperWeaponTarget(cursor_id));
    }
```

**Step 2: Verify**

Run: `cargo check`
Expected: PASS. `super_weapon_cursor_id` is in the same file (Task 2);
`CursorId` and `CursorFeedbackKind` are imported via the existing `use`
block at the top.

If imports are missing, add to the file's `use` lines:
```rust
use crate::app_types::CursorId; // already present
```

Run: `cargo test app_cursor::`
Expected: existing tests pass; Task 2 tests still pass.

**Step 3: Commit**

Message: `app_cursor: SW targeting reticle from Action= INI string`

---

### Task 12: `launch_super_weapon_at_cursor` in `app_commands.rs`

**Why:** Builds and emits `Command::LaunchSuperWeapon`. Required by
Task 13 (input wiring).

**Critical UI guard.** The same click that arms the SW (PRESS on the
cameo) also produces a RELEASE event on the cameo. The release handler
in `handle_mouse_input` (Task 13) checks `armed_super_weapon_type()` —
which is now Some — and would call this function with the cursor still
over the sidebar panel. Without a guard, this fires the SW at a bogus
off-map cell (sim's `Command::LaunchSuperWeapon` only validates
`is_active && is_ready`, NOT the cell). The leading
sidebar/minimap guard returns early WITHOUT clearing `targeting_mode`,
so arming persists for the next real tactical-map click — mirroring how
the building-placement preview-validity check protects placement.

**Files:**
- Modify: `src/app_cursor.rs:492` — promote `current_sidebar_view_hit`
  from `fn` to `pub(crate) fn`.
- Modify: `src/app_commands.rs` — append `launch_super_weapon_at_cursor`
  after `place_ready_building_at_cursor`.

**Pattern:** Sibling function to `place_ready_building_at_cursor` at
line 211-281.

**Step 1: Bump `current_sidebar_view_hit` visibility**

In `src/app_cursor.rs:492`, change:
```rust
fn current_sidebar_view_hit(state: &AppState) -> bool {
```
to:
```rust
pub(crate) fn current_sidebar_view_hit(state: &AppState) -> bool {
```

**Step 2: Append the launch function**

In `src/app_commands.rs`, after `place_ready_building_at_cursor` (around
line 281), add:

```rust
/// Schedule `Command::LaunchSuperWeapon` at the current cursor cell.
///
/// `section` is the SW INI section name (e.g., "LightningStormSpecial").
///
/// Returns early WITHOUT clearing `targeting_mode` when the cursor is
/// over the sidebar or minimap — this matters for the release of the
/// arming click itself, which lands on the cameo. Leaving the mode armed
/// lets the next real tactical-map click fire the SW. On a real
/// tactical-map click, schedules the command and clears the mode. The
/// sim-side dispatch validates `is_active && is_ready` (the SW could
/// have de-readied between cursor-frame and click-frame); UI does not
/// duplicate that check.
pub(crate) fn launch_super_weapon_at_cursor(state: &mut AppState, section: &str) {
    // Guard: arming click's RELEASE lands on the cameo. Don't fire the SW
    // at a bogus off-map cell behind the sidebar panel; leave the mode
    // armed so the next real map click fires.
    if crate::app_sidebar_render::is_cursor_over_minimap(state)
        || crate::app_cursor::current_sidebar_view_hit(state)
    {
        return;
    }

    let owner: String = resolve_owner(state);
    let sw_type_id = intern_type(state, section);
    let (rx, ry) = crate::app_sim_tick::screen_point_to_world_cell(
        state, state.cursor_x, state.cursor_y,
    );
    schedule_command(
        state,
        &owner,
        Command::LaunchSuperWeapon {
            sw_type_id,
            target_rx: rx,
            target_ry: ry,
        },
    );
    state.targeting_mode = None;
    log::info!(
        "SuperWeapon launch queued: owner={} section={} cell=({}, {}) execute_tick>=current+{}",
        owner, section, rx, ry, state.configured_input_delay_ticks,
    );
}
```

Note: `Command::LaunchSuperWeapon` is already imported via the file's
top-level `use crate::sim::command::{Command, ...}`.

**Step 3: Verify**

Run: `cargo check`
Expected: PASS.

**Step 4: Commit**

Message: `app_commands: launch_super_weapon_at_cursor with sidebar/minimap guard`

---

### Task 13: Wire SW launch + cancel into `handle_mouse_input`

**Why:** Final wiring — clicks on the tactical map fire the SW; clicks
on UI chrome don't (already enforced by `handle_sidebar_mouse_input`
running first); right-click and Esc cancel both modes.

**Files:**
- Modify: `src/app_input.rs:44-158` (`handle_mouse_input`)

**Pattern:** Sibling branch to building-placement Left release at
line 61-64.

**Step 1: Update Left-press to suppress drag while targeting**

In `src/app_input.rs:44-56`, replace the `if state.armed_building_placement.is_some()`
check (already migrated to `state.armed_building_type().is_some()` in
Task 6) with a check that covers BOTH targeting modes:

```rust
        MouseButton::Left => {
            if btn_state.is_pressed() {
                if crate::app_sidebar_render::try_begin_minimap_drag(state) {
                    return;
                }
                if state.targeting_mode.is_some() {
                    return; // suppress selection drag while either targeting mode is active
                }
                state.selection_state.begin_drag(state.cursor_x, state.cursor_y);
            } else {
```

**Step 2: Update Left-release to dispatch SW launch first**

In `src/app_input.rs:56-64` (the `} else {` branch of Left-press), the
current code looks like:

```rust
            } else {
                if state.minimap_dragging {
                    state.minimap_dragging = false;
                    return;
                }
                if let Some(type_id) = state.armed_building_type().map(str::to_owned) {
                    place_ready_building_at_cursor(state, &type_id);
                    return;
                }
                ...
```

Insert the SW launch dispatch BEFORE the building-placement dispatch
(though they're mutually exclusive via `targeting_mode`, ordering reads
better):

```rust
            } else {
                if state.minimap_dragging {
                    state.minimap_dragging = false;
                    return;
                }
                if let Some(section) = state.armed_super_weapon_type().map(str::to_owned) {
                    crate::app_commands::launch_super_weapon_at_cursor(state, &section);
                    return;
                }
                if let Some(type_id) = state.armed_building_type().map(str::to_owned) {
                    place_ready_building_at_cursor(state, &type_id);
                    return;
                }
                ...
```

**Step 3: Verify right-click cancel covers both modes**

In `src/app_input.rs:146-155` (right-click branch). Task 6 already
migrated this to `state.targeting_mode.is_some()` — verify the body
is:

```rust
        MouseButton::Right if btn_state.is_pressed() => {
            if state.targeting_mode.is_some() {
                state.targeting_mode = None;
                state.building_placement_preview = None;
                return;
            }
            queue_selection_snapshot_command(state, Vec::new(), false);
        }
```

If Task 6 left it referencing `armed_building_placement`, fix per
above (this is a defensive check — Task 6 should have caught it).

**Step 4: Verify Esc handler covers both modes**

In `src/app_input.rs:286-309` (Escape branch in `handle_hotkey_pressed`).
Task 6 migrated this. Verify:

```rust
            } else if state.targeting_mode.is_some() {
                state.targeting_mode = None;
                state.building_placement_preview = None;
            }
```

**Step 5: Verify**

Run: `cargo check`
Expected: PASS.

Run: `cargo test`
Expected: full suite passes (1588 sim tests + UI tests).

**Step 6: Commit**

Message: `app_input: wire SW launch on left-click; right-click and Esc cancel both modes`

---

### Task 14: Full integration verification

**Why:** Confirm the pipeline matches gamemd.exe behavior end-to-end
per the parity ledger.

**Files:** none modified (verification only).

**Verify in-game:**

1. **Charged SW cameo click → reticle.** Start a skirmish. Build a
   Battle Lab + Soviet Radar to grant Lightning Storm (or any granting
   building for the side you pick). Wait for the SW to charge fully
   (READY appears on the cameo). Click the cameo — the cursor should
   change to the Lightning Storm reticle (cursor frame 279 in mouse.sha).
2. **Reticle moves with cursor; sprite doesn't change on hover targets.**
   Move the cursor over friendly units, enemy units, shroud, water,
   buildings. Cursor should stay the same reticle in every case.
3. **Reticle does NOT show over sidebar.** Move the cursor onto the
   sidebar. The cursor should revert to the default UI cursor (the
   sidebar hit short-circuits in `current_cursor_feedback_kind`).
4. **Left-click on tactical fires.** Click any cell on the tactical
   map. The targeting mode should clear, the cursor should revert to
   default, and the sim should produce visible Lightning Storm effects
   (clouds gather → strike → recharge cycle).
5. **Sidebar cameo flips to charging after fire.** Watch the cameo
   GCLOCK2 overlay restart from 0 progress.
6. **Right-click cancels.** Re-arm the SW (wait for next charge).
   Click cameo to arm, then right-click anywhere on the tactical map.
   The targeting mode should clear without firing.
7. **Esc cancels.** Re-arm. Press Esc. Targeting clears without firing.
8. **Charging cameo click does nothing.** During the charge cycle,
   click the cameo. Nothing happens (no log, no cursor change, no
   command).
9. **Right-click on cameo does nothing.** During charge OR ready,
   right-click the cameo. Nothing happens.
10. **Toggle: click ready cameo → arm; click same cameo → clear.**
    Confirm the second click on the same cameo cancels arming.
10a. **Arming click does NOT self-fire the SW.** This is the critical
    regression test for the sidebar/minimap guard in
    `launch_super_weapon_at_cursor`. Charge a Lightning Storm. Click
    its cameo (single click — press and release both on the cameo).
    Verify: the cursor changes to the Lightning Storm reticle (arming
    succeeded), and the SW has NOT fired (no clouds, no charge reset,
    no log line "SuperWeapon launch queued"). The SW must remain armed
    until the player clicks somewhere on the tactical map.
11. **Mutual exclusion with building placement.** Build a Power Plant.
    When ready, click its cameo to arm placement (foundation ghost
    follows cursor). Without placing, click a charged SW cameo. The
    foundation ghost should vanish; the SW reticle should take over.
12. **Auto-cancel on building destroyed.** Arm Lightning Storm. Sell
    or destroy the granting building (Battle Lab + Soviet Radar pair —
    the radar that grants the SW). The targeting mode should clear
    automatically.
13. **Per-SW cursor sprite parity.** Repeat for at least 3 different
    SW types (e.g., Iron Curtain, Paradrop, Force Shield) and confirm
    each has its distinct reticle.

**Sim-side regression:** Run `cargo test`. All 1588 existing sim tests
plus the new UI tests should pass.

**Commit:** none (verification step).

If any verification fails, file the deviation and decide whether to
fix immediately or accept as a documented parity drift.

---

## Sources & References

- **Design doc:** docs/plans/2026-05-06-sw-targeting-pipeline-design.md
- **Ghidra reports:**
  - ra2-rust-game-docs/SUPERCLASS_SYSTEM_GHIDRA_REPORT.md
  - ra2-rust-game-docs/SUPERWEAPON_LAUNCH_HANDLERS_REPORT.md
  - ra2-rust-game-docs/SUPERWEAPON_SYSTEM_CONSOLIDATED_REPORT.md
- **gamemd.exe addresses (kept here, not in code comments):**
  - SuperWeaponTypeClass+0xBC — Action= field
  - SuperClass+0x60 — IsPresent guard flag
  - String table at 0x008425C0 — 12-entry SW Type enum
  - SuperClass::Launch at 0x006CC390 — sim-side dispatch ref
  - Cursor frame ranges in render/cursor_atlas.rs:250-340 (already
    extracted, no addresses needed in code)
- **INI keys driving behavior (rulesmd.ini):**
  - `[SWType] Type=` → SuperWeaponKind enum (already parsed)
  - `[SWType] Action=` → cursor sprite (this plan adds the mapping)
  - `[SWType] SidebarImage=` → cameo SHP (already wired)
  - `[SWType] RechargeTime=` → charge timer (already sim-side)
- **Related code:**
  - sim-side dispatch: src/sim/world/world_commands.rs:917-1020
  - SW instance state: src/sim/superweapon/mod.rs
  - SW type parser: src/rules/superweapon_type.rs
  - SW view builder: src/sim/superweapon/mod.rs:146-174
  - Cursor atlas: src/render/cursor_atlas.rs:250-340
- **Repo pattern mirrored:** building placement (the structural
  template) — src/app_input.rs (input flow),
  src/app_commands.rs:211-281 (place_ready_building_at_cursor),
  src/app_sidebar_render.rs:148-164 (sync), src/app_cursor.rs:33-42
  (cursor branch), src/app_sim_tick.rs:750-797 (per-tick preview).
