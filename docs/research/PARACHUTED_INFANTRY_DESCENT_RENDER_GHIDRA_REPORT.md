# Parachuted Infantry Descent/Render - Ghidra Research Report

**Address(es):** `0x00415C60`, `0x005F5940`, `0x005F3E70`, `0x005F6DA0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR post-`AircraftClass::Drop_Payload` infantry descent/render state: falling flags, per-tick altitude/rate update, locomotor/body-sequence involvement, `PARACH` attach/detach, and INI sources for fall rate/chute anim/sound.  
**Non-Scope:** Bridge touchdown layer choice, carrier approach/overfly cadence, passable-cell target selection, and full AnimClass renderer internals beyond PARACH ownership/lifecycle.  
**Confidence:** High for falling flags, fall-rate branch, PARACH attach/detach, body-sequence negative fact, and Rules offsets; Medium for initial spawn-Z source because that requires one more vtable-base Unlimbo pass.  
**Active in YR:** Yes. Allied/American/Yuri paradrop payload drops are active in stock `rulesmd.ini`; the same object falling branch is reached by standard object AI dispatch.

## Working Notes Seed

Target question: After a paradrop payload succeeds, what exact state makes infantry fall and render under a chute in standard YR?

Non-goals: Bridge touchdown, carrier mission approach/exit, target-cell validation, and broad Jumpjet locomotor state-machine behavior.

Evidence needed to mark COMPLETE: decompile plus caller/xref evidence for `Drop_Payload`, `ObjectClass::Unlimbo`, `ObjectClass::AI`, `DetachParachute`; INI/default source plus binary reader address for `ParachuteMaxFallRate`, `NoParachuteMaxFallRate`, `Parachute`, and `ChuteSound`; direct evidence for whether body sequence `Paradrop` is set in this path.

Stop conditions: stop after the post-Unlimbo falling/render slice is proven or each material unknown is explicitly deferred; do not investigate bridge landing or carrier mission timing.

## 1. Overview

After `AircraftClass::Drop_Payload` places a passenger and calls the passenger's parachute Unlimbo path, the infantry remains an ordinary infantry object with its base locomotor. `ObjectClass::Unlimbo @ 0x005F5940` sets the falling flag, creates an attached `PARACH` anim from `Rules+0xBBC`, and stores the anim pointer at `Object+0x88`. `ObjectClass::AI @ 0x005F3E70` then integrates the object's Z using a falling delta at `Object+0x2C`; if the object still has an attached anim marker (`Object+0x84`), the fall-rate clamp comes from `Rules+0x7B8` (`ParachuteMaxFallRate`).

The important correction for Rust is that normal paradropped infantry do not use Jumpjet/Parachute locomotor and do not use artmd `Paradrop=` body frames. The player-visible parachute is the attached `PARACH` AnimClass, while the infantry body remains on the normal infantry animation path until landing.

## 2. Class Layout / Key Offsets

| Offset | Owner | Purpose | Active in YR | Evidence |
|---|---|---|---|---|
| `+0x84` | ObjectClass | owner has attached anim marker; set by `AnimClass::SetOwnerObject`, also used by falling AI to select parachute-rate branch | Yes | `0x00424B50`, `0x005F3E70` |
| `+0x88` | ObjectClass | attached parachute anim pointer | Yes | `0x005F5940`, `0x005F6DA0` |
| `+0x8D` | ObjectClass | falling/down flag; gates the falling branch in `ObjectClass::AI` | Yes | `0x005F5940`, `0x005F3E70` |
| `+0x2C` | ObjectClass | per-tick falling rate/delta; negative means descending | Yes | `0x005F3E70` |
| `+0xA4` | ObjectClass | world Z/location field updated by falling branch | Yes | `0x005F3E70` |
| `+0x195` | AnimClass | loop/lifetime counter byte; landing writes `0` to force chute anim termination | Yes | `0x005F3E70`, `0x00423AC0` |
| `+0xCC` | AnimClass | owner object pointer for attached anims | Yes | `0x00424B50` |
| `Rules+0x7B8` | RulesClass | `ParachuteMaxFallRate` | Yes | `0x005F3E70`, `0x0066D530`, `rulesmd.ini:68` |
| `Rules+0x7BC` | RulesClass | `NoParachuteMaxFallRate` | Conditional: used only if falling without attached anim marker | `0x005F3E70`, `rulesmd.ini:69` |
| `Rules+0xBBC` | RulesClass | `Parachute=PARACH` AnimType pointer | Yes | `0x005F5940`, `0x0066D530`, `rulesmd.ini:564` |
| `Rules+0xBB8` | RulesClass | `BombParachute=PARABOMB` AnimType pointer | Conditional: used for `WhatAmI()==8`, not normal infantry paradrops | `0x005F5940`, `rulesmd.ini:565` |
| `Rules+0x71C` | RulesClass | `ChuteSound` Voc index | Yes | `0x0066ACEE`, `rulesmd.ini:702` |

## 3. Core Logic

### 3.1 Drop success sets up the falling object through vtable Unlimbo

`AircraftClass::Drop_Payload @ 0x00415C60` is live in YR: `AircraftClass::Fire_At @ 0x00415EF8` calls it when the aircraft cargo head at `aircraft+0x118` is non-null, and `Mission_Rescue @ 0x00415960` is a second direct caller. On the paradrop path, `Drop_Payload` pops one passenger, computes the V-pattern target, verifies `Can_Enter_Cell`, asks `CellClass::PlaceInfantryInCell @ 0x00481180` for a subcell coordinate, then calls the passenger vtable slot `+0xE8`. For infantry, existing vtable xrefs bind this slot to `ObjectClass::Unlimbo @ 0x005F5940`.

Active in YR: Yes. Evidence: decompile `0x00415C60`; xrefs to `0x00415C60` from `0x00415EF8` and `0x004159FB`; stock `[ParaDropSpecial]` and `[AmericanParaDropSpecial]` in `rulesmd.ini`.

### 3.2 `ObjectClass::Unlimbo` creates falling state and the PARACH anim

`ObjectClass::Unlimbo @ 0x005F5940` first verifies the coordinate is in the playfield, then writes `Object+0x8D = 1`. It performs the bridge gate already covered by the bridge report, calls the base Unlimbo slot with flag `0x80`, and after a successful base Unlimbo constructs an AnimClass.

For normal paradropped infantry (`WhatAmI() != 8`), it constructs the anim type from `Rules+0xBBC`, which is `[General] Parachute=PARACH`, using constructor flags `0x600`, then stores the returned AnimClass pointer into `Object+0x88`. The `WhatAmI()==8` branch instead uses `Rules+0xBB8` (`BombParachute=PARABOMB`), so it is not the normal infantry path.

After construction, Unlimbo calls `AnimClass::SetOwnerObject @ 0x00424B50` with the falling infantry as owner. That writes the owner's attached-anim marker (`Object+0x84 = 1`), records the owner pointer in the anim (`Anim+0xCC`), converts the anim coordinate to an owner-relative offset, and re-submits the anim to the display layer.

Active in YR: Yes for the normal `PARACH` branch; Conditional for the `PARABOMB` branch. Evidence: decompile `0x005F5940`; `RulesClass::ReadGeneral @ 0x0066D530` reads `Parachute` into `Rules+0xBBC`.

### 3.3 Per-tick falling runs in `ObjectClass::AI`, not locomotor code

`ObjectClass::AI @ 0x005F3E70` exits immediately if `Object+0x8D == 0`. If the flag is set, it:

1. snapshots the current render layer via vtable `+0x78`;
2. reads a height/Z baseline via vtable `+0x1D0`;
3. reads current falling delta from `Object+0x2C`;
4. writes object Z as baseline plus current delta, with remove/put marking around the write if visible;
5. checks effective height via vtable `+0x1C8`;
6. if effective height is `< 1`, snaps height via vtable `+0x1CC(0)`, clears `Object+0x8D`, changes mission to `2`, and writes `Anim+0x195 = 0` if `Object+0x88` is non-null;
7. if still falling, updates the fall-rate delta for the next tick.

The rate update is after the Z integration and landing check. For parachuted infantry, `Object+0x84 != 0`, so the update is `rate -= 1`, then clamp to no more negative than `Rules+0x7B8`. With stock `ParachuteMaxFallRate=-3`, the rate sequence entering ticks is `0, -1, -2, -3, -3...` if the object starts with zero fall delta.

If `Object+0x84 == 0`, the no-chute branch uses `Rules+0x7BC` (`NoParachuteMaxFallRate=-100`) instead. That means `Object+0x8D` means "falling", while `Object+0x84`/attached anim selects the parachute fall-rate branch.

Active in YR: Yes. Evidence: decompile `0x005F3E70`; xrefs show active dispatch from `MissionClass__Mission_Dispatch @ 0x005B3067` and other object AI callers; `RulesClass::ReadGeneral @ 0x0066D530` reads the fall-rate keys.

### 3.4 Chute detaches by killing the AnimClass, then callback clears `Object+0x88`

Landing does not directly zero `Object+0x88`. Instead, `ObjectClass::AI` writes `0` to the attached anim's `+0x195` byte. `AnimClass::AI @ 0x00423AC0` treats this as an exhausted lifetime/loop state and proceeds into the normal destroy path. `AnimClass::Destroy @ 0x004255CA` calls `AnimClass::SetOwnerObject`, and the owner cleanup path ultimately calls `ObjectClass::DetachParachute @ 0x005F6DA0`, which clears `Object+0x88` if the callback anim pointer matches.

Active in YR: Yes. Evidence: decompile `0x005F3E70`, `0x00423AC0`, `0x005F6DA0`; xref to `0x005F6DA0` from `0x00710410`; xref to `AnimClass::SetOwnerObject @ 0x00424B50` from `AnimClass::Destroy @ 0x004255CA`.

### 3.5 No Jumpjet/Parachute locomotor swap on normal paradrops

No function in the verified `Drop_Payload -> ObjectClass::Unlimbo -> ObjectClass::AI` chain constructs `JumpjetLocomotionClass`, calls a piggyback locomotor swap, or changes the passenger locomotor. `JumpjetLocomotionClass::Constructor @ 0x0054AC40` is a real live YR locomotor constructor for Rocketeer/Disc/Kirov-class units, but it is not called from the normal paradrop payload chain.

Active in YR: Yes as a negative fact for normal paradrops; Jumpjet itself is active for Jumpjet units. Evidence: decompiles `0x00415C60`, `0x005F5940`, `0x005F3E70`; Jumpjet constructor `0x0054AC40`; no call/xref from the paradrop chain to `0x0054AC40`.

### 3.6 Normal paradrops do not set infantry body sequence `Paradrop`

The verified post-payload path does not call the infantry body sequence dispatcher. The only body/mission transition in the falling branch is the landing-time mission change `vtable+0x18C(2)`. The artmd `Paradrop=`/`ParadropMoving=` sequence path is in `FootClass::Locomotion_AI @ 0x00520F40`, and it is gated by the infantry type's JumpJet flag plus an active locomotor CLSID comparison against the Jumpjet CLSID. That path selects sequence `0x17` or `0x18` for Jumpjet-style infantry, not for normal paradropped GIs/Conscripts/Initiates that keep Walk locomotion.

Active in YR: Conditional. Active for Jumpjet infantry; No for ordinary paradropped infantry. Evidence: decompile `0x00520F40`, xref from `InfantryClass::AI @ 0x0051BF7B`, Jumpjet CLSID constructor `0x0054AC40`, and absence of sequence calls in `0x00415C60`/`0x005F5940`/`0x005F3E70`.

## 4. INI Keys

| Key | Section | Stock YR value | Binary reader/use | Effect | Active in YR |
|---|---|---|---|---|---|
| `ParachuteMaxFallRate` | `[General]` | `-3` | read at `0x0066D530` to `Rules+0x7B8`; used at `0x005F3E70` | clamp for falling objects with attached anim marker | Yes |
| `NoParachuteMaxFallRate` | `[General]` | `-100` | read at `0x0066D530` to `Rules+0x7BC`; used at `0x005F3E70` | clamp for falling objects without attached anim marker | Conditional |
| `Parachute` | `[General]` | `PARACH` | read at `0x0066D530` to `Rules+0xBBC`; used at `0x005F5940` | normal infantry chute AnimType | Yes |
| `BombParachute` | `[General]` | `PARABOMB` | read at `0x0066D530` to `Rules+0xBB8`; used at `0x005F5940` | bomb/object type 8 chute AnimType | Conditional, not normal infantry |
| `ChuteSound` | `[AudioVisual]` | `ParachuteDrop` | read at `0x0066ACEE` to `Rules+0x71C`; drop path calls `VocClass::PlayAt` after successful Unlimbo | played at successful chute/drop point | Yes |
| `[PARACH] Rate/LoopStart/LoopEnd/LoopCount/AltPalette/ZAdjust` | `artmd.ini` | `400/20/39/30/yes/-10` | `AnimTypeClass::ReadINI @ 0x00427D00`; render/AI in `AnimClass` | frame timing and draw behavior for attached chute anim | Yes |
| `Paradrop=` body sequence keys | infantry sequence sections | many, e.g. `E1Sequence Paradrop=292,1,0` | consumed through Jumpjet-gated sequence path at `0x00520F40` | not used for ordinary paradropped infantry | Conditional; No for normal paradrops |

## 5. Integration Points

| Integration | Finding | Active in YR | Evidence |
|---|---|---|---|
| Payload success | `Drop_Payload` calls passenger vtable `+0xE8`; success then plays chute sound and updates carrier landing fields | Yes | `0x00415C60`; xref from `0x00415EF8` |
| Spawn/falling state | `ObjectClass::Unlimbo` sets `+0x8D`, creates `PARACH`, stores `+0x88`, attaches owner | Yes | `0x005F5940`, `0x00424B50` |
| Tick update | `ObjectClass::AI` integrates Z before updating fall rate for the next tick | Yes | `0x005F3E70`; xrefs from active object AI dispatch |
| Chute lifetime | Landing forces anim lifetime byte to zero; normal AnimClass cleanup clears owner pointer later | Yes | `0x005F3E70`, `0x00423AC0`, `0x005F6DA0` |
| Body animation | `Paradrop` body sequence dispatch is Jumpjet-gated, not normal paradrop-gated | Conditional | `0x00520F40`, `0x0054AC40` |

## 6. Current Rust Implementation Status

Current Rust already has the right high-level state surface:

- `src/sim/movement/parachute_descent.rs` has per-entity descent state, stock `-3` clamp support, and first-tick integration-before-rate-update tests.
- `src/sim/aircraft/drop_payload.rs` calls `begin_parachute_descent` after successful passenger placement and emits `SimSoundEvent::ChuteSound`.
- `src/app_chute_anim.rs` and `src/app_instances/overlays.rs` implement polling-based PARACH anim lifecycle/rendering tied to `entity.parachute_state`.
- `src/rules/ruleset.rs` parses `ParachuteMaxFallRate`, `Parachute`, and render config for `PARACH`.

Mismatch found:

- `begin_parachute_descent` applies `OverrideKind::Parachute` and changes locomotor kind/layer. The binary does not swap or piggyback a locomotor for normal paradrops; it uses object-level falling state over the existing base locomotor.
- `begin_parachute_descent` switches the body animation to `SequenceKind::Paradrop` and landing resets it to `Stand`. The binary does not set the normal paradropped infantry body sequence to `Paradrop`; that sequence path is Jumpjet-gated.

No Rust code was modified in this investigation.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `AircraftClass::Drop_Payload @ 0x00415C60` post-Unlimbo setup | verified | decompile + xrefs from `0x00415EF8`, `0x004159FB` | exact hidden arguments to `VocClass::PlayAt` are decompiler-poor but Rules reader identifies `ChuteSound` |
| `ObjectClass::Unlimbo @ 0x005F5940` falling/parachute branch | verified | decompile + vtable xrefs | base vtable `+0xD8` internals not drained |
| `ObjectClass::AI @ 0x005F3E70` falling branch | verified | decompile + active dispatch xrefs | none for rate/order |
| `ObjectClass::DetachParachute @ 0x005F6DA0` | verified | decompile + xref from `0x00710410` | none |
| `AnimClass::SetOwnerObject @ 0x00424B50` | verified | decompile + xref from Unlimbo/Destroy | none for attachment marker |
| `AnimClass::AI @ 0x00423AC0` chute lifetime reaction | touched-not-exhausted | decompile | full AnimClass loop semantics covered by `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md` |
| `FootClass::Locomotion_AI @ 0x00520F40` body sequence path | verified for negative fact | decompile + xref from `InfantryClass::AI` | none for ordinary paradrop negative |
| `RulesClass::ReadGeneral @ 0x0066D530` | verified | decompile | none for listed keys |
| `RulesClass::ReadAudioVisual @ 0x0066ACEE` | verified | decompile | none for `ChuteSound` |
| Initial spawn Z source | deferred | `0x00415C60`, `0x00481180`, `0x005F5940` touched | need vtable `+0xD8` base Unlimbo pass to prove exact initial airborne Z assignment |
| Bridge touchdown | deferred | explicit non-scope | already covered by `PARACHUTE_LANDING_BRIDGE_LAYER_SELECT_GHIDRA_REPORT.md` |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `Drop_Payload` on the live standard YR paradrop path? -> Yes, `AircraftClass::Fire_At` calls it when cargo exists.` (evidence: `0x00415EF8 -> 0x00415C60`; `rulesmd.ini` paradrop SW entries)
- `[RESOLVED] OQ-2 - What flag makes ObjectClass::AI run falling logic? -> `Object+0x8D != 0`.` (evidence: `0x005F5940`, `0x005F3E70`)
- `[RESOLVED] OQ-3 - What selects parachute vs no-parachute fall rate? -> `Object+0x84`, the attached-anim marker, selects `Rules+0x7B8`; false selects `Rules+0x7BC`.` (evidence: `0x00424B50`, `0x005F3E70`)
- `[RESOLVED] OQ-4 - Where is rate updated? -> In `ObjectClass::AI`, after Z integration and landing check.` (evidence: `0x005F3E70`)
- `[RESOLVED] OQ-5 - What is the stock parachute fall-rate default? -> `ParachuteMaxFallRate=-3`, read to `Rules+0x7B8`.` (evidence: `0x0066D530`, `rulesmd.ini:68`)
- `[RESOLVED] OQ-6 - What happens without attached chute marker? -> No-chute falling uses `NoParachuteMaxFallRate=-100`.` (evidence: `0x005F3E70`, `rulesmd.ini:69`)
- `[RESOLVED] OQ-7 - Which anim is created for normal paradropped infantry? -> `Rules+0xBBC`, stock `PARACH`.` (evidence: `0x005F5940`, `0x0066D530`, `rulesmd.ini:564`)
- `[RESOLVED] OQ-8 - Is `PARABOMB` used for normal infantry? -> No, only the `WhatAmI()==8` branch uses `Rules+0xBB8`.` (evidence: `0x005F5940`, `rulesmd.ini:565`)
- `[RESOLVED] OQ-9 - Does normal paradrop construct JumpjetLocomotion? -> No, no constructor/piggyback call in the verified chain.` (evidence: `0x00415C60`, `0x005F5940`, `0x005F3E70`, `0x0054AC40`)
- `[RESOLVED] OQ-10 - Is artmd `Paradrop=` body sequence used for normal paradropped infantry? -> No; the sequence path is Jumpjet-gated in `FootClass::Locomotion_AI`.` (evidence: `0x00520F40`, `0x0054AC40`)
- `[RESOLVED] OQ-11 - How is PARACH attached? -> `AnimClass::SetOwnerObject` sets owner marker, owner pointer, relative offset, and re-submits display layer.` (evidence: `0x00424B50`)
- `[RESOLVED] OQ-12 - How is PARACH removed on landing? -> Landing writes `Anim+0x195=0`; Anim cleanup later clears `Object+0x88` via detach callback.` (evidence: `0x005F3E70`, `0x00423AC0`, `0x005F6DA0`)
- `[RESOLVED] OQ-13 - Is `ChuteSound` parsed? -> Yes, `[AudioVisual] ChuteSound=ParachuteDrop` is read to `Rules+0x71C`.` (evidence: `0x0066ACEE`, `rulesmd.ini:702`)
- `[RESOLVED] OQ-14 - Is first-tick order instant `-3` or ramped? -> Ramped; current delta is integrated first, then decremented/clamped for next tick.` (evidence: `0x005F3E70`)
- `[DEFERRED] OQ-15 - What exact base Unlimbo code assigns the initial airborne Z?` (category: `requires-different-system-context`; reason: vtable `+0xD8` base Unlimbo target was not drained in this slice; next-step-if-pursued: resolve infantry vtable `+0xD8` and trace how flag `0x80` initializes Z/falling height)
- `[DEFERRED] OQ-16 - What happens if a chute anim is destroyed mid-descent before touchdown?` (category: `bounded-cost-too-high`; reason: requires AnimClass destruction/user kill path trace outside normal successful paradrop; next-step-if-pursued: trace owner death and anim destroy ordering)
- `[DEFERRED] OQ-17 - Save/load serialization of `+0x8D`, `+0x84`, `+0x88`, and `+0x2C`.` (category: `out-of-scope`; reason: no save/load scope requested; next-step-if-pursued: audit ObjectClass save/load fields)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Falling is object-level: `+0x8D` gates falling, existing base locomotor is not swapped to Jumpjet/Parachute | `0x00415C60`, `0x005F5940`, `0x005F3E70`; no call to `0x0054AC40` | mismatch: `OverrideKind::Parachute` changes locomotor kind/layer | `src/sim/movement/parachute_descent.rs`, `src/sim/movement/locomotor.rs` | model falling/parachuting as entity/object state without changing base locomotor identity | `paradrop_keeps_walk_locomotor_identity_during_descent` | Do not implement a Parachute locomotor or Jumpjet piggyback for normal paradrops |
| Attached anim marker selects parachute fall rate; `ParachuteMaxFallRate=-3` only applies while chute marker exists | `0x00424B50`, `0x005F3E70`, `0x0066D530` | mostly matched for rate, but Rust ties rate to `parachute_state` not explicit anim-attached marker | `src/sim/movement/parachute_descent.rs`, `src/app_chute_anim.rs` | keep object-level state and PARACH lifecycle coherent; if chute is absent/destroyed, no-chute behavior should be considered | `parachute_rate_uses_attached_chute_marker` | Do not treat `IsFallingDown` alone as proof of chute-slow fall |
| Z integrates before rate update; landing check occurs before next-tick rate update | `0x005F3E70` | matched by current tests | `src/sim/movement/parachute_descent.rs` | preserve ramp `0,-1,-2,-3` and first-tick no-move behavior | existing `test_3tick_rate_ramp`; add `paradrop_first_tick_integrates_before_rate_update` | Do not initialize rate directly to `-3` |
| Landing kills the chute anim by forcing anim lifetime byte to zero; object pointer clears later through detach callback | `0x005F3E70`, `0x00423AC0`, `0x005F6DA0` | Rust polling removal is acceptable shape, but should not depend on PARACH `LoopCount` expiring | `src/app_chute_anim.rs`, `src/app_instances/overlays.rs` | remove PARACH when descent state clears/entity dies; do not wait for 30 loops | `parachute_anim_removed_on_landing_before_loopcount_expires` | Do not use `[PARACH] LoopCount=30` as normal lifetime |
| Normal paradropped infantry do not switch body animation to `Paradrop`; artmd `Paradrop=` is Jumpjet-gated | `0x00520F40`, absence in `0x00415C60/0x005F5940/0x005F3E70` | mismatch: Rust switches to `SequenceKind::Paradrop` on attach and `Stand` on landing | `src/sim/movement/parachute_descent.rs`, `src/sim/animation.rs` | leave infantry body sequence unchanged during normal descent; PARACH anim supplies visible chute | `paradropped_gi_keeps_body_sequence_while_chute_attached` | Do not parse/apply infantry `Paradrop=` frames for normal paradrop SW |
| `ChuteSound` is parsed from `[AudioVisual]` and plays after successful drop/Unlimbo | `0x0066ACEE`, `0x00415C60`, `rulesmd.ini:702` | current Rust emits `SimSoundEvent::ChuteSound` after attach success | `src/sim/aircraft/drop_payload.rs`, `src/app_sim_tick.rs` | keep sound emission success-gated and resolved through Rules audio config | `drop_payload_plays_chute_sound_only_after_successful_attach` | Do not play sound on failed Unlimbo/requeued cargo |

