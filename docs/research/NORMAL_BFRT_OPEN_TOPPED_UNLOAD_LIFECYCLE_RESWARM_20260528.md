# Normal BFRT OpenTopped Unload Lifecycle - Re-Swarm Research Report

**Address(es):** `0x0073D630` (`UnitClass::Mission_Deploy_Building`), `0x007104A0` (`TechnoClass::ClearInOpenTransport`), `0x004DE710` / `0x00473430` cargo pop helpers  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** normal/manual generic transport unload for BFRT-like `OpenTopped=yes` unit transports, especially `Techno+0x82` clear timing, reveal/eject ordering, and active-list consequences.  
**Non-Scope:** fatal transport destruction except as contrast, IFV gunner weapon-slot math, full passability helper internals, UI/sidebar command routing into the unload mission.  
**Status:** COMPLETE for the requested normal BFRT/OpenTopped unload slice.  
**Confidence:** High for the caller/callee map and ordering in the normal unload body; Medium for human-readable names of some virtual slots.  
**Active in YR:** Yes. Stock `ini/rulesmd.ini` has `[BFRT] Passengers=5`, `OpenTopped=yes`, and `SizeLimit=2`, and the verified unload body is keyed by the transport's parsed `Passengers` field.

## 1. Executive Result

Normal BFRT/OpenTopped unload clears passenger `Techno+0x82` only after the passenger has been popped from cargo, a destination has been found, and the passenger's reveal/unlimbo placement virtual `+0xD8` succeeds. It does not clear `+0x82` merely because unload was requested, merely because the passenger was popped, or on placement failure.

The successful normal-unload ordering is:

1. `UnitClass::Mission_Deploy_Building` state 3 checks cargo count/skip index.
2. It pops one cargo head via `FUN_004DE710 -> FUN_00473430`.
3. It derives an eight-direction scan start from `RateTimer::Current`.
4. It validates candidate cells and computes final placement coordinates.
5. It calls passenger virtual `+0xD8` to reveal/unlimbo/place the passenger.
6. Only if `+0xD8` succeeds and the transport type has `OpenTopped != 0`, it calls `TechnoClass::ClearInOpenTransport(passenger)`, which only writes `passenger+0x82 = 0`.
7. For OpenTopped mixed-owner cargo, it optionally calls passenger virtual `+0x3C8(0)`.
8. It clears passenger transport pointer `+0x11C = 0`.
9. It queues passenger mission `2`.
10. It calls passenger virtual `+0x480(cell, 1)` for the selected destination.
11. It may play the transport leave sound at the transport coordinate.

If placement fails, the passenger is re-added to cargo through `CargoClass::AddPassenger` and `+0x82` is not cleared in the inspected failure branch. This differs from fatal BFRT death, where `CargoClass::ClearAllInOpenTransport` clears `+0x82` for the cargo chain before the fatal cargo ejection/death loop.

## 2. Function Map

| Function | Role in this slice | Direct evidence | Active in YR |
|---|---|---|---|
| `UnitClass::Mission_Deploy_Building @ 0x0073D630` | Normal generic vehicle passenger unload body, state 3 | Decompile shows `Passengers > 0` state machine; callees include `FUN_004DE710`, `TechnoClass::ClearInOpenTransport`, `CargoClass::AddPassenger`, `RateTimer::Current`, `RandomRanged` | Yes for unit transports with `Passengers > 0` |
| `FUN_004DE710 @ 0x004DE710` | Unit cargo pop wrapper | Calls `FUN_00473430`, then IFV/Gunner cleanup only when type `+0x805 != 0` and cargo count is zero | Yes |
| `FUN_00473430 @ 0x00473430` | Cargo head pop primitive | Reads `[CargoClass+4]`, advances to old head `+0x30`, clears old head `+0x30`, decrements count | Yes |
| `TechnoClass::ClearInOpenTransport @ 0x007104A0` | Single passenger OpenTopped flag clear | Direct caller map returns only `UnitClass::Mission_Deploy_Building`; body only writes `+0x82 = 0` | Yes for successful BFRT/OpenTopped normal unload |
| `CargoClass::ClearAllInOpenTransport @ 0x007104C0` | Fatal-damage contrast only | Direct caller map returns only `UnitClass::ReceiveDamage`; prior slot proves fatal-result-4 gate | Yes for fatal BFRT destruction, not normal unload |

Direct caller evidence:

- `get_function_callers(0x007104A0)` -> `UnitClass__Mission_Deploy_Building @ 0x0073D630`.
- `get_function_callers(0x007104C0)` -> `UnitClass__ReceiveDamage @ 0x00737C90`.
- `get_function_callees(0x0073D630)` includes `TechnoClass__ClearInOpenTransport @ 0x007104A0` and does not include `CargoClass__ClearAllInOpenTransport`.

## 3. Core Normal-Unload Logic

### 3.1 Cargo Pop

`UnitClass::Mission_Deploy_Building` state 3 checks whether the cargo count at `Unit+0x114` is greater than the skip/lower-bound value at `Unit+0x6E4`. If so, it calls `FUN_004DE710`.

Assembly support:

- `0x0073D8B7`: reads `dword ptr [ESI + 0x114]`.
- `0x0073D8BD`: reads `dword ptr [ESI + 0x6E4]`.
- `0x0073D8C3..0x0073D8C5`: compare and skip if count is not greater.
- `0x0073D8CB..0x0073D8CD`: `ECX=ESI`, call `0x004DE710`.
- `0x0073D8D2..0x0073D8D6`: moved return into `EDI`, null-guarded before placement work.

`FUN_004DE710` first pops the cargo head:

- `0x004DE715`: `LEA EDI,[ESI + 0x114]`.
- `0x004DE71B..0x004DE71D`: `ECX=EDI`, call `0x00473430`.
- `0x004DE722`: stores popped passenger in `EBX`.
- `0x004DE724..0x004DE742`: only if transport type `+0x805` (`Gunner`) is nonzero and cargo count is now zero, calls transport virtual `+0x4D8(poppedPassenger)`.

`FUN_00473430` is a linked-list head pop:

- `0x00473430`: `EAX = [ECX + 4]` cargo head.
- `0x00473438..0x0047343B`: read old head next pointer `[EAX + 0x30]` and store it to `[ECX + 4]`.
- `0x0047343E`: clear old head `+0x30`.
- `0x00473445`: decrement cargo count `[ECX]`.

Tiny detail: the pop helper itself does not clear `Techno+0x82`, does not clear passenger transport pointer `+0x11C`, and does not call an active-vector helper. Those happen later or not at all.

### 3.2 Direction Selection And Placement Attempt

The normal unload path uses a timer-derived scan start, not scenario RNG, for destination search:

- `0x0073D8E7`: calls `RateTimer::Current @ 0x004C93D0`.
- `0x0073D8EC`: reads the returned word.
- `0x0073D8F4`: adds `0x7FFF`.
- `0x0073D901..0x0073D907`: shifts by 12, increments, then shifts by 1 to form the base direction.
- `0x0073D919..0x0073D922`: computes `(base + local_index) & 7`.
- `0x0073D917..0x0073DA02`: loops up to eight candidate directions.

Candidate handling:

- `0x0073D925`: reads the transport cell through vtable `+0x1B8`.
- The code indexes `g_DirectionOffsets @ 0x0089F688` to derive adjacent seed cells.
- `0x0073D9C2`: calls passenger virtual `+0x1AC` for candidate validation.
- `0x0073DA66..0x0073DA7C`: if passenger RTTI is `0x0F`, calls `FUN_004ACA10` for infantry subcell placement.
- Non-infantry falls through to `FootClass::Find_Nearby_Passable_Cell @ 0x0056DC20`.
- `0x0073DB6A`: calls passenger virtual `+0xD8` with the final coordinate block; if AL is zero, branch goes to failure rollback.

Tiny detail: `g_MapEditorMode` is incremented and decremented around the placement attempt in the decompile. This slice did not assign a player-facing name to that global, but the increment/decrement brackets the placement calculation and must not be treated as random/no-op state if this path is implemented at exact mechanism level.

### 3.3 Successful OpenTopped Unload Ordering

The OpenTopped clear is strictly after successful `+0xD8` placement:

- `0x0073DB6A`: call passenger virtual `+0xD8`.
- `0x0073DB77..0x0073DB7F`: test AL and jump to failure rollback on zero.
- `0x0073DB85`: read transport type pointer `[ESI + 0x6C4]`.
- `0x0073DB8B`: read byte `[Type + 0x5E4]` (`OpenTopped`).
- `0x0073DB91..0x0073DB93`: skip clear if zero.
- `0x0073DB95..0x0073DB98`: push passenger pointer `EDI`, set `ECX=ESI`, call `TechnoClass::ClearInOpenTransport @ 0x007104A0`.

