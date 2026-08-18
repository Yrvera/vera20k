# Paradrop ChuteSound Audio Dispatch - Implementation Plan

> Execute this plan task-by-task. Each task is self-contained and grounded in the approved design.

**Goal:** Play the configured `[AudioVisual] ChuteSound=` at the final paradrop passenger drop cell after each successful `Drop_Payload`, using the existing app/audio spatial SFX pipeline.

**Architecture:** Keep sim deterministic and presentation-free. The sim already emits `SimSoundEvent::ChuteSound { rx, ry }` after successful drop attach. This plan adds the missing rules field, typed app event, and app conversion arm so the existing `drain_sound_events` spatial playback path can resolve `[ParachuteDrop]` through `soundmd.ini`.

**Design Doc:** [docs/plans/2026-05-23-paradrop-chutesound-audio-dispatch-design.md](2026-05-23-paradrop-chutesound-audio-dispatch-design.md)

---

## Grounding Summary

- **Verified GameMD behavior:** successful paradrop payload release plays `ChuteSound` after passenger unlimbo/drop setup. Sources: `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`, `PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`, `0x004159FB -> 0x00415C60`.
- **INI source:** stock YR has `[AudioVisual] ChuteSound=ParachuteDrop` in `ini/rulesmd.ini:702`; GameMD reads it to `Rules+0x71C` at `0x0066ACEE`.
- **Sound data:** `ini/soundmd.ini [ParachuteDrop]` resolves `Sounds=sparadra` with `FShift=-10 10` and `VShift=15`. Current audio code resolves `Sounds=` and spatial range/volume; `FShift/VShift` are out of scope for this dispatch fix.
- **Current Rust sim state:** `src/sim/aircraft/drop_payload.rs` already emits `SimSoundEvent::ChuteSound { rx, ry }` only after `begin_parachute_descent` succeeds.
- **Current Rust app mismatch:** `src/app_sim_tick.rs` currently matches `SimSoundEvent::ChuteSound { rx, ry }` and drops it with a deferred-hookup comment.
- **Existing app pattern:** `BuildingGarrisonedSfx` and `RefineryExitSfx` resolve optional rules-configured sound IDs in `app_sim_tick.rs`, convert cell coordinates with `iso_to_screen`, and rely on `GameSoundEvent::screen_pos()` plus `drain_sound_events` for spatial playback.

## Key Technical Decisions

- **Parse `ChuteSound=` into `GeneralRules`, not into sim event payloads.** The sim event remains semantic and deterministic: "a chute sound should play at this cell."
- **Use a typed `GameSoundEvent::ChuteSound` variant.** This preserves the paradrop-specific audit trail and keeps the behavior discoverable.
- **Skip playback when `ChuteSound=` is missing or empty.** This matches existing optional audio-key handling and avoids hardcoding `ParachuteDrop`.
- **Do not add any new sim-side sound lookup.** `sim/` must not depend on `audio/` or presentation rules beyond pure data already available.
- **Do not implement `FShift` / `VShift` in this pass.** Those soundmd fields are real but belong to a broader audio playback parity task.

## Open Questions

### Resolved During Planning

- **Should `[AudioVisual] ChuteSound=` be parsed as part of this fix?** Yes. Hardcoding `ParachuteDrop` would match stock YR but fail modded rules parity.
- **Should the event be positional?** Yes. The sim event already carries the final drop cell, and current spatial SFX handling can use it.
- **Should threshold/approach emit the sound?** No. Earlier trace evidence shows standard `Mission_Open` is silent at threshold; sound belongs to successful payload release.

### Deferred

