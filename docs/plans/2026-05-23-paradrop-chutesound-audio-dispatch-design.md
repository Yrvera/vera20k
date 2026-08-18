# Paradrop ChuteSound Audio Dispatch Design

## Goal

Play the stock paradrop chute sound after each successful passenger drop, resolving the sound through `[AudioVisual] ChuteSound=` instead of hardcoding stock data.

## Architecture Context

The simulation already emits a pure data event for this behavior. `src/sim/aircraft/drop_payload.rs` pushes `SimSoundEvent::ChuteSound { rx, ry }` only after passenger placement succeeds, occupancy is installed, and `begin_parachute_descent` succeeds.

The app layer drains `sim.sound_events` in `src/app_sim_tick.rs` and translates each `SimSoundEvent` into a `GameSoundEvent`. Those app events are later drained by `app_building_anim::drain_sound_events`, which routes positional sounds through `SfxPlayer::play_sound_with_volume`. Spatial falloff comes from `soundmd.ini` `Range=` / `MinVolume=` via `SoundRegistry`.

The current mismatch is the conversion arm for `SimSoundEvent::ChuteSound`: it explicitly drops the event. `GeneralRules` also does not currently expose `[AudioVisual] ChuteSound=`, so a correct app conversion needs one small rules field before it can preserve modded data.

## Impact Analysis

- `src/rules/ruleset.rs`: add `GeneralRules::chute_sound: Option<String>` parsed from `[AudioVisual] ChuteSound=`, trimmed and empty-filtered like nearby audio keys.
- `src/audio/events.rs`: add a typed `GameSoundEvent::ChuteSound { sound_id, screen_pos }` and include it in `sound_id()` / `screen_pos()`.
- `src/app_sim_tick.rs`: replace the current `continue` arm with lookup of `state.rules.general.chute_sound`, cell-to-screen conversion, and app-event creation.
- Tests should cover rules parsing, app event accessors, and the existing sim success-gating behavior.

This does not alter deterministic simulation state. The sim still emits only `(rx, ry)`; rules-to-sound-ID resolution and playback remain in app/audio layers above `sim/`.

## Chosen Approach

Use a typed app-layer event:

```rust
GameSoundEvent::ChuteSound {
    sound_id,
    screen_pos: Some((sx, sy)),
}
```

`app_sim_tick.rs` resolves `sound_id` from `state.rules.as_ref().and_then(|r| r.general.chute_sound.as_deref())`. If the rules key is missing or empty, the app skips playback. Stock YR provides `ChuteSound=ParachuteDrop`, and the existing `SoundRegistry` resolves `[ParachuteDrop]` to `sparadra`.

This follows the existing `BuildingGarrisonedSfx` and `RefineryExitSfx` pattern: sim produces a semantic event, app resolves the configured sound ID, and audio playback applies positional volume through the generic spatial sound path.

## Tiny-Detail Ledger

- Standard YR paradrop path is active: `Mission_Rescue` calls `Drop_Payload` for stock `Type=ParaDrop` / `Type=AmerParaDrop` carriers. Source: `PARADROP_DROP_CADENCE_GHIDRA_REPORT.md`, `0x004159FB -> 0x00415C60`.
- The sound plays after successful passenger unlimbo/drop setup, not when the plane merely enters `ParadropRadius`. Source: `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md`, `PARADROP_APPROACH_DROP_CADENCE_TRACE.md`.
- Failed or requeued payload drops must not play `ChuteSound`. Source: `PARACHUTED_INFANTRY_DESCENT_RENDER_GHIDRA_REPORT.md` implementation handoff.
- The sound ID is parsed from `[AudioVisual] ChuteSound=`, stock value `ParachuteDrop`, read by GameMD to `Rules+0x71C`. Source: `rulesmd.ini:702`, Ghidra reader `0x0066ACEE`.
- Current Rust already success-gates the sim event in `drop_payload.rs`; implementation must not add an earlier threshold sound path in `paradrop_mission.rs`.
- `soundmd.ini [ParachuteDrop]` has `Sounds=sparadra`, `FShift=-10 10`, and `VShift=15`. The current audio registry handles `Sounds=` and default volume/range, but not pitch/volume shifts. This design does not solve `FShift/VShift`; that remains a broader audio-parity limitation.
- The event should use the final drop cell `(rx, ry)` for spatial playback, not the original click target or aircraft cell. Source: current sim event payload from `drop_payload.rs` and GameMD `Drop_Payload` sound placement evidence.