`TechnoClass::ClearInOpenTransport` body:

```asm
007104a0: MOV EAX,dword ptr [ESP + 0x4]
007104a4: TEST EAX,EAX
007104a6: JZ 0x007104af
007104a8: MOV byte ptr [EAX + 0x82],0x0
007104af: RET 0x4
```

There are no calls inside this helper. It does not unregister the object, does not unhide the body, does not clear transport pointer `+0x11C`, and does not set a mission.

Post-clear success continuation:

- `0x0073DB9D..0x0073DBBB`: if transport type `OpenTopped` is nonzero and transport owner `+0x21C` differs from passenger owner `+0x21C`, enter extra cleanup.
- `0x0073DBBD..0x0073DBC3`: call passenger virtual `+0x3C8(0)` for the mixed-owner OpenTopped case.
- `0x0073DBCB..0x0073DBD1`: push `0`, push `2`, set `ECX=EDI`, and write `passenger+0x11C = 0`.
- `0x0073DBDB`: call passenger virtual `+0x1E8`; decompile resolves this as mission assignment with mission id `2`.
- `0x0073DBE9..0x0073DC06`: call passenger virtual `+0x480(cell, 1)` to hand off the selected destination cell.
- `0x0073DC1E..0x0073DC67`: if transport type `+0x568 != -1`, play leave sound at the transport coordinate.

Tiny detail: transport pointer `+0x11C` is cleared after the optional OpenTopped owner-difference target cleanup, not before `ClearInOpenTransport`. The passenger is therefore still linked to the transport when `+0x82` is cleared.

### 3.4 Failure Rollback

If no placement validates or `+0xD8` fails, the code re-adds the popped passenger to cargo:

- `0x0073DB7F`: failed `+0xD8` jumps to `0x0073DC71`.
- `0x0073DC71..0x0073DC78`: pushes passenger pointer, `ECX = transport + 0x114`, calls `CargoClass::AddPassenger @ 0x004733A0`.
- `0x0073DC87..0x0073DC96`: if transport type `Gunner` byte `+0x805` is nonzero, calls transport virtual `+0x4D4(passenger)`.
- `0x0073DC9C..0x0073DCA6`: calls passenger virtual `+0x11C` with no argument in the decompile view.

`CargoClass::AddPassenger` itself starts by calling passenger virtual `+0xD4`, then relinks through cargo `+4` / passenger `+0x30` and recomputes count.

Important negative fact: no `TechnoClass::ClearInOpenTransport` call occurs in this failure rollback branch before `CargoClass::AddPassenger`. For a BFRT passenger that was already marked `+0x82=1` while inside, failed normal unload keeps the OpenTopped byte set and puts the passenger back in cargo.

## 4. Active-List Membership Consequences

Normal OpenTopped unload does not call `FUN_0055BAA0` and does not call the active-vector removal helper in the verified `UnitClass::Mission_Deploy_Building` body. The passenger was made live-active earlier by `TechnoClass::SetInOpenTransport @ 0x00710470`, which sets `+0x82=1`, calls vtable `+0x3D0`, then calls `FUN_0055BAA0(this, 0)`.

Consequences:

- While inside BFRT, the passenger remains in the LogicClass active vector.
- During normal unload, cargo pop does not change active membership.
- Successful placement clears only the OpenTopped firing/contained byte `+0x82`, then clears the transport pointer and assigns move mission/destination.
- No new active registration is needed on successful unload, because the passenger was already active.
- Failed placement returns the passenger to cargo without clearing `+0x82`, preserving the contained OpenTopped state.

This is mechanism-distinct from true `ObjectClass::Conceal` / `ObjectClass::Reveal` as a full lifecycle pair. The normal unload path uses the passenger's placement virtual `+0xD8`, but the OpenTopped clear helper itself is only a byte clear.

## 5. Fatal Transport Death Contrast

Fatal BFRT death uses a different helper and timing:

| Aspect | Normal/manual BFRT unload | Fatal BFRT transport death |
|---|---|---|
| Clear helper | `TechnoClass::ClearInOpenTransport @ 0x007104A0` | `CargoClass::ClearAllInOpenTransport @ 0x007104C0` |
| Direct caller | `UnitClass::Mission_Deploy_Building` only | `UnitClass::ReceiveDamage` only |
| Clear timing | After one passenger placement `+0xD8` succeeds | Before fatal cargo ejection/death loop |
| Scope | One passenger | Cargo chain |
| Failure behavior | Placement failure re-adds passenger; `+0x82` not cleared in inspected branch | Prior death report: all visited cargo-chain passenger `+0x82` bytes are cleared before ejection/death attempts |
| Active-vector effect | No register/unregister call | Clear helper itself has no register/unregister; later failed ejection reaches object cleanup |