- **Soundmd `FShift` / `VShift`:** not supported by the current `SoundRegistry` / `SfxPlayer` path. Track as broader audio parity.
- **Manual audible verification:** optional after implementation if a local scenario is easy to run; unit checks should cover the code path.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/rules/ruleset.rs` | Add `GeneralRules::chute_sound` default, parser, and unit tests |
| Modify | `src/audio/events.rs` | Add typed `GameSoundEvent::ChuteSound` and accessor coverage |
| Modify | `src/app_sim_tick.rs` | Convert `SimSoundEvent::ChuteSound` into app sound event |
| Read/check | `src/sim/aircraft/drop_payload.rs` | Confirm existing success-gated sim emission remains unchanged |

## Interface Changes

- `GeneralRules` gains `pub chute_sound: Option<String>`.
- `GameSoundEvent` gains:

```rust
ChuteSound {
    sound_id: String,
    screen_pos: Option<(f32, f32)>,
}
```

- No change to `SimSoundEvent`; `ChuteSound { rx, ry }` already exists.
- No change to `SfxPlayer`; existing `sound_id()` / `screen_pos()` dispatch will handle the typed app event after accessor updates.

## Sim Checklist

- [x] No new sim state.
- [x] No deterministic hash impact.
- [x] No new sim dependency on audio/render/ui/sidebar/net.
- [x] No fixed-point or tick-order change.
- [x] Existing success-gated sound emission remains in `drop_payload.rs`.

## Risk Areas

- **Forgetting accessor updates:** `GameSoundEvent::sound_id()` and `screen_pos()` must include the new variant or playback/tests will fail.
- **Rules field omission in defaults:** `GeneralRules::default()` must set `chute_sound: None` or tests/constructors will fail.
- **Wrong fallback behavior:** do not hardcode `ParachuteDrop` in app dispatch. Missing config should skip playback, while full retail rules should parse the stock value.
- **Early duplicate sound:** do not re-enable `paradrop_mission.rs` threshold sound behavior; the real standard SW sound is post-drop.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1 | Parse `[AudioVisual] ChuteSound=` | Mods can change the chute sound; stock value is data, not a constant. | Unit test with `ChuteSound=CustomDrop`; full rules load should expose `ParachuteDrop`. |
| 2 | Typed app event carries screen position | Spatial playback must happen at the final drop cell. | `GameSoundEvent::screen_pos()` accessor test. |
| 3 | App conversion skips missing/empty config | Avoids hidden stock hardcode and matches optional audio-key pattern. | Unit/parser tests and code review of conversion arm. |
| 4 | Sound remains success-gated | Failed/requeued drops must stay silent. | Existing/added `drop_payload` test inspects `sim.sound_events`. |
| 5 | No threshold sound | Prevents audible sound before visible paratrooper release. | Ensure no implementation touches `paradrop_mission.rs` threshold sound logic. |

---

## Tasks

### Task 1: Parse `[AudioVisual] ChuteSound=`

**Why:** GameMD uses `Rules+0x71C` from `[AudioVisual] ChuteSound=`, stock `ParachuteDrop`. The app cannot resolve the correct sound without this rules field.

**Files:**
- Modify: `src/rules/ruleset.rs`

**Steps:**

1. Add `pub chute_sound: Option<String>` near the existing `[AudioVisual]` sound fields on `GeneralRules`.
2. Add `chute_sound: None` to `GeneralRules::default()`.
3. In `GeneralRules::from_ini`, parse:

```rust
chute_sound: audio_visual
    .and_then(|s| s.get("ChuteSound"))
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_string),
```

4. Add unit tests near the existing sound-key tests:
   - `test_chute_sound_parsed`
   - `test_chute_sound_empty_treated_as_none`
   - optional: extend the trim test to cover whitespace.

**Verification:**

Run:

```powershell
cargo test rules::ruleset::tests::test_chute_sound
```

If exact test filtering is awkward, run:

```powershell
cargo test rules::ruleset
```

### Task 2: Add typed app sound event

**Why:** The app queue needs a typed event so the existing spatial playback path can resolve the configured sound ID while keeping the paradrop-specific behavior visible.

**Files:**
- Modify: `src/audio/events.rs`

**Steps:**

1. Add `GameSoundEvent::ChuteSound { sound_id: String, screen_pos: Option<(f32, f32)> }` near other positional SFX variants.
2. Include the new variant in `GameSoundEvent::sound_id()`.
3. Include the new variant in `GameSoundEvent::screen_pos()`.
4. Add an accessor test similar to `test_building_garrisoned_sfx_screen_pos_accessor`.

**Verification:**

Run:

```powershell
cargo test audio::events::tests::test_chute_sound
```

Fallback:

```powershell
cargo test audio::events
```

### Task 3: Wire sim event to app event

**Why:** This is the actual bug: `app_sim_tick.rs` currently drops `SimSoundEvent::ChuteSound`.

**Files:**
- Modify: `src/app_sim_tick.rs`

**Steps:**

1. Replace the current `SimSoundEvent::ChuteSound { rx, ry }` `continue` arm.
2. Resolve the sound ID from `state.rules.as_ref().and_then(|r| r.general.chute_sound.as_deref())`.
3. If missing or empty, `continue`.
4. Convert `(rx, ry)` to screen coordinates with `crate::map::terrain::iso_to_screen(rx, ry, 0)`.
5. Return `GameSoundEvent::ChuteSound { sound_id, screen_pos: Some((sx, sy)) }`.

**Expected shape:**

```rust
SimSoundEvent::ChuteSound { rx, ry } => {
    let sound_id = match state
        .rules
        .as_ref()
        .and_then(|r| r.general.chute_sound.as_deref())
    {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => continue,
    };
    let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
    GameSoundEvent::ChuteSound {
        sound_id,
        screen_pos: Some((sx, sy)),
    }
}
```

**Verification:**

Run a compile-focused check after Tasks 1-3:

```powershell
cargo check
```

### Task 4: Confirm sim success gating remains correct

**Why:** The sound must play only on successful drop attach, not on failed cell entry, failed subcell placement, or attach retry.

**Files:**
- Read/check: `src/sim/aircraft/drop_payload.rs`
- Modify only if an existing focused test is missing: `src/sim/aircraft/drop_payload.rs`

**Steps:**

1. Inspect current `drop_payload` tests for success and retry paths.
2. If no test asserts sound gating, add a narrow test:
   - success path emits one `SimSoundEvent::ChuteSound`;
   - attach-failure or impassable retry emits none.
3. Do not move sound emission earlier than `begin_parachute_descent` success.

**Verification:**

Run:

```powershell
cargo test sim::aircraft::drop_payload
```

### Task 5: Focused end verification

**Why:** This change crosses rules, app event conversion, and audio event accessors; verify the narrow surface before considering it done.

**Commands:**

```powershell
cargo test rules::ruleset::tests::test_chute_sound
cargo test audio::events::tests::test_chute_sound
cargo test sim::aircraft::drop_payload
cargo check
```

If local filters do not match exact generated names, run the broader module tests:

```powershell
cargo test rules::ruleset
cargo test audio::events
cargo test sim::aircraft
cargo check
```

**Manual smoke check, optional but useful:**

1. Start a skirmish with American paradrop available.
2. Fire `AmericanParaDropSpecial`.
3. Listen for `ParachuteDrop` at each visible passenger release.
4. Confirm there is no chute sound merely when the carrier reaches `ParadropRadius` before a passenger appears.

## Done Criteria

- Full retail rules parse `general.chute_sound == Some("ParachuteDrop")`.
- `SimSoundEvent::ChuteSound` converts to `GameSoundEvent::ChuteSound` with the configured sound ID.
- `GameSoundEvent::ChuteSound` is spatial through `screen_pos()`.
- Failed/requeued drops do not emit or play chute sound.
- No hardcoded `ParachuteDrop` in app dispatch.
- `cargo check` passes, or any remaining failure is explicitly unrelated and documented.
