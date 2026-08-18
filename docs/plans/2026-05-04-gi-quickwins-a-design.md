# GI Quick-Wins A — Sound-key parsing batch + CrushSound wiring

## Goal

Add the 5 unparsed `[E1]` sound keys to `ObjectType`, wire `CrushSound` (and
its companion `DieSound`) into the existing crush-kill path so today's silent
crush kills produce a squish + cry, and clean up three stale findings in the
GI research report.

## Architecture Context

**Sound flow today** (DieSound is the template):
1. `ObjectType` (`src/rules/object_type.rs`) holds 5 voice/sound `Option<String>`
   fields populated from INI: `voice_select`, `voice_move`, `voice_attack`,
   `die_sound`, `move_sound`. Field decls at lines 211-219; parse calls at
   lines 667-671.
2. Sim emits `SimSoundEvent` variants (enum at `src/sim/world/mod.rs:87-130`)
   into `sim.sound_events` — pure data, no audio dependency.
3. App layer drains the queue each frame (`src/app_sim_tick.rs:295+`) and
   translates `SimSoundEvent::EntityDied { die_sound_id, rx, ry }` into
   `AudioEvent::EntityDestroyed { sound_id, screen_pos }` for the audio backend.
4. Combat death path (`src/sim/combat/mod.rs:1311`) pushes `EntityDied` when an
   entity dies from weapon damage.

**The crush-kill path** (`src/sim/movement/movement_tick.rs:895-903`) zeros the
victim's HP and removes the entity from the store with **no sound event push**.
Today every crush kill is silent — both `DieSound` and `CrushSound` are missing.

**The 5 unparsed keys on `[E1]`**: `VoiceFeedback`, `VoiceSpecialAttack`,
`CrushSound`, `DeploySound`, `UndeploySound`. Their values in `rulesmd.ini` are
silently dropped during ObjectType parse.

## Impact Analysis

**Files touched** (5 source + 1 doc):
- `src/rules/object_type.rs` — +5 fields, +5 parse lines, +5 default initializers
  in test helpers.
- `src/sim/world/mod.rs` — +1 enum variant on `SimSoundEvent`.
- `src/sim/movement/movement_tick.rs` — emit `EntityCrushed` + `EntityDied` in
  the existing crush-kill loop (~10 lines).
- `src/audio/events.rs` — +1 `AudioEvent::EntityCrushed` variant.
- `src/app_sim_tick.rs` — translate `SimSoundEvent::EntityCrushed` →
  `AudioEvent::EntityCrushed` (~6 lines, mirror of EntityDied).
- `docs/research/GI_GHIDRA_REPORT.md` — flag 3
  stale findings (sub-cell, IronCurtain, DieSound) as already-implemented.

**Test-helper updates** (default-initializer compile fix):
- `src/sim/movement/locomotor_tests.rs:49`
- `src/sim/movement/teleport_movement.rs:284`
- Anywhere else `ObjectType { ... }` is built literal-style with all fields named.

**Determinism**: `SimSoundEvent` is drained per frame and not part of the state
hash — no determinism impact. Sim tick order unchanged.

**Blast radius**: zero. New `Option<String>` fields default to `None` —
existing callers unaffected. New enum variants are added, not changed.

## Chosen Approach

A2 — **Parse-now batch** (5 keys), wire `CrushSound` + `DieSound` on the crush
path only. The 4 deferred keys (`VoiceFeedback`, `VoiceSpecialAttack`,
`DeploySound`, `UndeploySound`) sit as parsed `Option<String>` fields awaiting
their state-machine consumers in future slices (deploy state for Slice B,
fear runtime for Slice D).

Rejected:
- **A1** (parse only `CrushSound`, defer the other 4): cheaper now, but commits
  us to re-editing `object_type.rs` four more times across future slices.
- **A3** (skip parsing entirely, only wire CrushSound by string lookup): same
  problem as A1.

## Design

### Components

