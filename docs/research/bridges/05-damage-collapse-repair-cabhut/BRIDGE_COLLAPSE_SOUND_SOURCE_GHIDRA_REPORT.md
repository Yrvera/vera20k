# Bridge Collapse Sound Source - Ghidra Research Report

**Address(es):** `0x0047DD70` (`CellClass::BlowUpBridge`), `0x00424CE0` (`AnimClass::Middle`), `0x00427D00` (`AnimTypeClass::ReadINI`)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** exact sound source/key path for animation objects spawned by `CellClass::BlowUpBridge` during bridge cell collapse.  
**Non-Scope:** bridge repair sound mechanics except as a negative comparison; bridge repair logic; generic animation sound system beyond the call sites needed here.  
**Confidence:** High  
**Active in YR:** Yes. Standard YR `rulesmd.ini` defines non-empty `BridgeExplosions=`, and standard YR `artmd.ini` gives those anims `Report=` sounds.

## Working Notes

Target question: Does `CellClass::BlowUpBridge` emit bridge collapse sound directly, indirectly through spawned `AnimClass`/`AnimType` sound fields, or both?  
Non-goals: Do not investigate bridge repair, campaign trigger behavior, or generic animation audio beyond the `BlowUpBridge` spawn path.  
Evidence needed to mark COMPLETE: decompile plus assembly for `BlowUpBridge` spawn calls, decompile plus assembly for `AnimTypeClass::ReadINI` sound-key parsing, decompile plus assembly for `AnimClass::Middle` playback, INI/default source for the standard YR anim and sound IDs, and current Rust surface scan.  
Stop conditions: stop when direct-vs-indirect emission, exact keys, call addresses, argument order, per-cell granularity, and YR activity are all resolved; defer only runtime numeric audio-event pointer identities.

## 1. Overview

`CellClass::BlowUpBridge` does not directly call `VocClass::PlayAt` or any sound API for bridge collapse. It spawns `AnimClass` instances from `[General] BridgeExplosions=` and optionally from `[General] MetallicDebris=`.

The audible bridge-collapse explosion comes indirectly from the spawned `BridgeExplosions` animation type's `StartSound=` / `Report=` field at `AnimTypeClass + 0x2F8`. In standard YR, the four `BridgeExplosions` anims are `TWLT026`, `TWLT036`, `TWLT050`, and `TWLT070`; their `artmd.ini` `Report=` keys resolve to sound IDs `ExplosionShard`, `Explosion06`, `Explosion07`, and `Explosion09`.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `RulesClass` | `+0x140` | `MetallicDebris.Vector` | `BlowUpBridge` assembly `0x0047DFAE`; parser string xref `0x0066DAA5` | Yes, `rulesmd.ini:528` |
| `RulesClass` | `+0x14C` | `MetallicDebris.ActiveCount` | `BlowUpBridge` assembly `0x0047DF7C`; prior DVC report | Yes |
| `RulesClass` | `+0x15C` | `BridgeExplosions.Vector` | `BlowUpBridge` assembly `0x0047E020`; parser string xref `0x0066DBA8` | Yes, `rulesmd.ini:529` |
| `RulesClass` | `+0x168` | `BridgeExplosions.ActiveCount` | `BlowUpBridge` decompile and assembly `0x0047DE33`, `0x0047DFF4` | Yes |
| `AnimTypeClass` | `+0x2F8` | `StartSound` / fallback `Report` resolved Voc index | `AnimTypeClass::ReadINI` `0x00428359`, `0x0042839A`; `AnimClass::Middle` `0x00424D01` | Yes |
| `AnimClass` | `+0x1A0` | start/report sound handle | `AnimClass::Middle` assembly `0x00424D0C`; decompile | Yes |

## 3. Core Logic

### 3.1 `CellClass::BlowUpBridge` sound-relevant path

