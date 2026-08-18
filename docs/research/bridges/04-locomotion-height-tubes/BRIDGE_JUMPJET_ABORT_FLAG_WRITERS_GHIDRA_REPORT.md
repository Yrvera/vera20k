# Bridge Jumpjet Abort Flag Writers - Ghidra Research Report

**Address(es):** `0x0054AEC0` (`JumpjetLocomotionClass::Process`), `0x0054C550` (state 4 descend/land), `0x0054CA90` (state 5 abort/emergency), writer sites listed below  
**Confidence:** HIGH for direct byte writers and state 5 gate shape; MEDIUM-HIGH for semantic names; MEDIUM for normal-play frequency  
**Active in YR:** Yes, conditional. The relevant paths are standard YR Jumpjet, EMP, Chrono/locomotor-piggyback, and Magnetron-capable code paths. Some same-displacement hits are TechnoType/UI false positives and are not Foot/Techno runtime flag writers.

## 1. Concise Verified Summary

Jumpjet state 5 is entered when the linked object has byte `+0x425` set while it is airborne and its Jumpjet state is neither 5 nor 6. Binary evidence supports naming `+0x425` as a falling/crashing request flag rather than a normal landing flag: it is set by `FootClass__ReceiveEMP`, set by the Chrono/locomotor-piggyback deploy cleanup path, read by Fly/Jumpjet/Foot logic as a falling/crash state, and cleared only after Jumpjet state 4 completes a successful landing/finalization.

Byte `+0x427` is not the state 5 trigger. It is a separate recovery-landing arm flag. State 5 only performs its `Can_Enter_Cell(candidate, -1, -1, 0, 1)` recovery probe when `+0x427 != 0` and either altitude is at/below ground or the falling object crosses the bridge-deck altitude plane. EMP sets `+0x425` but does not set `+0x427`, so EMP-driven fall/crash does not get the same recovery landing probe.

Byte `+0x6AD` is a FootClass runtime flag set by `TechnoClass__PerformDeploy` during locomotor piggyback setup. In this cluster it behaves as a locomotor-piggyback/deploy-in-progress flag, not as a native Jumpjet movement flag. It changes Jumpjet state 4/5 handling and many non-Jumpjet systems gate actions while it is set. Same-offset hits inside `TechnoTypeClass` are a different class layout and must not be treated as writes to the linked object.

The bridge-specific state 5 parity invariant is narrow but real: when a falling Jumpjet with `+0x427` armed crosses `ground_height + g_JumpjetBridgeAltitudeThreshold` over a bridge cell, state 5 may run the same candidate-only `Can_Enter_Cell(..., -1, -1, 0, 1)` probe that it would otherwise only run at ground. This can decide whether a Magnetron/locomotor-piggyback dropped object recovers on the bridge deck or continues to terminal crash/failure.

## 2. Key Offsets And Proposed Semantic Names

| Offset | Proposed research name | Binary evidence | Confidence |
|---|---|---|---|
| linked object `+0x425` | `is_falling_or_crashing` / `falling_crash_requested` | Set by EMP (`0x004DEC7F`), set by deploy/piggyback cleanup (`0x0070FF25`), read by Jumpjet Process to enter state 5 (`0x0054AF33`), read by Fly locomotor to apply falling tilt (`0x004CF6A5`), Foot AI mirrors it to `+0x426` for sound-state transitions (`0x004DACDF`..`0x004DADBE`) | HIGH for behavior, MEDIUM-HIGH for name |
| linked object `+0x426` | prior/latched falling state | Techno constructor initializes to 0, Foot AI compares `+0x425` vs `+0x426`, then copies `+0x425` into `+0x426` after starting/releasing falling sounds | HIGH |
| linked object `+0x427` | `fall_recovery_landing_armed` | Set with `+0x425` by `BuildingClass__DeployUnit_ChronoWarp`, cleared with `+0x425` by Jumpjet state 4 landing finalization, read by Jumpjet state 5 before any candidate landing probe | HIGH for behavior, MEDIUM-HIGH for name |
| linked object `+0x428` | recovery/source building pointer | Set by `BuildingClass__DeployUnit_ChronoWarp` to the building `this`, read by state 5 cleanup/fallback paths | HIGH |
| linked object `+0x42C` | recovery/source owner or auxiliary source pointer | Set by `BuildingClass__DeployUnit_ChronoWarp` to building field `+0x21C` (`param_1[0x87]`), read by state 5 fallback paths | MEDIUM-HIGH |
| linked object `+0x6AD` | `locomotor_piggyback_active` / `deploy_locomotor_swap_active` | Foot constructor initializes it to 0; `TechnoClass__PerformDeploy` sets it to 1 after a temporary locomotor swap; Jumpjet state 4/5 special-case it; fire/cloak/AI code suppresses normal actions while it is set | HIGH for behavior, MEDIUM for exact name |
| linked object `+0x6AE` | post-landing/restored marker, not investigated here | State 4 writes `+0x6AE = 1` near finalization; deploy/chrono paths also touch it. It is adjacent but out of scope. | LOW-MEDIUM |