Do not generalize fatal death timing onto normal unload. In normal unload the passenger must successfully get out before `+0x82` is cleared.

## 6. Standard YR Activation

Stock `ini/rulesmd.ini`:

- `[BFRT]` at line `6917`.
- `Passengers=5` at line `6931`.
- `OpenTopped=yes` at line `6932`.
- `SizeLimit=2` at line `6933`.

Representative stock passengers have `OpenTransportWeapon=` assignments, including GI/GGI/SEAL/Tanya/Boris/Yuri Prime entries. This proves the BFRT OpenTopped passenger flow is stock-live YR behavior, not a mod-only path.

## 7. Current Rust Implementation Status

Rust surfaces scanned:

- `src/rules/object_type.rs:570` stores `open_topped`.
- `src/rules/object_type.rs:1084` parses `OpenTopped=`.
- `src/sim/passenger.rs:480` and `:732` set `PassengerRole::Inside`.
- `src/sim/passenger.rs:490` and `:745` assign `WeaponOverride::OpenTransport` to the transport when boarding an OpenTopped transport.
- `src/sim/passenger.rs:975..1005` normal unload pops one passenger and immediately sets `PassengerRole::None` / position.
- `src/sim/passenger.rs:1028..1058` issues a random scatter-like move after unload.
- `src/sim/combat/mod.rs:1370` skips inside-transport attackers.
- `src/sim/combat/mod.rs:925..930` blanket-kills non-garrison transport riders on transport destruction.

Rust has no explicit per-passenger `Techno+0x82` equivalent. `PassengerRole::Inside` currently conflates hidden/contained state with OpenTopped passenger firing eligibility and with normal cargo storage. That cannot express native normal-unload timing: inside cargo with `+0x82=1`, popped but not yet placed with `+0x82=1`, then successfully placed with `+0x82=0`.

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Successful normal BFRT unload clears `Techno+0x82` after passenger placement `+0xD8` succeeds, not at unload start or cargo pop. | `0x0073DB6A`, `0x0073DB77..0x0073DB98`; `0x007104A0` body | No per-passenger OpenTopped byte; `PassengerRole::None` is set immediately after pop/placement in current simplified flow | `src/sim/passenger.rs`, future lifecycle/active-object API | Represent OpenTopped-contained/fire eligibility as a per-passenger state separable from `PassengerRole`; clear it only after successful normal unload placement. | `normal_bfrt_unload_clears_open_topped_flag_after_successful_placement` | Do not clear the flag merely because unload order is active or the passenger was popped. |
| Failed normal unload re-adds the passenger to cargo without calling `ClearInOpenTransport`; `+0x82` remains set. | Failure branch `0x0073DB7F -> 0x0073DC71`; no call to `0x007104A0`; `CargoClass::AddPassenger @ 0x004733A0` | Rust currently skips tick if no exit cell before pop; other placement failures are not modeled with native pop/re-add semantics | `src/sim/passenger.rs::tick_unloading`, passability/reveal placement helpers | If a passenger is popped and placement fails, restore cargo and keep OpenTopped-contained/firing state. | `normal_bfrt_unload_failed_placement_keeps_passenger_inside_and_open_topped_active` | Do not turn failed placement into unloaded `PassengerRole::None` or clear OpenTopped fire eligibility. |
| Normal unload does not register or unregister the passenger in the active vector. | `UnitClass::Mission_Deploy_Building` callee list lacks `FUN_0055BAA0` and active-remove helper; prior `SetInOpenTransport` registers at boarding | Rust live active order currently does not model OpenTopped passengers as active attackers while inside | `src/sim/world/mod.rs`, `src/sim/combat/mod.rs`, `src/sim/passenger.rs` | Keep BFRT passengers live-active while inside and across successful unload; unload should not append a duplicate active-order entry. | `normal_bfrt_unload_preserves_existing_live_order_membership` | Do not unregister on board or re-register on unload as a substitute for the native byte transition. |
| Successful normal unload clears transport pointer `+0x11C` after optional OpenTopped owner-difference cleanup, then queues mission `2` and destination `+0x480(cell,1)`. | `0x0073DB9D..0x0073DBDB`, `0x0073DBE9..0x0073DC06` | Rust sets `PassengerRole::None`, then issues direct/random scatter-like movement | `src/sim/passenger.rs`, movement/order intent surfaces | After placement, clear inside role before issuing a native-equivalent move mission/destination; do not use garrison scatter semantics. | `normal_transport_unload_queues_move_mission_not_scatter` | Do not reuse CanBeOccupied sell/destruction scatter for generic vehicle transport unload. |
| Normal BFRT unload clears only one passenger per state-3 pass. Fatal BFRT death clears the cargo chain first. | `FUN_00473430` head pop; `0x007104A0` single-object body; prior fatal `0x007104C0` cargo-chain body | Rust has FIFO `Vec` unload and no distinct fatal clear-all flag pass | `src/sim/passenger.rs`, `src/sim/combat/mod.rs` | Keep normal unload one-passenger-at-a-time; use separate fatal transport death path for chain-wide OpenTopped flag clear. | `normal_bfrt_unload_clears_one_passenger_not_all_cargo_flags` | Do not call a cargo-wide clear helper for normal manual unload. |
| BFRT activates the normal OpenTopped branch in stock YR. | `rulesmd.ini:6931..6933`; binary reads transport type `+0x5E4` before calling `0x007104A0` | Current drift affects stock Allied late-game unit | `src/rules/object_type.rs`, `src/sim/passenger.rs`, `src/sim/combat/mod.rs` | Treat as stock gameplay parity requirement, not a mod-only edge case. | `stock_bfrt_manual_unload_uses_open_topped_clear_timing` | Do not defer this as rare or custom-content-only. |