**1. ObjectType parse** (`src/rules/object_type.rs`):

Add 5 fields after `move_sound` (line 219):
```rust
pub voice_feedback: Option<String>,
pub voice_special_attack: Option<String>,
pub crush_sound: Option<String>,
pub deploy_sound: Option<String>,
pub undeploy_sound: Option<String>,
```

Add 5 INI reads after existing `move_sound` parse (line 671):
```rust
voice_feedback: section.get("VoiceFeedback").map(|s| s.to_string()),
voice_special_attack: section.get("VoiceSpecialAttack").map(|s| s.to_string()),
crush_sound: section.get("CrushSound").map(|s| s.to_string()),
deploy_sound: section.get("DeploySound").map(|s| s.to_string()),
undeploy_sound: section.get("UndeploySound").map(|s| s.to_string()),
```

Update each test/default-init site to add `: None` for all 5 new fields.

**2. SimSoundEvent variant** (`src/sim/world/mod.rs:87-130`):
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

**3. Crush-kill emit** (`src/sim/movement/movement_tick.rs:895-903`):

Modify the deferred-crush-kills loop. Before each `entities.remove(victim_id)`:
- Look up victim's `ObjectType` via interner+ruleset.
- If `obj.crush_sound` is `Some(s)`: intern and push
  `SimSoundEvent::EntityCrushed { crush_sound_id, rx, ry }`.
- If `obj.die_sound` is `Some(s)`: intern and push
  `SimSoundEvent::EntityDied { die_sound_id, rx, ry }`.
- Skip emit when the field is `None` — no empty events in the queue.

The crush-kill loop already has access to `entities` (read-only after the
`get_mut` zero-HP write). The ruleset and interner reference must be threaded
in if not already in scope. **Verify** during implementation that
`tick_movement_with_grids` (the caller) has the ruleset/interner available;
if not, propagate them as args (matching the pattern used elsewhere in
movement_tick.rs).

**4. AudioEvent variant** (`src/audio/events.rs`, near `EntityDestroyed`):
```rust
/// An entity was crushed — play CrushSound (squish).
EntityCrushed {
    sound_id: String,
    screen_pos: Option<(f32, f32)>,
},
```

**5. App-side translation** (`src/app_sim_tick.rs`, near line 295):

Mirror the existing `SimSoundEvent::EntityDied → AudioEvent::EntityDestroyed`
arm:
```rust
SimSoundEvent::EntityCrushed { crush_sound_id, rx, ry } => {
    audio_events.push(AudioEvent::EntityCrushed {
        sound_id: sim.interner.resolve(crush_sound_id).to_string(),
        screen_pos: tactical.world_to_screen(rx, ry),
    });
}
```

**6. Doc cleanup** (`docs/research/GI_GHIDRA_REPORT.md`):

Edit three sections (§6, §P3.13, "Final Implementation Status") and add a new
appendix `## Verified-already-implemented (post-Phase-3 audit)` listing:
- Sub-cell allocator `[2, 3, 4]` — `src/sim/movement/bump_crush.rs:31`
- IronCurtain kills infantry — `src/sim/superweapon/iron_curtain.rs:57-60`
- DieSound parsing — `src/rules/object_type.rs:217+670`
- DieSound emit on combat death — `src/sim/combat/mod.rs:1311`

Plus update the report's "Final Implementation Status" table rows for
"InfantryType parsing" and "Damage gate (`ReceiveDamage`)" to reflect the new
state after this slice.

### Interfaces / Contracts

No public API changes. All edits are additive:
- `ObjectType` gains 5 optional fields — no caller breakage.
- `SimSoundEvent` and `AudioEvent` gain 1 variant each — exhaustive matches in
  app_sim_tick.rs need a new arm; that's the only required match update.

### Data Flow