## 3. Writer Matrix

### 3.1 Runtime object byte `+0x425`

| Address | Function | Write value | Conditions | Active in standard YR? | Notes |
|---|---|---:|---|---|---|
| `0x006F2FF9` | `TechnoClass__Constructor @ 0x006F2FF0` | `0` | Constructor initialization, adjacent to `+0x426` and `+0x427` zeroing | Yes | Binary shows constructor initializes `+0x425`, `+0x426`, and `+0x427` to zero. |
| `0x004DEC7F` | `FootClass__ReceiveEMP @ 0x004DEB50` | `1` | Runs only after altitude vfunc `+0x1C8` returns `> 0`. If the object is already airborne/height-positive, EMP sets the falling/crash flag, issues vfunc `+0x274(3)`, calls vfunc `+0x3A0`, EMPs passengers, and may randomize fall rotation for non-class-`0xF` objects. | Conditional. EMP code exists and EMP INI data exists, but `EMPulseSpecial` is commented as disabled in shipped rules. EMP-style warheads/effects remain in standard data but normal skirmish frequency is low unless a map/mod/weapon uses them. | Assembly bytes at `0x004DEC7F`: `C6 86 25 04 00 00 01`, i.e. `mov byte ptr [esi+0x425], 1`. |
| `0x0070FF25` | `BuildingClass__DeployUnit_ChronoWarp @ 0x0070FEE0` | `1` | If `building+0x2AC` linked unit exists, is a Foot/Techno object (`flags & 4`), and altitude vfunc `+0x1C8` returns `> 0`. Then writes `+0x425=1`, `+0x427=1`, `+0x428=building`, `+0x42C=building+0x21C`, calls linked vfuncs `+0x3A0` and `+0x480(0,1)`, clears building link. | Yes, conditional. Used by Chrono/locomotor-piggyback deploy infrastructure; this is the writer that arms state 5 recovery. Magnetron reaches the same shared piggyback infrastructure through `IsLocomotor` warhead flow, with Jumpjet CLSID from `[LocomotorBeam]`. | Assembly bytes at `0x0070FF25`: `C6 81 25 04 00 00 01`. |
| `0x0054CA12` | `JumpjetLocomotionClass_State4_DescendLand @ 0x0054C550` | `0` | State 4 successful landing/finalization block after setting state 0, restoring occupancy/crate pickup, setting `+0x6AE=1`, and clearing `+0x427`. | Yes. Jumpjet state 4 is active in standard YR Jumpjet state machine. | Assembly bytes at `0x0054CA12`: `C6 82 25 04 00 00 00`. This is the observed runtime clear after successful recovery/landing. |

False positives and non-writer hits:

