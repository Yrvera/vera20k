# AMCV Deploy Blocked EVA Trace - 2026-05-27

## Scenario

Player-controlled Allied AMCV at `(20,22)` attempts to deploy into `GACNST`, but a structure occupies a cell inside the target `GACNST` foundation. Expected stock YR result: deploy fails, AMCV remains, no ConYard appears, and local human player hears `EVA_CannotDeployHere`.

## Pipeline

`Command::DeployMcv`
-> `Simulation::deploy_mcv`
-> target foundation blocker check
-> `SimSoundEvent::CannotDeployHere`
-> app local-owner/faction gate
-> `GameSoundEvent::CannotDeployHere { sound_id: ceva063 }`
-> `SfxPlayer::play_voice_sound`

## Stage Trace

### Stage 1 - Trigger

- Input: `Command::DeployMcv { entity_id: AMCV }`.
- Rust path: `src/sim/world/world_commands.rs::Command::DeployMcv`.
- gamemd: `UnitClass::Deploy @ 0x007393C0` is active for stock `[AMCV] DeploysInto=GACNST`.
- Verdict: PASS for scoped trigger.

### Stage 2 - Target Foundation Validation

- Input: AMCV cell `(20,22)`, target `[GACNST] Foundation=4x4`, large-foundation origin `(-1,-1)` -> Rust origin `(19,21)`.
- Rust output: while walking the computed foundation cells, a structure overlap causes `deploy_mcv` to return `false` before `despawn_entity`.
- Code: `src/sim/world/world_spawn.rs:690..725`.
- gamemd: target BuildingType vtable `+0xA8 -> 0x00716150` walks the base foundation and calls `Cell_passability_building_placement @ 0x0047C620`; false reaches the cannot-deploy failure branch.
- Source: `docs/research/AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md:74..82`.
- Verdict: PASS for the concrete structure-blocked footprint case. Broader overlay/slope/bridge/buildability taxonomy remains UNCHECKED.

### Stage 3 - Failure Side Effects

- Rust order:
  1. Detect occupied foundation cell.
  2. Push `SimSoundEvent::CannotDeployHere { owner: owner_id }`.
  3. Return `false`.
  4. Skip `despawn_entity` and skip `spawn_object_at_height`.
- Code: `src/sim/world/world_spawn.rs:710..724`, `:729`.
- gamemd order:
  1. Target placement virtual returns false.
  2. For human player and `type+0x5EC == 0`, call `VoxClass__PlayEVA("EVA_CannotDeployHere")`.
  3. Re-enable deploy interface.
  4. Clear deploy state and return `0`.
  5. AMCV is not removed.
- Source: `docs/research/AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md:76..82`.
- Verdict: PASS for standard AMCV/GACNST audible failure and no-consume behavior. UNCHECKED for unmodeled `type+0x5EC` deploy-target flag variants.

### Stage 4 - Sim Event Payload

- Rust output: `SimSoundEvent::CannotDeployHere { owner = Americans }`.
- Test: `src/sim/deploy_tests.rs:291..297`.
- gamemd: EVA is gated by human player; Rust emits owner-bound sim event and app layer applies local-human gate, matching existing deterministic sound-event pattern.
- Verdict: PASS for local human owner gating model.

### Stage 5 - App EVA Resolution

- Rust input: `SimSoundEvent::CannotDeployHere { owner }`.
- Rust output: if `owner` equals the local owner, resolve `EVA_CannotDeployHere` through `EvaRegistry`, fallback `ceva063`.
- Code: `src/app_sim_tick.rs:435..453`.
- INI: `ini/evamd.ini:1101..1106` maps Allied to `ceva063`, Russian to `csof063`, Yuri to `cyur063`.
- gamemd: uses EVA string `EVA_CannotDeployHere @ 0x0082012C`.
- Source: `docs/research/AMCV_CANDEPLOY_PREDICATE_GHIDRA_REPORT.md:77`.
- Verdict: PASS for standard Allied fallback and faction-registry path.

### Stage 6 - Playback

- Rust output: `GameSoundEvent::CannotDeployHere` calls `sfx.play_voice_sound(...)`.
- Code: `src/app_building_anim.rs:592..600`.
- gamemd: `VoxClass__PlayEVA` plays the EVA announcement.
- Verdict: PASS for dispatch to the voice/EVA playback path. Actual audible output was not manually listened to in this trace.

## Verification Commands

```powershell
cargo test --lib deploy_mcv_rejects_structure_in_rightmost_foundation_column -- --nocapture
cargo test --lib test_cannot_deploy_here_sound_id_accessor -- --nocapture
cargo check -q
```

All passed. Existing unrelated warnings remain.

## Verdict

PASS for the scoped player-visible bug: a blocked AMCV deploy now preserves the AMCV and blocker, spawns no ConYard, emits owner-bound `CannotDeployHere`, resolves `EVA_CannotDeployHere`/`ceva063` for the local Allied player, and sends it to the voice playback path.

Remaining UNCHECKED surfaces:

- Exact blocker taxonomy for overlay/slope/nonbuildable/bridge marker cases.
- The unmodeled `type+0x5EC` deploy-target flag gate for nonstandard deployers.
- Manual audible verification in a running game session.