## 9. Negative Facts / Do Not Do

- Do not use `CargoClass::ClearAllInOpenTransport` for normal/manual unload. Its direct caller is fatal `UnitClass::ReceiveDamage`, not `Mission_Deploy_Building`.
- Do not clear `+0x82` on unload command start, cargo pop, or failed placement. The normal clear is after successful passenger `+0xD8` placement.
- Do not model `ClearInOpenTransport` as unhide/reveal/despawn. Its body only writes byte `+0x82 = 0`.
- Do not unregister BFRT passengers from active order on boarding or normal unload. Normal unload has no active-vector registration/removal call.
- Do not implement normal transport unload as garrison scatter. The verified success path queues mission `2` and destination `+0x480`, not Scatter mission `0x0F`.
- Do not collapse BFRT passengers into a transport-owned weapon override. Native normal unload proves the state being cleared is on the passenger.

## 10. Stale Docs / Follow-Up Docs

- `docs/research/IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` should replace any broad "cleared when it leaves" wording with: "Successful generic passenger unload from `UnitClass::Mission_Deploy_Building @ 0x0073D630` calls `TechnoClass::ClearInOpenTransport @ 0x007104A0` only after passenger placement virtual `+0xD8` succeeds, gated by transport type `OpenTopped` byte `+0x5E4`; failed placement re-adds cargo without this clear."
- The same doc should keep fatal `CargoClass::ClearAllInOpenTransport @ 0x007104C0` separate from normal unload: fatal death clears the cargo chain before ejection/death attempts, while normal unload clears one passenger after successful placement.
- Rust comments that describe generic unload as scatter should be updated in a later implementation/doc patch to distinguish generic vehicle transport move mission `2` from garrison/sell/destruction scatter behavior.

## 11. Coverage Ledger

| Area / branch | Status | Evidence | Remaining uncertainty |
|---|---|---|---|
| Direct caller of `ClearInOpenTransport` | verified | `get_function_callers(0x007104A0)` | none |
| Direct caller of `ClearAllInOpenTransport` for contrast | verified | `get_function_callers(0x007104C0)` | none for contrast |
| Normal unload cargo pop | verified | decompile and disassembly of `0x0073D8B7..0x0073D8D6`, `0x004DE710`, `0x00473430` | full boarding insertion FIFO/LIFO semantics remain separate |
| Direction scan and placement attempt | verified for ordering | decompile and assembly contexts `0x0073D8E7`, `0x0073D917`, `0x0073D9C2`, `0x0073DB6A` | exact internals of passenger `+0x1AC`, `+0xD8`, infantry subcell helper are separate systems |
| `+0x82` clear timing | verified | assembly context `0x0073DB85..0x0073DB98`; `0x007104A0` body | none |
| Success post-clear ordering | verified | assembly contexts `0x0073DB9D..0x0073DC06` | exact semantic names for `+0x3C8`, `+0x1E8`, `+0x480` can be refined in vtable docs |
| Failure rollback | verified for no-clear/re-add shape | assembly context `0x0073DC71..0x0073DCA6`; `CargoClass::AddPassenger` body | exact meaning of passenger virtual `+0x11C` after re-add remains deferred |
| Active-list membership | verified by absence/presence evidence | `Mission_Deploy_Building` callee map lacks register/remove helpers; prior `SetInOpenTransport` registers | runtime active-order watchpoint not performed |
| Stock BFRT activation | verified | `ini/rulesmd.ini:6931..6933`; `+0x5E4` branch | none |