- `0x004A7464` (`DiskLaserClass__AI`) reads source object `+0x425` and aborts the disk laser sequence if set.
- `0x004CF6A5` (`FlyLocomotionClass__Move_To`) reads `+0x425`; if set, it applies falling/crash pitch/roll behavior and forces output flags to `0xFFFFFFFF`.
- `0x004DAA40`..`0x004DADBE` (`FootClass__AI`) reads `+0x425`, compares it with `+0x426`, plays/release falling-related sound events, and latches `+0x426 = +0x425`.
- `0x0054AF33` (`JumpjetLocomotionClass::Process`) reads `+0x425` as the state 5 entry trigger.
- `0x006580A1`, `0x00559083`, and similar UI/data hits are immediate values or unrelated code/data, not object-field access.

### 3.2 Runtime object byte `+0x427`

| Address | Function | Write value | Conditions | Active in standard YR? | Notes |
|---|---|---:|---|---|---|
| `0x006F3005` | `TechnoClass__Constructor @ 0x006F2FF0` | `0` | Constructor initialization, immediately after `+0x425`/`+0x426` initialization | Yes | Default is unarmed. |
| `0x0070FF32` | `BuildingClass__DeployUnit_ChronoWarp @ 0x0070FEE0` | `1` | Same branch as `+0x425=1`: linked unit exists, is Foot/Techno, and is above ground (`vfunc +0x1C8 > 0`) | Yes, conditional | This is the only direct runtime setter found. It arms state 5's recovery `Can_Enter_Cell` probe. |
| `0x0054CA08` | `JumpjetLocomotionClass_State4_DescendLand @ 0x0054C550` | `0` | State 4 finalization block, immediately before clearing `+0x425` | Yes | State 4 clears recovery arm first, then clears falling/crash request. |

False positives and non-writer hits:

- `0x0054CC3B` (`JumpjetLocomotionClass_State5_AbortEmergencyLanding`) reads `+0x427` and gates the candidate landing/recovery check.
- `0x0069951C` and `0x00559083` are unrelated immediate/UI/data occurrences.

### 3.3 Runtime object byte `+0x6AD`

| Address | Function | Write value | Conditions | Active in standard YR? | Notes |
|---|---|---:|---|---|---|
| `0x004D3414` | `FootClass__Constructor @ 0x004D31E0` | `0` | Constructor initialization in the FootClass extension region | Yes | Runtime Foot object default. Assembly bytes include `88 9E AD 06 00 00` (`mov [esi+0x6AD], bl`) with `bl=0` in constructor context. |
| `0x00710352` | `TechnoClass__PerformDeploy @ 0x00710000` | `1` | After successful locomotor piggyback setup: original/temporary locomotor handling, base pointer swap, `piVar1[0xAB]` linked object assignment, vfunc `+0x480(piVar1,1)`, vfunc `+0x150()`, optional `FUN_006EA870`, then writes `target+0x6AD=1`. | Yes, conditional | This is the important gameplay setter. It is shared by Chrono/locomotor-piggyback infrastructure and therefore by Magnetron's `IsLocomotor` Jumpjet override path. |

False positives and non-runtime-object hits:

- `0x0071114A` and many `0x0071....` hits are `TechnoTypeClass` constructor/ReadINI accesses to `TechnoTypeClass+0x6AD`, a different class layout. They must not be interpreted as writes to linked object byte `+0x6AD`.
- Numerous `JumpjetLocomotionClass` hits (`0x0054AFE2`, `0x0054C56E`, `0x0054CAC6`, `0x0054CFDE`, etc.) read object `+0x6AD` to alter state behavior. They are not writers.
- Cloak/fire/AI readers such as `TechnoClass__CanAutoCloak @ 0x006FBF6D` and `TechnoClass__GetFireError @ 0x006FC1AA` use `+0x6AD` to block or alter normal behavior while the locomotor-piggyback/deploy state is active.

No direct runtime clear of object `+0x6AD` besides constructor initialization was found in the immediate byte-write scan. That does not prove the flag can never be cleared indirectly through object reconstruction, save/load, or a wider memory copy; it means no direct `mov byte ptr [object+0x6AD], 0` writer was found in the direct displacement scan used for this report.

## 4. State 5 Trigger Flow

### 4.1 Process-level entry

`JumpjetLocomotionClass::Process @ 0x0054AEC0` checks:

| Step | Binary condition | Effect |
|---|---|---|
| 1 | linked object `+0x425 != 0` | Candidate falling/crash state trigger |
| 2 | current Jumpjet state `!= 5` and `!= 6` | Do not re-enter abort or terminal state |
| 3 | linked object altitude vfunc `+0x1C8() > 0` | Only enter state 5 while above ground |
| 4 | if linked object `+0x6AD == 0`, accept immediately | Normal falling/crash state can enter state 5 |
| 5 | if `+0x6AD != 0`, compare current/derived coordinates | Piggyback/deploy path can defer or update destination fields until coordinate condition is met |
| 6 | on accepted trigger | write Jumpjet instance `+0x80 = -5`, state `+0x50 = 5`; if object class code is `0xF`, call vfunc `+0x558(0x22,0,0)` |

### 4.2 State 5 candidate-check gate

`JumpjetLocomotionClass_State5_AbortEmergencyLanding @ 0x0054CA90` computes a local bridge-plane-crossing byte before the candidate probe:

| Gate | Required value | Effect |
|---|---|---|
| current cell has bridge flag `cell+0x140 & 0x100` | true | Without a bridge cell, bridge crossing byte remains 0 |
| previous/current Z (`iVar11` in decompile) | `>= ground_height + DAT_00ABC5DC` | Must start at or above the bridge deck plane |
| next/local Z after descent (`local_10`) | `< ground_height + DAT_00ABC5DC` | Must cross downward through the bridge deck plane |
| linked object `+0x427` | nonzero | Recovery landing probe is armed |
| altitude vfunc `+0x1C8()` | `<= 0` OR bridge crossing byte set | Probe may run at ground, or early at bridge deck crossing |

When all state 5 gates pass, the call at `0x0054CE34` is:

```text
linked_object->Can_Enter_Cell(candidate_current_cell, -1, -1, 0, 1)
```

The candidate comes from the linked object's current coordinate, not the original destination. This matches prior reports: direction `-1`, height `-1`, parent `0`, arg5 `1` use the candidate-only bridge traversal fallback path.

### 4.3 State 5 outcomes

| Result path | Conditions | Observable consequence |
|---|---|---|
| Recovery to state 4 | `+0x427` armed, ground or bridge-plane gate opens, `Can_Enter_Cell` and follow-up passability checks succeed | Jumpjet transitions to state 4, calls linked vfunc `+0x480(0,1)`, and can complete a landing/recovery. |
| Continue falling | Gate not reached yet, or `+0x6AD` special case defers terminal cleanup | State remains in abort/emergency descent. |
| Terminal state 6 | Recovery fails or landing is not armed/allowed when required | State set to 6, vertical motion fields cleared, voice/event `0x117C` emitted, source pointers `+0x428/+0x42C` cleared. |

## 5. Gameplay Events That Enter State 5

| Event | Writes | Uses state 5? | Live-YR confidence | Notes |
|---|---|---|---|---|
| EMP applied to airborne Foot object | `+0x425=1`, not `+0x427` | Yes, if object uses Jumpjet Process and is above ground | Conditional | EMP code is in binary and EMP data exists, but standard YR comments mark EMPulse special disabled. This is not a common standard skirmish Rocketeer trigger unless another live weapon/map uses EMP. |
| Chrono/locomotor-piggyback cleanup while linked unit is airborne | `+0x425=1`, `+0x427=1`, `+0x428/+0x42C` source pointers | Yes | Yes, conditional | This is the canonical recovery-landing armed path. |
| Magnetron `LocomotorBeam` forced lift | Through shared `IsLocomotor`/piggyback infrastructure, using Jumpjet CLSID | Yes, through the same Jumpjet state 5 path after piggyback end/drop | HIGH for shared infrastructure, MEDIUM-HIGH for exact per-scenario timing | `[LocomotorBeam]` in `rulesmd.ini` has `IsLocomotor=yes` and `Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5}`. Prior `MAGNETRON_SYSTEM_GHIDRA_REPORT.md` maps this to the same piggyback path. |
| Native Jumpjet normal arrival/landing | No `+0x425` writer in normal state 2/3/4 path found | No, normally state 4 | HIGH | Native landing uses state 4. State 5 is not the normal Rocketeer/Siege Chopper/Floating Disk/Kirov idle-arrival landing path. |
| Native Jumpjet killed/crashing by normal damage | Not fully enumerated in this report | Possibly, if death/damage code calls EMP/falling writer or a separate crash writer not found by direct byte scan | LOW-MEDIUM | This report found no extra direct `+0x425` setter beyond EMP and deploy/piggyback. Death handling may still route through a vfunc or broader state mutation. Needs separate death/crash investigation if implementation requires it. |