Verified behavior: the function gates the visual/audio-producing animation block on `BridgeExplosions.ActiveCount > 0` and an outer random probability check against `0.95`. If the gate fails, it does not spawn either the optional metallic debris or the bridge explosion anim for that cell.

Evidence: `CellClass::BlowUpBridge @ 0x0047DD70`, assembly `0x0047DE33..0x0047DE72`; constant memory `0x007E4F58 = 0.95`, `0x007E3570` is the random-to-double scale. Active in YR: Yes; standard YR `BridgeExplosions` count is 4.

After the outer gate passes, it computes a centered cell coordinate:

- `x = cell_x * 0x100 + 0x80`
- `y = cell_y * 0x100 + 0x80`
- `z = cell_level * DAT_0089E7C0 + DAT_0089E7B4`

Then it consumes two random draws to jitter `x` and `y` by approximately `[-25, +25)` leptons using constants `0.5` and `50.0`. Evidence: assembly `0x0047DEC6..0x0047DF27`, constant memory `0x007E1738 = 0.5`, `0x007E4F50 = 50.0`. Active in YR: Yes.

### 3.2 Optional `MetallicDebris` spawn

Verified behavior: after jitter, `BlowUpBridge` runs a 50% gate. If it passes and allocation succeeds, it selects one `MetallicDebris` anim type and constructs it with zero delay:

| Constructor argument | Value |
|---|---|
| `this` | newly allocated `AnimClass` (`operator_new(0x1C8)`) |
| `type` | `RulesClass.MetallicDebris.Vector[random(0, count - 1)]` |
| `coords` | jittered cell-center coord |
| `delay` | `0` |
| `loopCount` | `1` |
| `drawFlags` | `0x600` |
| `zAdjust` | `0` |
| `reverse` | `0` |

Evidence: decompile `0x0047DD70`; assembly call `0x0047DFBA` with argument pushes at `0x0047DF9C..0x0047DFB9`. Active in YR: Yes, conditional on the 95% outer gate, 50% metallic gate, non-empty `MetallicDebris`, and allocation success.

Sound implication: standard YR `DBRIS*` metallic debris entries do not define `StartSound=` or `Report=`, so the metallic anim itself is silent at creation. Several entries do define `ExpireAnim=TWLT036` or `ExpireAnim=TWLT026`; that can later create a TWLT anim with its own `Report`, but that is a delayed animation-expiration consequence, not a direct `BlowUpBridge` sound emission. Evidence: `artmd.ini:14986..15427` and `AnimClass::AI` expire/bounce branch creates `ExpireAnim` at `0x00423AC0`. Active in YR: Conditional.

### 3.3 Guaranteed `BridgeExplosions` spawn after the outer gate

Verified behavior: after optional metallic debris, `BlowUpBridge` always attempts to allocate one `BridgeExplosions` anim for that cell if the outer gate passed. It selects a delay in the inclusive range `1..5`, selects a random bridge-explosion anim type from `RulesClass + 0x15C`, then constructs `AnimClass`:

| Constructor argument | Value |
|---|---|
| `this` | newly allocated `AnimClass` (`operator_new(0x1C8)`) |
| `type` | `RulesClass.BridgeExplosions.Vector[random(0, count - 1)]` |
| `coords` | same jittered cell-center coord |
| `delay` | `random(1, 5)` inclusive |
| `loopCount` | `1` |
| `drawFlags` | `0x600` |
| `zAdjust` | `0` |
| `reverse` | `0` |

Evidence: decompile `0x0047DD70`; assembly `0x0047DFD7..0x0047E02C` (`PUSH 0x5`, `PUSH 0x1`, call random, then `CALL 0x00421EA0`). Active in YR: Yes, conditional on outer 95% gate and allocation success.

### 3.4 How the sound actually plays

Verified behavior: `AnimTypeClass::ReadINI` first tries `StartSound=` at `0x00428359`; only if the resulting value is `-1` does it read `Report=` at `0x0042839A`. Both keys write the same `AnimTypeClass + 0x2F8` sound field.