## 12. Open Questions - Final State

- `[RESOLVED] OQ-01 - What normal unload path clears OpenTopped passenger `+0x82`? -> `UnitClass::Mission_Deploy_Building @ 0x0073D630` calls `TechnoClass::ClearInOpenTransport @ 0x007104A0` in the successful state-3 passenger unload path.`
- `[RESOLVED] OQ-02 - Is `+0x82` cleared before or after reveal/eject placement? -> After passenger virtual `+0xD8` succeeds.`
- `[RESOLVED] OQ-03 - Is `+0x82` cleared on placement failure? -> No clear call occurs in the failure rollback branch; the popped passenger is re-added to cargo.`
- `[RESOLVED] OQ-04 - Does normal unload clear all BFRT passenger flags? -> No; it pops and processes one passenger. Fatal destruction uses the separate cargo-chain clear helper.`
- `[RESOLVED] OQ-05 - Does `ClearInOpenTransport` unhide/unlimbo/unregister? -> No; the helper only null-checks and writes `byte[p+0x82]=0`.`
- `[RESOLVED] OQ-06 - Does normal unload add/remove active-list membership? -> No direct active registration/removal helper is called in the verified body; passenger active membership comes from boarding.`
- `[RESOLVED] OQ-07 - Is BFRT standard YR stock-active for this branch? -> Yes; `[BFRT]` has `Passengers=5` and `OpenTopped=yes`.`
- `[DEFERRED] OQ-08 - Exact semantic label for passenger virtual `+0x11C` called after failure re-add.` Category: vtable taxonomy. Reason: no need to prove `+0x82` timing; requires broader Techno/Foot virtual-slot audit.
- `[DEFERRED] OQ-09 - UI/sidebar command routing into `Mission_Deploy_Building` unload state.` Category: command pipeline. Reason: the target was lifecycle after normal/manual unload is active; prior generic unload report also deferred the command setter.
- `[DEFERRED] OQ-10 - Full `+0xD8` reveal/unlimbo internals for every passenger class.` Category: different slice. Reason: ordering around the call is verified; derived reveal details belong in Object/Foot reveal investigations.

## 13. Sources

- Live Ghidra decompile/read-only:
  - `UnitClass::Mission_Deploy_Building @ 0x0073D630`
  - `TechnoClass::ClearInOpenTransport @ 0x007104A0`
  - `CargoClass::ClearAllInOpenTransport @ 0x007104C0`
  - `FUN_004DE710 @ 0x004DE710`
  - `FUN_00473430 @ 0x00473430`
  - `CargoClass::AddPassenger @ 0x004733A0`
- Live Ghidra caller/callee evidence:
  - `get_function_callers(0x007104A0)`
  - `get_function_callers(0x007104C0)`
  - `get_function_callees(0x0073D630)`
- Live Ghidra assembly contexts:
  - `0x0073D8B7..0x0073D8D6`
  - `0x0073D8E7..0x0073D917`
  - `0x0073D9C2`
  - `0x0073DB6A..0x0073DC06`
  - `0x0073DC71..0x0073DCA6`
  - `0x007104A0..0x007104AF`
  - `0x004DE710..0x004DE74D`
  - `0x00473430..0x00473447`
  - `0x004733A0..0x0047342C`
- Research docs referenced:
  - `docs/research/GENERIC_TRANSPORT_MANUAL_UNLOAD_MAPPING_GHIDRA_REPORT.md`
  - `docs/research/SET_IN_OPEN_TRANSPORT_VTABLE_3D0_RESWARM_20260528.md`
  - `docs/research/CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md`
  - `docs/research/IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`
- INI checked:
  - `ini/rulesmd.ini`
- Rust scanned:
  - `src/sim/passenger.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/combat/combat_weapon.rs`
  - `src/rules/object_type.rs`