## 6. Magnetron / Forced Lift Answer

Magnetron does reuse the same Jumpjet state 5 path, but not because the Magnetron unit itself is a Jumpjet. The Magnetron weapon path uses `IsLocomotor=yes` on `[LocomotorBeam]` and supplies the Jumpjet locomotor CLSID. The shared locomotor-piggyback infrastructure installs a temporary Jumpjet locomotor on the target and `TechnoClass__PerformDeploy` sets object `+0x6AD=1` during that swap. When the piggyback ends while the target is airborne, `BuildingClass__DeployUnit_ChronoWarp`/shared deploy cleanup can set `+0x425=1` and `+0x427=1`, arming Jumpjet state 5 recovery.

Binary-verified pieces:

- `TechnoClass__PerformDeploy @ 0x00710000` sets target object `+0x6AD=1` at `0x00710352`.
- `BuildingClass__DeployUnit_ChronoWarp @ 0x0070FEE0` sets linked object `+0x425=1` and `+0x427=1` when altitude is positive.
- `JumpjetLocomotionClass::Process @ 0x0054AEC0` enters state 5 from `+0x425`.
- `JumpjetLocomotionClass_State5_AbortEmergencyLanding @ 0x0054CA90` uses `+0x427` and bridge-plane crossing before the recovery `Can_Enter_Cell` probe.

Inference:

- The exact user-visible Magnetron drop timing depends on the piggyback `Is_Ok_To_End`/release path, which the prior Magnetron report marks as not fully traced. The binding to Jumpjet state 5 is strong; exact tick frequency is scenario-dependent.

## 7. Bridge-Deck Crossing Frequency And Player Visibility

State 5's bridge-deck crossing gate matters only under a narrow combination:

1. The object is in Jumpjet state 5, not normal state 4 landing.
2. `+0x427` is armed, which the writer scan ties to the locomotor-piggyback/chrono recovery path, not EMP-only falling.
3. The object is above a bridge cell.
4. Its descent crosses `ground_height + DAT_00ABC5DC` before reaching ground.
5. The candidate `Can_Enter_Cell(..., -1, -1, 0, 1)` outcome differs between bridge deck and ground/under-bridge occupancy.

For normal native Rocketeer/Siege Chopper/Floating Disk/Kirov/Hornet gameplay, this is uncommon:

- Rocketeer/Floating Disk/Kirov-style Jumpjet units with `BalloonHover=yes` normally hover and do not voluntarily land through state 5.
- Siege Chopper has Jumpjet locomotor in YR data, but its normal movement/landing/deploy flow does not require `+0x425`.
- Hornet/ORCA-style aircraft usually use Fly or other aircraft locomotors, not this state 5 Jumpjet abort path.

For Magnetron/forced-lift over bridges, this can be player-visible whenever a lifted vehicle is released above or near a bridge deck. The outcome can decide whether the dropped vehicle lands/recover-checks on the bridge deck, collides with deck occupants, or continues into terminal crash/failure behavior. Frequency in standard skirmish is conditional but not TS-dead: Magnetrons are standard YR units, bridges are common map features, and `[LocomotorBeam]` is standard YR data.

## 8. Rust Invariants, Research-Only

Future Rust should preserve these output-level invariants:

1. Native Jumpjet normal landing and abort/emergency falling must remain distinct states. Normal arrival should not accidentally take the state 5 recovery path unless an abort/falling flag equivalent is set.
2. A falling/crashing request equivalent to object `+0x425` must be expressible separately from a recovery-landing-arm equivalent to object `+0x427`.
3. EMP-style airborne falling may set the falling/crashing request without arming recovery landing. Do not assume every state 5 fall may run `Can_Enter_Cell`.
4. Locomotor-piggyback/forced-lift state equivalent to object `+0x6AD` must be expressible separately from native Jumpjet state. It gates special state 4/5 behavior and other Techno actions.
5. State 5 recovery checks must be allowed to fire at bridge-deck plane crossing, not only at `altitude <= 0`.
6. The bridge-deck state 5 candidate check must have the binary call shape: candidate/current cell, direction `-1`, height `-1`, parent/current `0`, arg5 `1`, with candidate-only bridge traversal seeding inside the callee.
7. The state 5 recovery candidate must be derived from the linked object's current coordinate, not the original move destination.
8. A successful recovery check must be able to transition to state 4 landing/finalization; a failed or unarmed check must be able to continue to terminal state 6 crash/failure.
9. Bridge deck occupancy/layer decisions for this check must be made in sim/map movement logic, not render/UI/sidebar/audio/net, preserving the `sim/` layering invariant.

## 9. Current Rust Expressiveness Snapshot

This is a read-only scan, not an implementation plan.

| Area | Current Rust evidence | Expressiveness gap |
|---|---|---|
| Jumpjet state shape | `src/sim/movement/air_movement.rs` uses `AirMovePhase::{Landed, Ascending, Cruising, Hovering, Descending}`; `jumpjet_movement.rs` descends to `Landed` using normal or crash speed. | No explicit state 5 abort/emergency state or state 6 terminal equivalent. |
| Falling vs recovery arm | `LocomotorState` has `jumpjet_crash_speed` and `air_phase`, but no separate `is_falling_or_crashing` and `fall_recovery_landing_armed` flags. | Cannot represent EMP-style fall (`+0x425` only) differently from piggyback recovery (`+0x425` plus `+0x427`). |
| Piggyback forced Jumpjet | `OverrideKind` currently has `Teleport`, `DropPod`, and `Parachute`; no Jumpjet/Magnetron forced-lift override kind. | Cannot express Magnetron's temporary Jumpjet locomotor piggyback with a saved base locomotor and `+0x6AD`-like active flag. |
| Bridge deck crossing | Air movement updates altitude but does not run a bridge-plane crossing `Can_Enter_Cell` probe during descent. | Cannot reproduce state 5 bridge-deck early recovery/collision behavior. |
| Candidate-only Can_Enter_Cell | Bridge movement code focuses on ground pathing/layer transitions; prior reports note no binary-shaped Jumpjet landing `Can_Enter_Cell(target, -1, -1, 0, 1)` equivalent. | Needs a sim-level query that can apply candidate bridge deck occupancy/list semantics for airborne landing checks. |
| Layering | `GameEntity` stores `on_bridge` and `bridge_occupancy` in sim state; no dependency on render/ui/sidebar/audio/net is needed for this feature. | Existing architecture can host the invariant, but current structures do not expose the required flags/state transitions. |

## 10. Binary-Verified vs Inferred Findings

### Binary-verified

- `TechnoClass__Constructor @ 0x006F2FF0` initializes object `+0x425`, `+0x426`, and `+0x427` to 0.
- `FootClass__Constructor @ 0x004D31E0` initializes object `+0x6AD` to 0.
- `FootClass__ReceiveEMP @ 0x004DEB50` writes object `+0x425 = 1` when altitude is positive.
- `BuildingClass__DeployUnit_ChronoWarp @ 0x0070FEE0` writes object `+0x425 = 1`, `+0x427 = 1`, and source pointers `+0x428/+0x42C` when a linked object is airborne.
- `TechnoClass__PerformDeploy @ 0x00710000` writes object `+0x6AD = 1` in the locomotor-piggyback setup path.
- `JumpjetLocomotionClass::Process @ 0x0054AEC0` reads `+0x425` and enters state 5 when airborne and not already state 5/6.
- `JumpjetLocomotionClass_State5_AbortEmergencyLanding @ 0x0054CA90` reads `+0x427` and requires it before the recovery `Can_Enter_Cell` call.
- State 5 bridge crossing uses `DAT_00ABC5DC` as an altitude threshold added to current cell ground height.
- State 4 clears `+0x427` and `+0x425` after successful landing/finalization.
- Same displacement `+0x6AD` in `TechnoTypeClass` constructor/ReadINI is not the linked object runtime byte.