Evidence: string xrefs `StartSound @ 0x00818418 -> 0x00428359`, `Report @ 0x00818410 -> 0x0042839A`; decompile `AnimTypeClass::ReadINI @ 0x00427D00`. Active in YR: Yes.

Verified behavior: `AnimClass::Middle` checks `AnimType + 0x2F8 != -1`, gets the animation coordinates, and calls `VocClass::PlayAt` through `0x007509E0` using the `AnimClass + 0x1A0` sound handle. This is called immediately from the constructor only when `delay == 0`; otherwise it runs when the delay counter reaches zero in `AnimClass::AI`.

Evidence: `AnimClass::Middle @ 0x00424CE0`, assembly `0x00424D01..0x00424D2B`; `AnimClass::Constructor @ 0x00421EA0` calls `Middle` only when the delay field is zero. Active in YR: Yes.

Therefore the standard bridge-explosion sound is delayed by `1..5` frames per collapsed cell that passes the outer gate, because `BlowUpBridge` passes that delay to the `BridgeExplosions` anim. The optional metallic-debris anim is created with zero delay but has no standard direct start/report sound.

## 4. INI Keys

| Key | Section / file | Standard YR value | Binary reader | Effect | Active in YR |
|---|---|---|---|---|---|
| `BridgeExplosions=` | `[General]`, `rulesmd.ini:529` | `TWLT026,TWLT036,TWLT050,TWLT070` | `RulesClass::ReadGeneral`, string xref `0x0066DBA8` | Anim pool for one per-cell bridge explosion | Yes |
| `MetallicDebris=` | `[General]`, `rulesmd.ini:528` | `DBRIS1LG..DBRS10SM` | `RulesClass::ReadGeneral`, string xref `0x0066DAA5` | Optional per-cell debris anim pool | Yes |
| `StartSound=` | anim section, `artmd.ini` | not set on standard TWLT bridge-explosion anims | `AnimTypeClass::ReadINI`, `0x00428359` | Primary anim start sound key | Yes |
| `Report=` | anim section, `artmd.ini` | set on standard TWLT bridge-explosion anims | `AnimTypeClass::ReadINI`, `0x0042839A` | Fallback anim start sound key, same field as `StartSound` | Yes |
| `RepairBridgeSound=` | `[AudioVisual]`, `rulesmd.ini:721` | `BridgeRepaired` | used in engineer repair path, not `BlowUpBridge` | Repair-only spatial sound | Yes, but not collapse |

Standard YR bridge-explosion sound mapping:

| `BridgeExplosions` anim | `artmd.ini` key | sound list ID | `soundmd.ini` index | Samples | Active in YR |
|---|---|---|---:|---|---|
| `TWLT026` | `Report=ExplosionShard` (`artmd.ini:15659`) | `ExplosionShard` | `291` (`soundmd.ini:323`) | `gexpshaa`, `gexpshaa` | Yes |
| `TWLT036` | `Report=Explosion06` (`artmd.ini:15667`) | `Explosion06` | `280` (`soundmd.ini:312`) | `gexp06a` | Yes |
| `TWLT050` | `Report=Explosion07` (`artmd.ini:15675`) | `Explosion07` | `281` (`soundmd.ini:313`) | `gexp07a` | Yes |
| `TWLT070` | `Report=Explosion09` (`artmd.ini:15683`) | `Explosion09` | `283` (`soundmd.ini:315`) | `gexpifva`, `gexpifvb`, `gexpifvc` | Yes |

YR patch note: base `art.ini` maps `TWLT026` to `Explosion05`, but `artmd.ini` overrides it to `ExplosionShard`; YR uses the `artmd.ini` value.

## 5. Integration Points