```
INI [E1] CrushSound=InfantrySquish
   ↓
ObjectType.crush_sound = Some("InfantrySquish")
   ↓
[Tank crushes GI in movement tick]
   ↓
movement_tick: lookup victim ObjectType
   ↓
push SimSoundEvent::EntityCrushed { crush_sound_id: intern("InfantrySquish"), rx, ry }
push SimSoundEvent::EntityDied    { die_sound_id:    intern("GIDie"),         rx, ry }
   ↓
[App tick drains sim.sound_events]
   ↓
AudioEvent::EntityCrushed   { sound_id: "InfantrySquish", screen_pos: ... }
AudioEvent::EntityDestroyed { sound_id: "GIDie",          screen_pos: ... }
   ↓
audio backend plays both
```

### Error Handling

- Missing INI key → `Option<String>` is `None` → emit skipped → no sound plays.
  Matches gamemd behavior (silent if no key).
- Empty string in INI (`CrushSound=`) → `Some("")` → interner produces an empty
  `InternedId` → audio backend's `play_sound("")` is the existing no-op path
  for DieSound (verified by behavior). No special-casing needed.
- Unknown sound name (key set but not in `sound.ini`) → audio backend logs and
  drops, matching DieSound behavior.

### Testing Strategy

**New test** in `src/sim/movement/movement_tick.rs` (or wherever the closest
crush-kill integration test lives):
1. Construct sim with one Crusher tank and one Crushable infantry, infantry's
   `ObjectType` having `crush_sound = Some("InfantrySquish")` and
   `die_sound = Some("GIDie")`.
2. Run one movement tick where the tank enters the infantry's cell.
3. Assert `sim.sound_events` contains exactly one `EntityCrushed` with
   `crush_sound_id` resolving to `"InfantrySquish"` and one `EntityDied` with
   `die_sound_id` resolving to `"GIDie"`, both at the victim's pre-removal
   position.
4. Assert `stats.crush_kills == 1`.

**Negative test**: same scenario but with `crush_sound = None` — assert no
`EntityCrushed` event emitted but `EntityDied` still fires (and vice versa for
`die_sound = None`).

**Existing tests** (`test_crusher_crushes_crushable_infantry` etc. in
`bump_crush.rs`) continue unchanged — they test the can_crush predicate and
victim collection, not sound emission.

## Architectural Decisions

- **Pattern followed**: existing `SimSoundEvent → AudioEvent → playback` chain
  with `InternedId` keys. No new abstraction.
- **Two events per crush, not one combined**: gamemd plays CrushSound and
  DieSound as separate audio cues; combining them into a single event would
  break symmetry with the combat-death path (which pushes `EntityDied` only).
  Two events keep the data layout uniform.
- **Field naming**: `crush_sound`, `die_sound`, `voice_feedback` etc. follow the
  existing snake_case convention from `voice_select` / `voice_move` / etc.
- **Position field naming**: `rx`/`ry` matches the rest of `SimSoundEvent`. App
  layer translates to `screen_pos: Option<(f32, f32)>` matching `AudioEvent`
  convention.
- **No new files**: no module-size pressure (object_type.rs ~870 lines is data-
  heavy and OK per CLAUDE.md; world/mod.rs is fine for an enum addition).

**Tech debt introduced**: None. The 4 unwired sound fields are not abstractions
or behaviors — they're pure data preserved from the source-of-truth INI files.
Their consumers will be added in Slices B (deploy state machine) and D (fear
runtime) without further `object_type.rs` edits.

## Alternatives Considered

- **A1** (parse only `CrushSound`): rejected — forces 4 future re-edits of
  `object_type.rs`, churn for no real saving.
- **A3** (no parsing, lookup `crush_sound` by string from INI section at runtime):
  rejected — bypasses the established parsing layer, breaks the pattern.
- **Combined `EntityKilled { cause, sounds }` enum**: rejected — premature
  abstraction without a second use case.
- **Wire DeploySound/UndeploySound now as no-ops**: rejected — violates
  CLAUDE.md "no behavior beyond what's required."