### Inferred but strongly supported

- `+0x425` is best named a falling/crashing request or active falling flag.
- `+0x427` is best named a fall recovery landing armed flag.
- `+0x6AD` is best named a locomotor-piggyback/deploy-swap active flag.
- Magnetron forced-lift uses the same state 5 path after piggyback release because the warhead's `Locomotor` is the Jumpjet CLSID and the shared infrastructure writes the same state flags.

### Open or lower-confidence

- The exact direct clear path for runtime object `+0x6AD` was not found as a direct byte-displacement write in this pass.
- The exact tick on which Magnetron's piggyback release calls the cleanup writer remains dependent on the `IPiggyback::Is_Ok_To_End` chain, which prior Magnetron research did not fully close.
- Native Jumpjet death/crash from ordinary damage was not exhaustively traced beyond the direct `+0x425` writer scan. If future work needs normal death behavior, audit damage/death writers separately.

## Sources

- Ghidra decompiled:
  - `JumpjetLocomotionClass::Process` / `FUN_0054AEC0 @ 0x0054AEC0`
  - `JumpjetLocomotionClass_State4_DescendLand` / `FUN_0054C550 @ 0x0054C550`
  - `JumpjetLocomotionClass_State5_AbortEmergencyLanding` / `FUN_0054CA90 @ 0x0054CA90`
  - `FootClass__ReceiveEMP @ 0x004DEB50`
  - `FlyLocomotionClass__Move_To @ 0x004CF5F0`
  - `FootClass__AI @ 0x004DA530`
  - `TechnoClass__Constructor @ 0x006F2FF0`
  - `FootClass__Constructor @ 0x004D31E0`
  - `BuildingClass__DeployUnit_ChronoWarp @ 0x0070FEE0`
  - `TechnoClass__PerformDeploy @ 0x00710000`
  - `TechnoClass__CanAutoCloak @ 0x006FBF6D`
  - `TechnoClass__GetFireError @ 0x006FC1AA`
  - `TechnoTypeClass__Constructor @ 0x00710AF0` and `TechnoTypeClass__ReadINI @ 0x00712180` for same-offset false-positive separation
- Byte-pattern scans:
  - Direct displacement `25 04 00 00`, `27 04 00 00`, `AD 06 00 00`
  - Direct byte writes `C6 81/82/86 ...`, constructor byte writes `88 9E ...`
  - Memory bytes around `0x004DEC7F`, `0x0054CA08`, `0x0054CA12`, `0x0070FF25`, `0x0070FF32`
- Prior reports read:
  - `BRIDGE_JUMPJET_HEIGHT_MINUS_ONE_RUNTIME_GHIDRA_REPORT.md`
  - `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`
  - `BRIDGE_RUNTIME_CAN_ENTER_CELL_CALLSITE_MATRIX_GHIDRA_REPORT.md`
  - `BRIDGE_CHECK_TRAVERSAL_PARENT_FALLBACK_GHIDRA_REPORT.md`
  - `BRIDGE_LOCOMOTOR_AIR_HOVER_GHIDRA_REPORT.md`
  - `MAGNETRON_SYSTEM_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini`: `[LocomotorBeam] IsLocomotor=yes`, Jumpjet CLSID; Magnetron `[TELE]`; Jumpjet units including `[JUMPJET]`, `[DISK]`, `[SCHP]`, `[ZEP]`
  - `ini/rules.ini`: base Jumpjet/EMP fallback data
- Rust read-only scan:
  - `src/sim/movement/locomotor.rs`
  - `src/sim/movement/air_movement.rs`
  - `src/sim/movement/jumpjet_movement.rs`
  - `src/sim/game_entity.rs`