`BlowUpBridge` has many unconditional call xrefs from bridge damage/ramp-collapse functions, including `ProcessBridgeDamageStateMachine_Low`, `ProcessBridgeDamageStateMachine_High`, `MapClass__UpdateRamp_*_Collapse*_Low`, and `CellClass__SetBridgeDirection_NWSE`. Evidence: `get_function_xrefs 0x0047DD70`, with call sites such as `0x005716C7`, `0x00576DAD`, and `0x0047E544`. Active in YR: Yes; these are the already-verified bridge collapse paths, not TS-only dead paths.

Granularity: the sound-producing `BridgeExplosions` anim is spawned per `CellClass::BlowUpBridge` invocation, not once per whole collapse event. A CABHUT collapse that destroys many cells can therefore schedule many delayed TWLT report sounds, subject to the per-cell 95% gate and allocation success. Active in YR: Yes.

There is no direct `VocClass::PlayAt` call in `CellClass::BlowUpBridge`. The direct sound call is in `AnimClass::Middle` at `0x00424D2B`, reached through the spawned animation object. Active in YR: Yes.

## 6. Current Rust Implementation Status

Current Rust already creates per-cell `WorldEffect` visuals in `src/sim/world/bridge_orchestrator.rs:925` through `spawn_bridge_debris`, with delayed `BridgeExplosions` visuals at `:979..999`.

Current Rust sound event surfaces include `SimSoundEvent::BridgeRepaired` in `src/sim/world/mod.rs:175..181` and `GameSoundEvent::BridgeRepaired` in `src/audio/events.rs:143..158`, but there is no bridge-collapse or animation-report sound event in the current scan. Current `WorldEffect` creation is visual-only.

