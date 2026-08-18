# GI Quick-Wins A — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> Tasks 1-3 are commit unit "data parsing"; Tasks 4-7 are commit unit "sound
> wiring"; Tasks 8-9 are commit unit "doc cleanup".

**Goal:** Add 5 unparsed `[E1]` sound keys to `ObjectType`, wire `CrushSound` +
`DieSound` emission on the existing crush-kill path so today's silent crush
kills produce a squish + cry, clean up 3 stale findings in the GI report.

**Architecture:** Mirrors the existing `DieSound` flow — INI key → `ObjectType`
field → `SimSoundEvent` variant → app-side translation to `AudioEvent`. Adds
two new params (`&RuleSet`, `&mut Vec<SimSoundEvent>`) to
`tick_movement_with_grids`, matching the pattern in `combat/mod.rs`. Pure-data
sound bus → no determinism impact.

**Design Doc:** [docs/plans/2026-05-04-gi-quickwins-a-design.md](2026-05-04-gi-quickwins-a-design.md)

---

## Grounding Summary

- **ra2-rust-game-docs/**: `GI_GHIDRA_REPORT.md` (just-completed Phase 1-3,
  HIGH confidence). §4 INI Keys table lists all 5 target keys with their type
  offsets. Phase 3 P3.1 corrected `+0xEAD = Cyborg` and `+0xEBC = Fearless`
  via xref evidence. CrushSound is in Section A of the [E1] dump
  (`CrushSound=InfantrySquish`).
- **Ghidra**: Sound key parsing happens in `InfantryTypeClass::ReadINI @ 0x005240A0`
  (DeploySound→0xEA4, UndeploySound→0xEA8) and `TechnoTypeClass::ReadINI @
  0x00712170` (VoiceFeedback, VoiceSpecialAttack, CrushSound — TechnoType
  level keys per `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md`). Crush kill emission
  in gamemd: `UnitClass::OnEnterCell_Triggers @ 0x00744720` calls `RecordKill`
  which does NOT play CrushSound directly — playback happens via the warhead
  detonation path on the death animation. For our purposes, **gamemd plays
  CrushSound when the victim's CrushSound INI key is set and the kill source
  is a crush kill** — so emitting a `EntityCrushed` sound event on our crush
  path matches observed behavior.
- **Repo pattern (DieSound, the perfect template)**:
  - Field decl: `src/rules/object_type.rs:217` (`die_sound: Option<String>`)
  - Parse: `src/rules/object_type.rs:670` (`section.get("DieSound")...`)
  - SimSoundEvent variant: `src/sim/world/mod.rs:94-99` (`EntityDied`)
  - Combat death emit: `src/sim/combat/mod.rs:1311`
  - App translation: `src/app_sim_tick.rs:295-302` (`EntityDied → EntityDestroyed`)
  - AudioEvent: `src/audio/events.rs:50` (`EntityDestroyed`)
- **INI keys (verified in `ini/rulesmd.ini` [E1] section per Phase 1 report)**:
  `VoiceFeedback=GIFear`, `VoiceSpecialAttack=GIMove`,
  `CrushSound=InfantrySquish`, `DeploySound=GIDeploy`, `UndeploySound=GIUndeploy`.
  None currently parsed by `src/rules/object_type.rs`.
- **Blockers / unknowns**:
  - `tick_movement_with_grids` signature lacks `&RuleSet` and
    `&mut Vec<SimSoundEvent>`. Need to add both — the mod.rs wrapper
    `tick_movement` at line 225 also forwards. Caller is
    `world/mod.rs:1033` which has both available on `&mut self`.

## Key Technical Decisions

- **Two events per crush** (`EntityCrushed` + `EntityDied`) — Confidence: HIGH.
  Source: gamemd plays both audio cues per Phase 3 §P3.8; combat death already
  pushes EntityDied via `combat/mod.rs:1311` so symmetry is preserved.
- **Add 2 new params (`&RuleSet`, `&mut Vec<SimSoundEvent>`) to
  `tick_movement_with_grids`** — Confidence: HIGH. Source: matches the existing
  pattern in `combat/mod.rs` where the sink + rules are passed in. No new
  abstraction.
- **`InternedId` for sound keys in SimSoundEvent** — Confidence: HIGH. Source:
  `EntityDied { die_sound_id: InternedId }` is the established convention.
- **Field naming snake_case** (`crush_sound`, `voice_feedback`, etc.) —
  Confidence: HIGH. Source: existing `voice_select`, `voice_move`, `die_sound`
  fields use the same convention.
- **Skip emit when `Option<String>` is `None`** — Confidence: HIGH. Source:
  matches DieSound behavior (`combat/mod.rs:394` guards with
  `if let Some(ref die_sound) = obj.die_sound`).

## Open Questions

### Resolved During Planning

- **Q: Does `tick_movement_with_grids` have access to RuleSet/sound_events?**
  Resolved: NO — add as new params. Caller `world/mod.rs:1033` has both on
  `&mut self`. Wrapper `movement::mod.rs:225` also needs forwarding update.
- **Q: Should `MovementTickStats` carry the kill data instead of taking sink as
  a param?** Resolved: NO — `MovementTickStats` is `Copy`-derived. Adding a Vec
  breaks Copy. Sink-as-param matches combat's pattern.

### Deferred to Implementation

- **Q: Do existing test-helper struct literals using `ObjectType { ... }` need
  updating beyond the two found in grounding?** Deferred: discover during
  Task 2 by `cargo build` and fix all compilation errors. The two known sites
  are `src/sim/movement/locomotor_tests.rs:49` and
  `src/sim/movement/teleport_movement.rs:284`.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/object_type.rs` | Add 5 fields + 5 parse lines |
| Modify | `src/sim/world/mod.rs` | Add `SimSoundEvent::EntityCrushed` variant |
| Modify | `src/audio/events.rs` | Add `AudioEvent::EntityCrushed` variant |
| Modify | `src/app_sim_tick.rs` | Translate `EntityCrushed → AudioEvent::EntityCrushed` |
| Modify | `src/sim/movement/movement_tick.rs` | Emit crush sound events; new params |
| Modify | `src/sim/movement/mod.rs` | Forward new params in `tick_movement` wrapper |
| Modify | `src/sim/world/mod.rs` | Pass new args to `movement::tick_movement_with_grids` at line 1033 |
| Modify | `src/sim/movement/locomotor_tests.rs:49` | Add `: None` for 5 new fields in test helper |
| Modify | `src/sim/movement/teleport_movement.rs:284` | Same |
| Modify | `docs/research/GI_GHIDRA_REPORT.md` | Stale-findings cleanup + new appendix |

## Interface Changes

- **`tick_movement_with_grids`** gains 2 params:
  `rules: &RuleSet`, `sound_events: &mut Vec<SimSoundEvent>`. Caller in
  `world/mod.rs:1033` passes them. The wrapper `movement::tick_movement` in
  `mod.rs:225` also forwards.
- **`SimSoundEvent`** gains 1 variant `EntityCrushed`. Exhaustive matches in
  `app_sim_tick.rs` need a new arm.
- **`AudioEvent`** gains 1 variant `EntityCrushed`. Exhaustive matches in
  audio backend (anywhere `AudioEvent` is exhaustively matched) need a new arm.
- **`ObjectType`** gains 5 `Option<String>` fields. Default-initializer test
  helpers need `: None` lines added.

## Sim Checklist

- [x] All math uses `fixed`-point — N/A (no math added)
- [x] New state included in deterministic state hash — `SimSoundEvent` is
  pure data drained per frame, NOT in state hash. No change.
- [x] No dependencies on render/ui/sidebar/audio/net — sim emits SimSoundEvent;
  app layer translates to AudioEvent. Boundary preserved.
- [x] Tick ordering impact noted — none. Crush-kill loop is at end of
  `tick_movement_with_grids`; sound emit happens before `entities.remove(victim_id)`.
- [x] BTreeMap iteration order considered — N/A. Iterating `crush_kills: Vec<u64>`
  not a BTreeMap.

## Risk Areas

- **Highest blast radius**: Adding 2 params to `tick_movement_with_grids`. One
  call site (`world/mod.rs:1033`), one wrapper (`mod.rs:225`). Compile-fail
  surfaces all callers.
- **Test-helper compile breaks**: 2 known struct-literal init sites
  (locomotor_tests.rs, teleport_movement.rs). `cargo build` after Task 1 will
  surface any others — Task 2 handles them.
- **Existing crush tests** (`bump_crush.rs`): no sound assertions in current
  tests, so they pass unchanged. Verified during grounding.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 6 | Crush emits both `EntityCrushed` (squish) AND `EntityDied` (cry) | gamemd plays both audio cues simultaneously when a tank crushes a GI; emitting only one is audibly wrong | Task 7 unit test asserts both events present with correct interned IDs; visual confirmation in-game (tank crushes GI → hear "InfantrySquish" + "GIDie") |
| Task 6 | Sound emit happens BEFORE `entities.remove(victim_id)` | After remove, `entities.get(victim_id)` returns None; type_ref lookup fails; sounds drop silently | Task 7 negative test: skip emit when crush_sound is None must NOT skip die_sound emission and vice versa |

---

## Tasks

### Task 1: Add 5 sound fields to `ObjectType`

**Why:** Preserve INI source data so future slices (B = deploy state machine,
D = fear runtime) can wire these sounds without re-editing this file.

**Files:**
- Modify: `src/rules/object_type.rs:217-219` (struct decl)
- Modify: `src/rules/object_type.rs:670-671` (parse function)

**Pattern:** Mirror existing `voice_select` / `die_sound` field-and-parse rows.

**Step 1: Add struct fields after `move_sound` (line 219)**

After the existing line:
```rust
pub move_sound: Option<String>,
```
add:
```rust
pub voice_feedback: Option<String>,
pub voice_special_attack: Option<String>,
pub crush_sound: Option<String>,
pub deploy_sound: Option<String>,
pub undeploy_sound: Option<String>,
```

**Step 2: Add parse lines after `move_sound` (line 671)**

After the existing line:
```rust
move_sound: section.get("MoveSound").map(|s| s.to_string()),
```
add:
```rust
voice_feedback: section.get("VoiceFeedback").map(|s| s.to_string()),
voice_special_attack: section.get("VoiceSpecialAttack").map(|s| s.to_string()),
crush_sound: section.get("CrushSound").map(|s| s.to_string()),
deploy_sound: section.get("DeploySound").map(|s| s.to_string()),
undeploy_sound: section.get("UndeploySound").map(|s| s.to_string()),
```

**Step 3: Verify**

Run: `cargo build -p ra2_engine 2>&1 | head -40` (or the workspace name from
`Cargo.toml` — check first).

**Expected:** Compile errors at `ObjectType { ... }` literal-init sites.
The struct itself compiles. The errors point to test helpers that need updates.

### Task 2: Update test-helper `ObjectType` literal-init sites

**Why:** `ObjectType { ... }` literals require all fields named — adding 5
fields breaks two known test helpers and possibly more.

**Files:**
- Modify: `src/sim/movement/locomotor_tests.rs:49` (struct literal)
- Modify: `src/sim/movement/teleport_movement.rs:284` (struct literal)
- Modify: any additional sites surfaced by `cargo build`

**Pattern:** Mirror existing `die_sound: None` lines in the same struct literals.

**Step 1: At `locomotor_tests.rs:49`, find the existing `ObjectType { ... }` literal containing `die_sound: None,`. Add 5 lines after `die_sound: None,`:**

```rust
die_sound: None,
move_sound: None,
voice_feedback: None,
voice_special_attack: None,
crush_sound: None,
deploy_sound: None,
undeploy_sound: None,
```

(Match the existing indentation.)

**Step 2: Same edit at `teleport_movement.rs:284`.**

**Step 3: Verify**

Run: `cargo build -p ra2_engine 2>&1 | grep -E "missing field|expected struct" | head -20`

**Expected:** No "missing field" errors for the 5 new fields. If any other
struct-literal sites surface, edit them with the same 5 `: None` lines.

**Step 4: Final compile check**

Run: `cargo build 2>&1 | tail -10`

**Expected:** Workspace builds cleanly (warnings allowed; no errors).

### Task 3: Commit "rules: parse 5 missing infantry sound keys"

**Why:** Atomic commit — the parsing changes are self-contained and testable.

**Step 1: Stage**

Run:
```
git add src/rules/object_type.rs src/sim/movement/locomotor_tests.rs src/sim/movement/teleport_movement.rs
```

**Step 2: Commit**

```
git commit -m "$(cat <<'EOF'
rules: parse VoiceFeedback/VoiceSpecialAttack/CrushSound/DeploySound/UndeploySound on ObjectType

Adds five new Option<String> fields and matching ReadINI lines, mirroring
the existing voice_select/die_sound pattern. Wiring of CrushSound playback
follows in a separate commit; the other four await consumer state machines
(deploy state, fear runtime) in later slices.
EOF
)"
```

**Step 3: Verify**

Run: `git status` — expect "nothing to commit, working tree clean."

### Task 4: Add `SimSoundEvent::EntityCrushed` variant

**Why:** Define the new sound-event shape before any code emits it. Interface
ordering principle.

**Files:**
- Modify: `src/sim/world/mod.rs:87-130` (`SimSoundEvent` enum)

**Pattern:** Mirror `EntityDied` at lines 94-99.

**Step 1: After the existing `EntityDied { ... }` variant (lines 94-99), add:**

```rust
/// An entity was crushed by a vehicle — play CrushSound (the squish).
/// Crush kills also emit `EntityDied` for the death cry — these are
/// independent audio events that play together (matches gamemd).
EntityCrushed {
    crush_sound_id: InternedId,
    rx: u16,
    ry: u16,
},
```

**Step 2: Verify**

Run: `cargo build 2>&1 | grep -E "non-exhaustive|missing match arm" | head -20`

**Expected:** Possible non-exhaustive-match warnings at any `match` over
`SimSoundEvent`. Tasks 5 (app translation) will satisfy them. Build passes
even with exhaustive-match warnings as warnings — confirm no hard errors.

### Task 5: Add `AudioEvent::EntityCrushed` + app-side translation

**Why:** Complete the sim → audio bridge so the new sim event reaches the
audio backend.

**Files:**
- Modify: `src/audio/events.rs` (`AudioEvent` enum, near `EntityDestroyed`)
- Modify: `src/app_sim_tick.rs:295` (translation match arm)

**Pattern:** Mirror `EntityDestroyed` variant + its translation at
`app_sim_tick.rs:295-302`.

**Step 1: In `src/audio/events.rs`, after the `EntityDestroyed { ... }` variant (around line 50-58), add:**

```rust
/// An entity was crushed by a vehicle — play CrushSound (squish).
EntityCrushed {
    /// sound.ini ID from the entity's CrushSound= field.
    sound_id: String,
    /// Screen position of the sound source (for spatial audio).
    screen_pos: Option<(f32, f32)>,
},
```

**Step 2: In `src/app_sim_tick.rs:295`, find the existing `SimSoundEvent::EntityDied { die_sound_id, rx, ry } => { ... }` match arm. After it, add:**

```rust
SimSoundEvent::EntityCrushed { crush_sound_id, rx, ry } => {
    audio_events.push(AudioEvent::EntityCrushed {
        sound_id: sim.interner.resolve(crush_sound_id).to_string(),
        screen_pos: tactical.world_to_screen(rx, ry),
    });
}
```

(Match the surrounding indentation and the exact field/method names used by
the EntityDied arm — copy-paste from there and rename `die_sound_id` →
`crush_sound_id`, `EntityDestroyed` → `EntityCrushed`.)

**Step 3: Find any other exhaustive `match audio_event` site**

Run: `grep -rn "match.*AudioEvent\|AudioEvent::Entity" src/ | head -20`

For each exhaustive match arm, add a handler for `AudioEvent::EntityCrushed`
mirroring the `EntityDestroyed` arm. (In the audio backend, the playback
logic for crush-sound is identical to die-sound — both look up the sound by
ID and play it spatially.)

**Step 4: Verify**

Run: `cargo build 2>&1 | tail -10`

**Expected:** Workspace builds cleanly.

### Task 6: Extract `emit_crush_kill_sounds` helper + wire into `tick_movement_with_grids`

**Why:** Make the sound-emit logic isolated and testable without mocking
the 16-param tick function. The helper goes in `bump_crush.rs` to keep
crush-adjacent logic colocated.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs` (new helper near `collect_crush_victims`)
- Modify: `src/sim/movement/movement_tick.rs:298-313` (signature)
- Modify: `src/sim/movement/movement_tick.rs:895-903` (crush-kill loop)
- Modify: `src/sim/movement/mod.rs:223-228` (wrapper signature + forward call)
- Modify: `src/sim/world/mod.rs:1033` (caller passes new args)

**Pattern:** Mirror combat's `RuleSet`+`sink` parameter pattern. Helper is
a pure function — easy to unit-test.

**Step 1: Add the helper** to `src/sim/movement/bump_crush.rs`. After the
`collect_crush_victims` definition (roughly line 408), add:

```rust
/// Emit `EntityCrushed` (CrushSound) and `EntityDied` (DieSound) sound
/// events for a single crush victim. Each event is skipped if the
/// corresponding `ObjectType` field is `None`. Caller invokes BEFORE
/// removing the victim from the EntityStore so victim.position and
/// victim.type_ref are still valid.
///
/// Pure function — no entity mutation, no return value.
pub fn emit_crush_kill_sounds(
    victim: &crate::sim::game_entity::GameEntity,
    rules: &crate::rules::ruleset::RuleSet,
    interner: &mut crate::sim::intern::StringInterner,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
) {
    let rx = victim.position.rx;
    let ry = victim.position.ry;
    let type_str = interner.resolve(victim.type_ref).to_string();
    let Some(obj) = rules.object(&type_str) else {
        return;
    };
    if let Some(ref crush_sound) = obj.crush_sound {
        let id = interner.intern(crush_sound);
        sound_events.push(crate::sim::world::SimSoundEvent::EntityCrushed {
            crush_sound_id: id,
            rx,
            ry,
        });
    }
    if let Some(ref die_sound) = obj.die_sound {
        let id = interner.intern(die_sound);
        sound_events.push(crate::sim::world::SimSoundEvent::EntityDied {
            die_sound_id: id,
            rx,
            ry,
        });
    }
}
```

(If `interner.resolve` returns a borrowed `&str`, the `.to_string()` is a
cheap allocation needed because `rules.object(&type_str)` borrow conflicts
with the same interner used by `interner.intern(crush_sound)`. Verify the
borrow shape during implementation; if `resolve` returns an owned `String`
or `&str` that doesn't conflict, drop the `.to_string()`.)

**Step 2: Update `tick_movement_with_grids` signature** at `movement_tick.rs:298`.

After the existing `interner` parameter, add:
```rust
    rules: &crate::rules::ruleset::RuleSet,
    sound_events: &mut Vec<crate::sim::world::SimSoundEvent>,
```

Change the existing `interner: &crate::sim::intern::StringInterner` parameter
to `interner: &mut crate::sim::intern::StringInterner` (the helper needs
`&mut` to call `interner.intern`).

**Step 3: Update the wrapper** at `mod.rs:223-228`. Same parameter changes
(add 2 params, change interner to `&mut`); forward them in the call.

**Step 4: Update the caller** at `world/mod.rs:1033`. The existing call
passes `&self.interner` — change to `&mut self.interner` and add
`&self.rules, &mut self.sound_events` after it. Verify by inspection that
both `rules` and `sound_events` exist as fields on `Simulation`. (Both
should — the field name `sound_events` is referenced elsewhere; `rules` is
also a Simulation field.)

**Step 5: Modify the crush-kill loop** at `movement_tick.rs:895-903`.
Replace:
```rust
for &victim_id in &crush_kills {
    if let Some(victim) = entities.get_mut(victim_id) {
        victim.health.current = 0;
    }
    entities.remove(victim_id);
    stats.crush_kills = stats.crush_kills.saturating_add(1);
}
```
with:
```rust
for &victim_id in &crush_kills {
    // Emit sounds BEFORE entity mutation/removal so position + type_ref
    // are still valid on the victim.
    if let Some(victim) = entities.get(victim_id) {
        bump_crush::emit_crush_kill_sounds(victim, rules, interner, sound_events);
    }
    if let Some(victim) = entities.get_mut(victim_id) {
        victim.health.current = 0;
    }
    entities.remove(victim_id);
    stats.crush_kills = stats.crush_kills.saturating_add(1);
}
```

(If `bump_crush` is not already imported in `movement_tick.rs`, add a `use`
or the fully-qualified `crate::sim::movement::bump_crush::...` path.)

**Step 6: Update interner type elsewhere if needed**

Changing `interner: &StringInterner` to `&mut StringInterner` in the
function signature may cascade — every internal use of `interner` is
unchanged for read calls (`interner.resolve`), but if any call inside
`tick_movement_with_grids` previously relied on shared borrow of `interner`,
the borrow checker will flag it. Resolve by re-borrowing (`&*interner`) or
restructuring the lifetime. Most likely no cascade needed.

**Step 7: Verify**

Run: `cargo build 2>&1 | tail -10`

**Expected:** Workspace builds cleanly. Any borrow-checker errors are
addressed by the Step 6 strategy.

### Task 7: Unit test the `emit_crush_kill_sounds` helper

**Why:** Lock in parity-critical behavior — both events fire on a normal
crush; each is independently skipped when its field is None.

**Files:**
- Modify: `src/sim/movement/bump_crush.rs` (new test module section)

**Pattern:** Inline `RuleSet::from_ini` matching `aircraft/attack_mission.rs:398`.
Synthesize a minimal `GameEntity` matching `bump_crush.rs:548` `infantry()` helper.

**Step 1: Add tests at the end of the existing `mod tests { ... }` in `bump_crush.rs`**

(After the existing `test_scatter_blocker_*` tests, before the closing brace
of the `mod tests` block.)

```rust
// -- emit_crush_kill_sounds tests --

#[cfg(test)]
mod crush_sound_emission {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::intern::StringInterner;
    use crate::sim::world::SimSoundEvent;

    fn build_test_rules(crush_sound: Option<&str>, die_sound: Option<&str>) -> RuleSet {
        // Compose a minimal rulesmd.ini that creates an [E1] InfantryType
        // with the requested CrushSound / DieSound keys (or omits them).
        let mut e1 = String::from("[E1]\nStrength=125\nArmor=none\nSpeed=4\n");
        if let Some(s) = crush_sound {
            e1.push_str(&format!("CrushSound={}\n", s));
        }
        if let Some(s) = die_sound {
            e1.push_str(&format!("DieSound={}\n", s));
        }
        let ini_text = format!(
            "[InfantryTypes]\n0=E1\n[VehicleTypes]\n[AircraftTypes]\n[BuildingTypes]\n{}",
            e1
        );
        let ini = IniFile::from_str(&ini_text);
        RuleSet::from_ini(&ini).expect("test rules build")
    }

    fn build_victim(interner: &mut StringInterner, rx: u16, ry: u16) -> crate::sim::game_entity::GameEntity {
        // Mirror bump_crush::tests::infantry() but with type_ref pointing at "E1".
        let inf = infantry(1, rx, ry, 2);
        let e1_ref = interner.intern("E1");
        let mut victim = inf;
        victim.type_ref = e1_ref;
        victim
    }

    #[test]
    fn emits_both_when_both_keys_set() {
        let rules = build_test_rules(Some("InfantrySquish"), Some("GIDie"));
        let mut interner = StringInterner::new();
        let victim = build_victim(&mut interner, 5, 5);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert_eq!(events.len(), 2, "expected 2 events, got {:?}", events);
        // Verify EntityCrushed is present with correct sound id.
        let crushed = events.iter().find_map(|e| match e {
            SimSoundEvent::EntityCrushed { crush_sound_id, rx, ry } => Some((*crush_sound_id, *rx, *ry)),
            _ => None,
        });
        let (cid, crx, cry) = crushed.expect("missing EntityCrushed");
        assert_eq!(interner.resolve(cid), "InfantrySquish");
        assert_eq!((crx, cry), (5, 5));

        let died = events.iter().find_map(|e| match e {
            SimSoundEvent::EntityDied { die_sound_id, rx, ry } => Some((*die_sound_id, *rx, *ry)),
            _ => None,
        });
        let (did, drx, dry) = died.expect("missing EntityDied");
        assert_eq!(interner.resolve(did), "GIDie");
        assert_eq!((drx, dry), (5, 5));
    }

    #[test]
    fn skips_crush_when_field_is_none_emits_die() {
        let rules = build_test_rules(None, Some("GIDie"));
        let mut interner = StringInterner::new();
        let victim = build_victim(&mut interner, 7, 9);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert_eq!(events.len(), 1);
        match &events[0] {
            SimSoundEvent::EntityDied { .. } => {}
            other => panic!("expected EntityDied, got {:?}", other),
        }
    }

    #[test]
    fn skips_die_when_field_is_none_emits_crush() {
        let rules = build_test_rules(Some("InfantrySquish"), None);
        let mut interner = StringInterner::new();
        let victim = build_victim(&mut interner, 3, 4);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert_eq!(events.len(), 1);
        match &events[0] {
            SimSoundEvent::EntityCrushed { .. } => {}
            other => panic!("expected EntityCrushed, got {:?}", other),
        }
    }

    #[test]
    fn no_events_when_both_none() {
        let rules = build_test_rules(None, None);
        let mut interner = StringInterner::new();
        let victim = build_victim(&mut interner, 1, 1);
        let mut events = Vec::new();

        emit_crush_kill_sounds(&victim, &rules, &mut interner, &mut events);

        assert!(events.is_empty(), "expected no events, got {:?}", events);
    }
}
```

(If `infantry()` helper is private to the parent `mod tests`, the inner
`mod crush_sound_emission` may need a different placement — try placing
the new tests directly at the bottom of `mod tests` (no nested module) so
they share scope with `infantry()` and `make_occ()`. If a `pub(super) fn`
is needed, use that. Match repo convention.)

**Step 2: Verify**

Run: `cargo test --lib emit_crush_kill_sounds 2>&1 | tail -30`

**Expected:** All 4 tests PASS. If a test fails:
- "missing EntityCrushed" → check Step 1 of Task 6 (helper logic)
- "expected 2 events, got 0" → check that `rules.object("E1")` returns Some
- Compile error on `RuleSet::from_ini` shape → check the actual signature
  in `src/rules/ruleset.rs` and adjust the build_test_rules helper.

### Task 8: Full regression test suite

**Why:** Confirm no existing test broke from signature changes, default-init
changes, or new match arms.

**Step 1: Run full test suite**

Run: `cargo test 2>&1 | tail -30`

**Expected:** All tests pass. If any fail, the failure is either:
- A test-helper struct literal we missed → add `: None` lines (per Task 2).
- A match arm we missed → add `EntityCrushed` arm.
- A regression we introduced → revisit Task 6 emit-order logic.

**Step 2: Run lints if applicable**

Run: `cargo clippy 2>&1 | tail -30`

**Expected:** No new warnings. If existing warnings are present, do NOT fix
them in this slice (out of scope per CLAUDE.md "no extra work").

### Task 9: Commit "audio: emit CrushSound + DieSound on crush kill"

**Why:** Atomic commit for the sound-wiring portion of this slice.

**Step 1: Stage**

Run:
```
git add src/sim/world/mod.rs src/audio/events.rs src/app_sim_tick.rs src/sim/movement/movement_tick.rs src/sim/movement/mod.rs
```

**Step 2: Commit**

```
git commit -m "$(cat <<'EOF'
audio: emit CrushSound + DieSound when a vehicle crushes infantry

Adds SimSoundEvent::EntityCrushed and AudioEvent::EntityCrushed variants;
wires both events from the deferred crush-kills loop in
tick_movement_with_grids. Crushed entities now produce both the squish
(CrushSound, e.g. InfantrySquish) and the death cry (DieSound, e.g. GIDie),
matching gamemd's two-cue audio.

Threads &RuleSet and &mut Vec<SimSoundEvent> into tick_movement_with_grids
to enable the lookup, mirroring the sink/rules pattern already used in
combat::tick_combat. Pure-data sound bus — no determinism impact.
EOF
)"
```

**Step 3: Verify**

Run: `git status` — expect clean.

### Task 10: Doc cleanup — flag stale findings in `GI_GHIDRA_REPORT.md`

**Why:** Three findings in the GI report describe "Rust bugs" or "missing"
that are actually already implemented. Future readers will waste time
re-investigating them.

**Files:**
- Modify: `docs/research/GI_GHIDRA_REPORT.md`

**Step 1: Update §6 (Phase 1 Rust Implementation Status)**

Find the "Sub-cell allocator" / "IronCurtain" / "DieSound" status rows.
Change their status from `BUG`/`MISSING`/`PARTIAL` to `IMPLEMENTED` and
add a `(verified at <file:line>)` annotation:
- Sub-cell `[2,3,4]` — `src/sim/movement/bump_crush.rs:31`
- IronCurtain kills infantry — `src/sim/superweapon/iron_curtain.rs:57-60`
- DieSound parsing + emit — `src/rules/object_type.rs:217+670`,
  `src/sim/combat/mod.rs:1311`

**Step 2: Update the Phase-2 §"Phase 2 Updates to §6"  table**

Same — flag the rows that are now resolved.

**Step 3: Update "Final Implementation Status" table**

Same — sub-cell row should now read HIGH coverage, not BUG.

**Step 4: Add new appendix at the end of the report**

After the existing `# Recommended next step` section, add:

```markdown
---

## Verified-already-implemented (post-Phase-3 audit)

The Phase 1-3 dossier described three items as Rust gaps. A `/brainstorm`
audit on 2026-05-04 found they were already implemented; the original
findings were based on stale prior research docs:

- **Sub-cell allocator `[2, 3, 4]`** — `src/sim/movement/bump_crush.rs:31`.
  The Phase 1 report cited an older `INFANTRY_SUBCELL_POSITIONING.md` claim
  of `[0, 3, 4]`; the Rust constant has been correct for some time.
- **IronCurtain on infantry kills the GI** — `src/sim/superweapon/iron_curtain.rs:57-60`.
  The handler explicitly zeroes infantry HP rather than applying invulnerability,
  with a comment matching the binary override behavior (P3.5).
- **DieSound parsing + emit** — `src/rules/object_type.rs:217+670` (parse),
  `src/sim/combat/mod.rs:1311` (emit on combat death). DieSound on **crush** kills
  was added by GI Quick-Wins A (2026-05-04) along with `CrushSound`.

This appendix exists so future readers of this report do not re-investigate
already-implemented mechanics.
```

**Step 5: Verify**

Open the file and visually scan to confirm the three target rows now read
"IMPLEMENTED" and the appendix is present.

### Task 11: Commit "docs: GI report — flag verified-already-implemented findings"

**Step 1: Stage**

Note: this commit is in the docs repo, not the main repo. The path
`docs/research/` is a separate location.
Check whether it's a git repo:

Run: `cd <local>/Documents/ra2-rust-game-docs && git status 2>&1 | head -5`

**If it's a git repo:** stage and commit there:
```
git add GI_GHIDRA_REPORT.md
git commit -m "docs: GI report — flag verified-already-implemented findings (sub-cell, IC, DieSound)"
```

**If it's NOT a git repo:** the file edit is preserved on disk but not
versioned. Note this to the user as an out-of-band edit.

**Step 2: Verify**

If git repo: `git log -1 --oneline` shows the new commit.
If not: open the doc file and confirm the appendix is present.

### Task 12: Manual in-game smoke test

**Why:** Verify the parity-critical audio cue actually plays. Unit tests
prove the sim emits the events; only running the game proves the audio
backend plays them spatially.

**Step 1: Build a debug binary**

Run: `cargo build` (debug, default profile).

**Step 2: Launch into a skirmish with at least one Crusher unit (Rhino)
and at least one GI**

Spawn a Rhino on one side and 5+ GIs on the other. Order the Rhino to drive
through the GIs.

**Step 3: Verify audio**

Listen for:
- `InfantrySquish` (the CrushSound — sounds like a wet thump) on each crush
- `GIDie` (a brief cry) on each crush
- Both should overlap, not sequence

If only one plays, the audio backend has a single-event-per-frame limit or
similar — investigate.

**Step 4: Side-by-side compare to gamemd.exe** (if the user has it set up)

Run the same scenario in original `gamemd.exe`. The crush sound + die cry
should sound identical to the Rust engine. Any audible difference is a
parity gap.

**Expected:** Crush + die sounds play on every crush, indistinguishable
from gamemd.exe.

---

## Sources & References

- **Design doc:** `docs/plans/2026-05-04-gi-quickwins-a-design.md`
- **Ghidra reports:**
  - `ra2-rust-game-docs/GI_GHIDRA_REPORT.md` (Phase 1 §4 INI keys, P3.1 bool
    name corrections, P3.8 voice playback chain)
  - `ra2-rust-game-docs/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md` (TechnoTypeClass
    sound key parsing)
- **gamemd.exe addresses:**
  - `InfantryTypeClass::ReadINI @ 0x005240A0` (DeploySound→0xEA4, UndeploySound→0xEA8)
  - `TechnoTypeClass::ReadINI @ 0x00712170` (CrushSound, VoiceFeedback, VoiceSpecialAttack)
  - `UnitClass::OnEnterCell_Triggers @ 0x00744720` (crush kill recorder)
- **INI keys** (`ini/rulesmd.ini` `[E1]`):
  - `CrushSound=InfantrySquish`
  - `DeploySound=GIDeploy`
  - `UndeploySound=GIUndeploy`
  - `VoiceFeedback=GIFear`
  - `VoiceSpecialAttack=GIMove`
- **Related code (the DieSound template — copy this exactly):**
  - `src/rules/object_type.rs:217` field decl
  - `src/rules/object_type.rs:670` parse line
  - `src/sim/world/mod.rs:94-99` SimSoundEvent::EntityDied
  - `src/sim/combat/mod.rs:394+1311` emit pattern
  - `src/app_sim_tick.rs:295-302` translation
  - `src/audio/events.rs:50-58` AudioEvent::EntityDestroyed
- **Pattern-mirror code:**
  - `src/sim/combat/mod.rs` — already takes `sink: &mut Vec<SimSoundEvent>`
    and `&RuleSet`; same pattern propagates to `tick_movement_with_grids`.