Stale Docs / Follow-up Docs:

- Replace any claim that normal paradropped infantry "use Jumpjet/Parachute locomotor" with: "Normal paradropped infantry keep their base locomotor; falling is object-level state in `ObjectClass::AI`, with the attached `PARACH` anim marker selecting the parachute fall-rate clamp."
- Replace any claim that normal paradrops "use artmd `Paradrop=` body frames" with: "The `Paradrop`/`ParadropMoving` body sequence path is Jumpjet-gated in `FootClass::Locomotion_AI`; normal paradropped infantry render the attached `PARACH` anim over their ordinary body pose."

## 10. Negative Facts / Do Not Do

- Do not create a separate Parachute locomotor for normal paradropped infantry.
- Do not piggyback or replace Walk locomotion with Jumpjet locomotion for normal paradrops.
- Do not switch ordinary paradropped GI/Conscript/Initiate body sequence to `Paradrop`.
- Do not use `[PARACH] LoopCount=30` as the normal chute lifetime.
- Do not apply `NoParachuteMaxFallRate=-100` while the attached PARACH marker is present.
- Do not play `ChuteSound` if `Drop_Payload` fails and requeues the passenger.

## 11. Remaining Uncertainty

The exact source of the initial airborne Z/height assigned by the base Unlimbo slot (`vtable+0xD8`) remains unresolved. This report proves where per-tick descent and fall-rate integration occur after `ObjectClass::Unlimbo` succeeds, but it does not fully prove whether initial Z comes from carrier altitude, a fixed spawn height, or a base Unlimbo special case tied to flag `0x80`. Current Rust assumes carrier altitude; that is plausible but should be verified before treating descent duration as final parity.