Rust-facing delta: future work should not add a fixed `BridgeCollapseSound` or reuse `BridgeRepaired`. It should make spawned bridge explosion effects produce their anim `Report=` sound at the same delayed moment that the visual starts, using the selected anim ID's resolved `StartSound`/`Report` field.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `CellClass::BlowUpBridge` direct sound calls | verified | decompile `0x0047DD70`; no `VocClass`/sound call in body | none |
| `BlowUpBridge` `BridgeExplosions` constructor call | verified | assembly `0x0047DFD7..0x0047E02C`; call at `0x0047E02C` | none |
| `BlowUpBridge` `MetallicDebris` constructor call | verified | assembly `0x0047DF63..0x0047DFBA`; call at `0x0047DFBA` | none |
| `AnimTypeClass::ReadINI` `StartSound`/`Report` field | verified | decompile `0x00427D00`; assembly `0x00428359`, `0x0042839A` | none |
| `AnimClass::Middle` playback call | verified | decompile `0x00424CE0`; assembly `0x00424D01..0x00424D2B` | none |
| Standard YR bridge explosion anim keys | verified | `rulesmd.ini:529`, `artmd.ini:15656..15686`, `soundmd.ini:307..323`, `soundmd.ini:2555..2637` | none |
| Bridge repair sound as collapse candidate | verified-negative | `RepairBridgeSound=BridgeRepaired` in `rulesmd.ini:721`; repair path xref `0x00519BC4`, not `BlowUpBridge` | none |
| Runtime numeric `AudioEventClass*` pointer identity | deferred | `VocClass::PlayAt @ 0x007509E0` resolves from loaded Voc index | only needed for audio engine internals, not Rust key mapping |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] Q1 - Entry point: Does BlowUpBridge directly call sound playback? -> No; no VocClass/sound call in the body.` (evidence: `0x0047DD70`)
- `[RESOLVED] Q2 - Entry point: Which spawned object can emit the bridge collapse sound? -> The `BridgeExplosions` AnimClass object through AnimType +0x2F8.` (evidence: `0x0047E02C`, `0x00424D01..0x00424D2B`)
- `[RESOLVED] Q3 - INI key: Where is `BridgeExplosions` read? -> `RulesClass::ReadGeneral`, string xref `0x0066DBA8`, DVC vector read at Rules +0x15C.` (evidence: `0x0066DBA8`, `0x0047E020`)
- `[RESOLVED] Q4 - INI key: Where is `MetallicDebris` read? -> `RulesClass::ReadGeneral`, string xref `0x0066DAA5`, DVC vector read at Rules +0x140.` (evidence: `0x0066DAA5`, `0x0047DFAE`)
- `[RESOLVED] Q5 - INI key: Are `StartSound` and `Report` separate fields? -> No; both resolve into `AnimType +0x2F8`, with `Report` only used if `StartSound` remains -1.` (evidence: `0x00428359`, `0x0042839A`)
- `[RESOLVED] Q6 - Existing claim: Is animation `Report=` the explosion sound source? -> Yes for `BridgeExplosions`, via `AnimClass::Middle` calling `VocClass::PlayAt`.` (evidence: `0x00424D01..0x00424D2B`)
- `[RESOLVED] Q7 - Existing claim: Is there an unidentified splash-sound call inside BlowUpBridge? -> Stale wording; the sound call is not in BlowUpBridge, it is in the spawned animation's start path.` (evidence: `0x0047DD70`, `0x00424CE0`)
- `[RESOLVED] Q8 - Rust function: Does current `spawn_bridge_debris` emit audio? -> No; scan found only `WorldEffect` pushes and no `SimSoundEvent` for collapse.` (evidence: `src/sim/world/bridge_orchestrator.rs:925..999`)
- `[RESOLVED] Q9 - Tick-cycle integration: When does BridgeExplosions sound play? -> On animation start after the `1..5` frame delay, not at the `BlowUpBridge` call site.` (evidence: `0x0047DFD7..0x0047E02C`, `0x00424D01..0x00424D2B`)
- `[RESOLVED] Q10 - TS legacy filter: Is this path active in standard YR? -> Yes; standard YR defines non-empty `BridgeExplosions` and TWLT Report values in `rulesmd.ini`/`artmd.ini`.` (evidence: `rulesmd.ini:529`, `artmd.ini:15656..15686`)
- `[RESOLVED] Q11 - Edge case: Empty `BridgeExplosions` vector? -> Entire effect block exits, including optional metallic debris; no collapse anim sound.` (evidence: branch at `0x0047DE33..0x0047DE3B`)
- `[RESOLVED] Q12 - Edge case: Allocation failure? -> The skipped allocation skips that anim, so no sound from that skipped anim.` (evidence: null checks at `0x0047DF72..0x0047DF74`, `0x0047DFCE..0x0047DFD0`)
- `[RESOLVED] Q13 - Edge case: Metallic debris immediate sound? -> Standard YR DBRIS entries have no `StartSound`/`Report`; creation is silent.` (evidence: `artmd.ini:14986..15427`, `0x00428359..0x0042839A`)
- `[RESOLVED] Q14 - Edge case: Base RA2 vs YR TWLT026 sound? -> YR overrides base `Explosion05` with `ExplosionShard`.` (evidence: `art.ini:11153`, `artmd.ini:15659`)
- `[DEFERRED] Q15 - Exact runtime `AudioEventClass*` pointer for each sound ID.` (category: `requires-different-system-context`; reason: Rust needs symbolic sound IDs, and pointers are runtime-loaded audio engine state; next-step-if-pursued: trace `VocClass__FindByName` and audio list construction.)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `BlowUpBridge` sound is indirect: one selected `BridgeExplosions` anim per collapsed cell plays its `Report=`/`StartSound` when the anim starts after delay `1..5`. | `0x0047E02C`, `0x00424D01..0x00424D2B`, `artmd.ini:15656..15686` | missing | `src/sim/world/bridge_orchestrator.rs:979..999`; app animation/effect audio routing | attach resolved anim sound ID to the spawned bridge explosion effect or emit a delayed anim-start sound event when the visual begins | collapse one bridge cell with deterministic RNG selecting `TWLT036`; after the chosen delay, exactly one positional `Explosion06` sound is emitted with the visual | do not emit a sound immediately at collapse time |
| Standard YR sound IDs are selected by the chosen TWLT anim, not by a global bridge-collapse key. | `rulesmd.ini:529`, `artmd.ini:15659/15667/15675/15683`, `0x00428359..0x0042839A` | missing | rules/art parsing and audio event resolution | resolve anim `StartSound`/fallback `Report` for `TWLT026/036/050/070`; use the selected anim's sound ID | seed RNG to select each TWLT entry across four runs and assert emitted IDs are `ExplosionShard`, `Explosion06`, `Explosion07`, `Explosion09` | do not hardcode one sound such as `Explosion06` for every bridge cell |
| `BridgeRepaired` / `RepairBridgeSound` is repair-only and not a collapse sound source. | `rulesmd.ini:721`; string xref `EVA_BridgeRepaired @ 0x00519BC4`; no repair sound xref in `0x0047DD70` | current repair event exists; no collapse reuse needed | `src/sim/world/mod.rs:175..181`; `src/audio/events.rs:143..158` | leave repair event separate; add/route collapse audio through animation sound handling | collapsing a bridge must not produce `BridgeRepaired` or `EVA_BridgeRepaired`; engineer repair still does | do not reuse `GameSoundEvent::BridgeRepaired` for collapse |