## Design

### Components

`GeneralRules` gains `chute_sound: Option<String>`, parsed alongside other `[AudioVisual]` sound IDs. Default is `None`, but stock full rules load `Some("ParachuteDrop")`.

`GameSoundEvent` gains a typed `ChuteSound` variant. Keeping it typed preserves the audit trail and makes future parity work easier than using a generic one-off string event.

`app_sim_tick.rs` maps `SimSoundEvent::ChuteSound { rx, ry }` to the typed app event only when `chute_sound` is configured.

### Interfaces / Contracts

- `sim/` contract remains semantic: `SimSoundEvent::ChuteSound { rx, ry }` means "a successful paradrop payload release should play the configured chute sound at this cell."
- `rules/` contract: `GeneralRules::chute_sound` is the raw sound ID from `[AudioVisual]`, not a filename.
- `audio/` contract: `GameSoundEvent::ChuteSound` behaves like other positional SFX and resolves through `SoundRegistry`.

### Data Flow

1. `drop_payload::try_drop` succeeds and pushes `SimSoundEvent::ChuteSound { rx: drop_rx, ry: drop_ry }`.
2. `app_sim_tick.rs` drains sim events after the sim tick.
3. The chute arm reads `state.rules.general.chute_sound`.
4. The drop cell converts through `iso_to_screen(rx, ry, 0)`.
5. The app queues `GameSoundEvent::ChuteSound`.
6. `drain_sound_events` handles it through the existing spatial default path.

### Error Handling

If rules are unavailable, `ChuteSound=` is absent, or the value is empty, skip playback. This matches existing optional sound-key patterns and avoids hardcoding stock data into the app layer.

If `SoundRegistry` cannot resolve the configured ID, `SfxPlayer` already logs/traces failure and returns false. No new failure mode is needed.

### Testing Strategy

- Add a `GeneralRules::from_ini` unit test proving `[AudioVisual] ChuteSound=CustomDrop` parses to `Some("CustomDrop")`.
- Add a trim/empty-value test or extend an existing audio-key trim test to cover `ChuteSound=`.
- Add an `audio::events` accessor test confirming `GameSoundEvent::ChuteSound` returns its sound ID and screen position.
- Keep or add a focused sim test confirming failed/requeued drops do not emit `SimSoundEvent::ChuteSound`; current `drop_payload.rs` success-gating should already support this.
- If an app conversion test seam exists, assert `SimSoundEvent::ChuteSound` maps to the configured `GameSoundEvent::ChuteSound` and skips when missing. If no seam exists, keep verification to unit coverage plus an integration/manual audio check.

## Architectural Decisions

- **Keep sound ID resolution out of `sim/`.** This preserves the existing layer boundary: deterministic sim emits semantic events, app/audio resolves presentation.
- **Use a typed event instead of a generic positional event.** This is slightly more enum churn, but it keeps this parity finding discoverable and avoids losing the source-specific behavior in a generic string path.
- **Do not hardcode `ParachuteDrop`.** Stock YR uses that value, but mods can change `[AudioVisual] ChuteSound=`.
- **Do not implement `FShift` / `VShift` here.** Those are real soundmd fields, but they require a broader audio registry/playback design and should not be smuggled into this narrow dispatch fix.

## Alternatives Considered

### Generic positional SFX event

This would add `GameSoundEvent::PositionalSfx { sound_id, screen_pos }` and route ChuteSound through it. It is viable, but less explicit and makes later audit work harder because the semantic chute event disappears after app conversion.

### Hardcoded `ParachuteDrop`

This is the smallest patch, but it fails modded rules parity and contradicts the verified `[AudioVisual] ChuteSound=` source. Rejected.

### Resolve the sound ID in `sim/`

This would let `SimSoundEvent::ChuteSound` carry an interned sound ID, but it couples gameplay/drop code to audio-configuration lookup and creates unnecessary deterministic-state churn. Rejected.