Mid-descent owner death/chute destruction cleanup was not traced. The normal landing cleanup path is verified.

## 12. Rust Test Name Proposals

- `paradrop_keeps_walk_locomotor_identity_during_descent`
- `paradropped_gi_keeps_body_sequence_while_chute_attached`
- `parachute_rate_uses_attached_chute_marker`
- `paradrop_first_tick_integrates_before_rate_update`
- `parachute_anim_removed_on_landing_before_loopcount_expires`
- `drop_payload_plays_chute_sound_only_after_successful_attach`
- `drop_payload_requeue_does_not_spawn_chute_or_sound`

## Sources

- Ghidra decompiled/read this session: `AircraftClass::Drop_Payload @ 0x00415C60`; `AircraftClass::Fire_At @ 0x00415EF8`; `ObjectClass::Unlimbo @ 0x005F5940`; `ObjectClass::AI @ 0x005F3E70`; `ObjectClass::DetachParachute @ 0x005F6DA0`; `AnimClass::SetOwnerObject @ 0x00424B50`; `AnimClass::AI @ 0x00423AC0`; `RulesClass::ReadGeneral @ 0x0066D530`; `RulesClass::ReadAudioVisual @ 0x0066ACEE`; `CellClass::PlaceInfantryInCell @ 0x00481180`; `FootClass::Locomotion_AI @ 0x00520F40`; `JumpjetLocomotionClass::Constructor @ 0x0054AC40`.
- Ghidra xrefs checked: `0x00415C60`, `0x005F5940`, `0x005F3E70`, `0x005F6DA0`, `0x00424B50`, `0x00520F40`.
- Prior docs referenced: `PARADROP_SUPERWEAPON_GHIDRA_REPORT.md`; `PARACHUTE_SHP_RENDERING_GHIDRA_REPORT.md`; `PARACHUTE_LANDING_BRIDGE_LAYER_SELECT_GHIDRA_REPORT.md`; `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini` (`ParachuteMaxFallRate`, `NoParachuteMaxFallRate`, `Parachute`, `BombParachute`, `ChuteSound`, paradrop SW entries); `ini/artmd.ini` (`[PARACH]`, infantry `Paradrop=` sequence keys).
- Rust scanned: `src/sim/movement/parachute_descent.rs`; `src/sim/aircraft/drop_payload.rs`; `src/sim/aircraft/paradrop_mission.rs`; `src/app_chute_anim.rs`; `src/app_instances/overlays.rs`; `src/rules/ruleset.rs`.