Proposed Rust test names:

- `bridge_collapse_explosion_report_sound_fires_after_anim_delay`
- `bridge_collapse_uses_selected_twlt_report_sound_id`
- `bridge_collapse_does_not_emit_bridge_repaired_sound`

### Negative Facts / Do Not Do

- Do not add a direct `VocClass`-equivalent call to `BlowUpBridge` in Rust; binary `BlowUpBridge @ 0x0047DD70` has no direct sound call.
- Do not use `RepairBridgeSound=BridgeRepaired` as bridge collapse audio; its binary xref is in the engineer repair path, not in `BlowUpBridge`.
- Do not treat `Report=` and `StartSound=` as two simultaneous sounds; `AnimTypeClass::ReadINI` stores exactly one resolved value at `+0x2F8`, trying `StartSound` first and `Report` only as fallback.
- Do not play the bridge explosion sound once per whole collapse event; binary granularity is per collapsed cell reaching `BlowUpBridge`, subject to per-cell gates and allocation.
- Do not use base `art.ini`'s `TWLT026 Report=Explosion05` for YR; `artmd.ini` overrides it to `ExplosionShard`.

### Stale Docs / Follow-up Docs

- `docs/research/traces/CABHUT_BRIDGE_COLLAPSE_VISUAL_TRACE.md`: replace "BlowUpBridge contains splash-sound call chain but exact sound ID is unidentified" with "BlowUpBridge has no direct sound call. Its bridge-collapse sound comes indirectly from the spawned `BridgeExplosions` anim's `StartSound`/fallback `Report` field; standard YR maps `TWLT026/TWLT036/TWLT050/TWLT070` to `ExplosionShard/Explosion06/Explosion07/Explosion09`."

## Sources

- Ghidra `CellClass::BlowUpBridge @ 0x0047DD70`
- Ghidra `AnimTypeClass::ReadINI @ 0x00427D00`
- Ghidra `AnimClass::Constructor @ 0x00421EA0`
- Ghidra `AnimClass::Middle @ 0x00424CE0`
- Ghidra `AnimClass::AI @ 0x00423AC0`
- Ghidra `VocClass::PlayAt @ 0x007509E0`
- Ghidra string xrefs: `BridgeExplosions @ 0x0083CEDC`, `MetallicDebris @ 0x0083CEF0`, `StartSound @ 0x00818418`, `Report @ 0x00818410`, `EVA_BridgeRepaired @ 0x00825538`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `ini/soundmd.ini`
- `docs/research/ANIMATION_SOUNDS_GHIDRA_REPORT.md`
- `docs/research/BRIDGEEXPLOSIONS_RULES_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_COLLAPSE_CHAIN_MECHANISM_GHIDRA_REPORT.md`
