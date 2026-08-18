# Cargo ClearAllInOpenTransport Damage Timing - Reswarm Research Report

**Address(es):** `0x007104C0` (`CargoClass::ClearAllInOpenTransport`), `0x007104A0` (`TechnoClass::ClearInOpenTransport`), caller slice in `UnitClass::ReceiveDamage @ 0x00737C90`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** OpenTopped cargo flag clearing during fatal unit damage/destruction, especially standard YR BFRT-like paths.  
**Non-Scope:** full normal unload bug audit, full passenger fire-position FLH selection, full aircraft/carryall cargo death behavior, full crashable jumpjet state machine.  
**Confidence:** High for the claimed damage/death slice.  
**Active in YR:** Yes. `[BFRT]` in `ini/rulesmd.ini` has `Passengers=5` and `OpenTopped=yes`; the fatal `UnitClass::ReceiveDamage` path gates on the transport type's `OpenTopped` byte.

## 1. Overview

`CargoClass::ClearAllInOpenTransport` is not a general "transport took damage" hook. In the verified caller slice it runs only after `FootClass::ReceiveDamage` returns death result `4` for a `UnitClass`, and only if the dying unit's type has `OpenTopped=yes`.

The helper clears passenger byte `Techno+0x82` for every cargo-chain node it visits. It does not eject, unhide, unlimbo, unregister, or destroy passengers by itself. Actual passenger survival/death is handled later in the same `UnitClass::ReceiveDamage` fatal path by a cargo-pop/eject loop.

## 2. Class Layout / Key Offsets

| Offset | Owner | Meaning in this slice | Evidence |
|---|---|---|---|
| `+0x82` | `TechnoClass` passenger | OpenTopped passenger flag; set at boarding and cleared here. | `0x0071047D`, `0x007104A8`, `0x007104D7` |
| `+0x114` | `UnitClass`/cargo subobject start | `CargoClass` subobject base used by `ClearAllInOpenTransport`; helper adds this before reading cargo head. | `0x007104C0` |
| `+0x118` | cargo owner | Cargo head pointer (`CargoClass+4`). | `FUN_00473450 @ 0x00473450`; fatal loop at `0x00737FC4` |
| `+0x30` | passenger object | Next cargo-chain pointer. | `0x007104DD`; `FUN_00473430 @ 0x00473430` |
| `+0x14 bit 0x04` | `AbstractClass`/object flags | Cargo/list-membership continuation guard for chained passengers after the first. | `0x007104E4` |
| `+0x5E4` | `TechnoTypeClass` | `OpenTopped=` byte, read on the dying transport type before `ClearAllInOpenTransport`. | `0x00737F80..0x00737F92`; `ini/rulesmd.ini:6932` |
| `+0xD95` | `TechnoTypeClass` | `Crashable=` byte; if set, the ordinary cargo-eject loop is skipped for crashable unit death. BFRT does not set it. | `0x00737FB0..0x00737FBE`, `0x00738457..0x00738465`; `units/AUDIT_INDEX.md` |
| `+0x21C` | `TechnoClass` | Owner pointer; passenger owner is compared with transport owner after successful ejection. | `0x00738114..0x00738122` |
| `+0x11C` | passenger | Transport pointer/link cleared after successful reveal/ejection. | `0x007380FA` |
| `+0x90` | `ObjectClass` | Alive byte; pending delete/destruction is still later than the cargo loop in the ordinary non-crashable path. | prior `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`; end call `0x007384A5` |

## 3. Core Logic

### 3.1 `TechnoClass::ClearInOpenTransport @ 0x007104A0`

Verified behavior:

```text
if passenger != null:
    passenger[0x82] = 0
return
```

Assembly support:

- `0x007104A0`: load argument pointer.
- `0x007104A4..0x007104A6`: null guard.
- `0x007104A8`: `MOV byte ptr [EAX + 0x82], 0`.
- `0x007104AF`: `RET 0x4`.

Caller map for this function is outside the damage/death slice: `get_function_callers(0x007104A0)` returns only `UnitClass::Mission_Deploy_Building @ 0x0073D630`. It is not called by `UnitClass::ReceiveDamage`.

### 3.2 `CargoClass::ClearAllInOpenTransport @ 0x007104C0`

Verified behavior:

```text
cargo = this + 0x114
p = cargo.first_passenger()
if p == null:
    return
do:
    p[0x82] = 0
    p = p[0x30]
while p != null and (p[0x14] & 0x04) != 0
```

Assembly support:

- `0x007104C0`: `ADD ECX,0x114`, selecting the cargo subobject.
- `0x007104C6`: calls `FUN_00473450`, which returns `[cargo + 4]`.
- `0x007104CB`: zeroes `ECX`; the loop writes byte zero via `CL`.
- `0x007104CD..0x007104CF`: empty-cargo early return.
- `0x007104D7`: `MOV byte ptr [EAX + 0x82], CL`.
- `0x007104DD`: advances to `[passenger + 0x30]`.
- `0x007104E4..0x007104E7`: continues only while `(next[0x14] & 4) != 0`.
- No call to `FUN_0055BAA0`, no active-vector unregister helper, and no object limbo/uninit call appears in the helper.

Caller map:

| Callee | Direct callers | Damage/death relevance |
|---|---|---|
| `CargoClass::ClearAllInOpenTransport @ 0x007104C0` | `UnitClass::ReceiveDamage @ 0x00737C90` only | The single verified damage/death caller. |
| `TechnoClass::ClearInOpenTransport @ 0x007104A0` | `UnitClass::Mission_Deploy_Building @ 0x0073D630` only | Not used for fatal transport damage. |

### 3.3 Fatal `UnitClass::ReceiveDamage` ordering

The relevant order after `FootClass::ReceiveDamage` returns result `4` is:

1. Run fatal unit side effects: attached tag handling, death explosion or crash-retention branch, and/or `Health=1` / alive restoration for special crashable paths.
2. Reach common fatal label at `0x00737F74`.
3. Call vtable `+0x124` with argument `0`.
4. If `this->Type->OpenTopped` is nonzero, call `CargoClass::ClearAllInOpenTransport`.
5. Call vtable `+0x1C8`; if result is greater than `0xD0`, call `FootClass::EMPPassengers`.
6. If `this->Type->Crashable == 0`, process the cargo-eject/death loop.
7. Later, after survivor/parachute/crash-related branches, call the dying transport's vtable `+0xF8` unless the crashable/deferred path suppresses it.

Assembly support around the OpenTopped clear:

- `0x00737F74..0x00737F7A`: call vtable `+0x124` before any OpenTopped clear.
- `0x00737F80..0x00737F8E`: read `[ESI + 0x6C4]`, then byte `[Type + 0x5E4]`, and skip if zero.
- `0x00737F90..0x00737F92`: `ECX=ESI`, then `CALL 0x007104C0`.
- `0x00737F97..0x00737FA6`: call vtable `+0x1C8`; compare result to `0xD0`.
- `0x00737FA8..0x00737FAB`: call `FootClass::EMPPassengers` only when that compare is greater-than.
- `0x00737FB0..0x00737FBE`: read `Type+0xD95`; if nonzero, jump over the cargo loop.
- `0x00738457..0x007384A5`: much later, the non-crashable path reaches vtable `+0xF8` on the dying transport.

### 3.4 Passenger cargo loop after the clear

For non-crashable dying units, `UnitClass::ReceiveDamage` pops cargo one passenger at a time:

1. Check cargo head at `this+0x118`.
2. Call `FUN_004DE710`, which calls `FUN_00473430` to pop the first cargo node and, if the dying transport is `Gunner=yes` and its gunner state is zero, calls vtable `+0x4D8` with the popped passenger.
3. Try to find/reveal the passenger near the transport's location through vtable `+0x1AC` and then vtable `+0xD8`.
4. If reveal/ejection succeeds:
   - write passenger `+0x11C = 0`;
   - if the dying transport is `OpenTopped` and passenger owner differs from transport owner, call passenger vtable `+0x3C8(0)`;
   - call passenger vtable `+0x174(DAT_00B1CFE8, 1, 0)`;
   - if not player-controlled, assign mission `0x0F` or run `FUN_006EA500` depending on transport state;
   - if the dying transport was selected by a human player at damage entry, call passenger vtable `+0x14C`.
5. If reveal/ejection fails:
   - call passenger vtable `+0xE0`;
   - call passenger vtable `+0xF8`, destroying/unqueuing the passenger through the normal object cleanup path.
6. Repeat while cargo head remains non-null.

Assembly support for the successful ejection subpath:

- `0x00737FC4..0x00737FDD`: cargo head guard, `FUN_004DE710`, null guard.
- `0x007380EC..0x007380F4`: passenger vtable `+0xD8` reveal/ejection attempt, branch to death subpath on failure.
- `0x007380FA`: passenger `+0x11C = 0`.
- `0x00738104..0x00738122`: if transport type is OpenTopped, compare transport owner at `+0x21C` with passenger owner at `+0x21C`.
- `0x00738124..0x0073812A`: if owners differ, call passenger vtable `+0x3C8(0)`.
- `0x00738130..0x0073813D`: call passenger vtable `+0x174(DAT_00B1CFE8, 1, 0)`.
- `0x0073815C..0x0073816E`: non-player-control branch runs `FUN_006EA500` or mission `0x0F`.

Assembly support for the failed ejection/destruction subpath:

- `0x00738188..0x00738191`: call passenger vtable `+0xE0`.
- `0x00738197..0x0073819B`: call passenger vtable `+0xF8`.
- `0x007381AE..0x007381B6`: reload cargo head and continue the loop if non-null.

### 3.5 Active-list membership implications

`ClearAllInOpenTransport` only clears `Techno+0x82`; it neither removes nor appends passengers to the `LogicClass` active vector. This matches the prior boarding result: `SetInOpenTransport @ 0x00710470` registers OpenTopped passengers into the live vector when they board.

For the fatal BFRT path:

- Before cargo ejection/death handling, all passengers have `+0x82` cleared.
- Successfully ejected passengers remain ordinary live objects; no new `FUN_0055BAA0` call is needed because they were already live-registered while inside the OpenTopped transport.
- Failed ejection reaches passenger `+0xE0` and `+0xF8`, so removal/deferred destruction follows the ordinary object cleanup path.
- The dying transport's own `+0xF8` happens after passenger processing in the ordinary non-crashable path.

## 4. INI Keys

| Key | Scope | Standard YR value relevant here | Effect in this slice |
|---|---|---|---|
| `OpenTopped=` | `TechnoType`/unit type | `[BFRT] OpenTopped=yes` in `ini/rulesmd.ini:6932` | Gates `ClearAllInOpenTransport` after fatal `UnitClass::ReceiveDamage`. |
| `Passengers=` | `TechnoType`/unit type | `[BFRT] Passengers=5` in `ini/rulesmd.ini:6931` | Enables cargo chain containing up to five passengers. |
| `SizeLimit=` | `TechnoType`/unit type | `[BFRT] SizeLimit=2` in `ini/rulesmd.ini:6933` | Boarding eligibility; not read in the death clear helper. |
| `Crashable=` | `TechnoType`/unit type | BFRT has no `Crashable=` assignment in its visible rules block; default false. | If true at `Type+0xD95`, skips the ordinary cargo-eject loop after the OpenTopped clear. |
| `Gunner=` | `TechnoType`/unit type | BFRT does not use `Gunner=yes`; IFV does. | Only affects `FUN_004DE710` gunner cleanup on cargo pop; not the OpenTopped clear. |

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `UnitClass::ReceiveDamage` death result | Calls `ClearAllInOpenTransport` only on fatal result `4`, after fatal unit side effects. | decompile `0x00737C90`; assembly `0x00737F74..0x00737F92` | Yes |
| Nonfatal damage | Does not call `ClearAllInOpenTransport`; nonfatal branch returns or runs retaliation/AI side effects. | decompile branch `param_8 != 4` | Yes |
| OpenTopped gate | Reads dying transport type byte `+0x5E4`; no call when false. | `0x00737F80..0x00737F8E` | Yes; BFRT sets it |
| Passenger cargo loop | Runs after the OpenTopped clear and EMP-passenger branch, and only when `Crashable == 0`. | `0x00737FB0..0x007381B6` | Yes for BFRT |
| Transport final uninit | For non-crashable ordinary path, dying transport `+0xF8` is after cargo processing. | `0x00738493..0x007384A5` | Yes for BFRT |

## 6. Current Rust Implementation Status

Rust already parses or models some adjacent pieces:

- `src/rules/object_type.rs:570` has `open_topped: bool`.
- `src/rules/object_type.rs:587` has `open_transport_weapon: i32`.
- `src/rules/object_type.rs:1084` parses `OpenTopped`.
- `src/rules/object_type.rs:1087` parses `OpenTransportWeapon`.
- `src/sim/passenger.rs:443..493` and `src/sim/passenger.rs:685..748` compute `transport_open_topped` and set an `OpenTransport` weapon override during boarding.
- `src/sim/combat/combat_weapon.rs:45..55` defines `WeaponOverride::OpenTransport`.

Main deltas in this damage/death slice:

- `src/sim/combat/mod.rs:925..930` currently kills riders for non-garrison transports. Native `UnitClass::ReceiveDamage` attempts to eject cargo alive first and only kills passengers whose reveal/ejection fails.
- `src/sim/combat/mod.rs:1370..1371` skips all entities inside a transport as attackers; prior work already established OpenTopped passengers should stay live-active while contained.
- Rust has no explicit per-passenger `Techno+0x82` equivalent. It uses `PassengerRole::Inside` plus `weapon_override`, so it cannot currently model the native order "clear open-transport firing flag before cargo ejection/death loop."
- Rust's `weapon_override` for OpenTopped is stored on the transport in boarding code, while native OpenTopped state is per passenger. This is a broader OpenTopped implementation drift, but it directly affects damage cleanup because clearing must happen per passenger before ejection.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `TechnoClass::ClearInOpenTransport @ 0x007104A0` | verified | decompile + assembly `0x007104A0..0x007104AF` | none for damage slice; normal deploy caller outside this report |
| `CargoClass::ClearAllInOpenTransport @ 0x007104C0` | verified | decompile + assembly `0x007104C0..0x007104E9` | none |
| Direct caller map for `ClearAllInOpenTransport` | verified | `get_function_callers(0x007104C0)` -> `UnitClass::ReceiveDamage` | none |
| Direct caller map for `ClearInOpenTransport` | verified | `get_function_callers(0x007104A0)` -> `UnitClass::Mission_Deploy_Building` | normal unload bug audit deferred |
| Fatal `UnitClass::ReceiveDamage` OpenTopped branch | verified | decompile `0x00737C90`; assembly `0x00737F74..0x00737FAB` | none |
| Nonfatal damage does not clear OpenTopped flag | verified | `param_8 != 4` branch in decompile | none |
| Passenger cargo eject/death loop | verified for ordering | decompile `0x00737FC4..0x007381B6`; assembly contexts listed above | exact names for several virtual slots remain inferred from prior lifecycle docs |
| Crashable skip of ordinary cargo loop | verified | `Type+0xD95` branch `0x00737FB0..0x00737FBE` | full jumpjet crash cargo behavior outside scope |
| BFRT standard YR activity | verified | `ini/rulesmd.ini:6931..6933` and no `Crashable=` in BFRT block | runtime test not performed |
| Rust passenger death behavior | verified-by-scan | `src/sim/combat/mod.rs:925..930` | implementation not performed |
| Rust OpenTopped active firing skip | verified-by-scan | `src/sim/combat/mod.rs:1370..1371` | implementation not performed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is `ClearAllInOpenTransport` called from more than one damage/death function? -> No; direct caller map returns only `UnitClass::ReceiveDamage`.` (evidence: `get_function_callers(0x007104C0)`)
- `[RESOLVED] OQ-02 - Is `ClearInOpenTransport` used in the fatal damage path? -> No; direct caller map returns only `UnitClass::Mission_Deploy_Building`.` (evidence: `get_function_callers(0x007104A0)`)
- `[RESOLVED] OQ-03 - Does nonfatal damage clear OpenTopped passenger flags? -> No; `ClearAllInOpenTransport` is below the `param_8 == 4` fatal branch only.` (evidence: `UnitClass::ReceiveDamage @ 0x00737C90`)
- `[RESOLVED] OQ-04 - What gates the fatal clear? -> Dying transport type byte `+0x5E4` (`OpenTopped`) must be nonzero.` (evidence: `0x00737F80..0x00737F92`, `ini/rulesmd.ini:6932`)
- `[RESOLVED] OQ-05 - Does the clear helper unregister passengers from active logic? -> No; it only writes `+0x82=0` while walking `+0x30` cargo links.` (evidence: `0x007104C0..0x007104E9`)
- `[RESOLVED] OQ-06 - Does the clear helper eject passengers? -> No; ejection is later in `UnitClass::ReceiveDamage`, starting at the cargo head test.` (evidence: `0x00737FC4..0x007381B6`)
- `[RESOLVED] OQ-07 - Are passengers always killed when a BFRT dies? -> No; native first attempts reveal/ejection and only destroys passengers when that attempt fails or branch conditions send them to the failure subpath.` (evidence: `0x007380EC`, `0x00738188..0x0073819B`)
- `[RESOLVED] OQ-08 - Is the clear before or after the dying transport's final `UnInit`? -> Before; transport vtable `+0xF8` is reached later at `0x007384A5` in the ordinary non-crashable path.` (evidence: `0x00737F92`, `0x00738493..0x007384A5`)
- `[RESOLVED] OQ-09 - Does BFRT activate this path in stock YR? -> Yes; BFRT has `Passengers=5` and `OpenTopped=yes`, and no visible `Crashable=` override in its standard rules block.` (evidence: `ini/rulesmd.ini:6931..6933`)
- `[RESOLVED] OQ-10 - What happens if cargo is empty? -> `ClearAllInOpenTransport` returns immediately after first passenger pointer is null.` (evidence: `0x007104C6..0x007104CF`)
- `[RESOLVED] OQ-11 - What stops the cargo-chain clear loop? -> Null next pointer or next object's flags missing bit `0x04`.` (evidence: `0x007104DD..0x007104E7`)
- `[RESOLVED] OQ-12 - Is same-owner passenger handling identical to different-owner handling after ejection? -> Mostly, but the OpenTopped different-owner subpath calls passenger vtable `+0x3C8(0)` before common post-eject handling.` (evidence: `0x00738104..0x0073812A`)
- `[RESOLVED] OQ-13 - Does current Rust kill transport passengers on destruction? -> Yes for non-garrison cargo, by setting `health=0`, `dying=true`, and `PassengerRole::None`.` (evidence: `src/sim/combat/mod.rs:925..930`)
- `[RESOLVED] OQ-14 - Does current Rust have a native-like per-passenger `+0x82` flag clear point? -> No explicit equivalent found; OpenTopped is represented by passenger role plus transport/weapon override surfaces.` (evidence: `src/sim/passenger.rs:443..493`, `src/sim/combat/combat_weapon.rs:45..55`)
- `[DEFERRED] OQ-15 - Does normal manual unload still leave `+0x82` set as older docs suspected?` (category: `out-of-scope`; reason: this slot was scoped to fatal damage/destruction; next-step-if-pursued: focused normal BFRT unload report with vtable paths and runtime check)
- `[DEFERRED] OQ-16 - Exact semantic names for passenger vtable `+0x174`, `+0x3C8`, and transport vtable `+0x124` in this path.` (category: `requires-different-system-context`; reason: ordering and arguments are verified, but canonical names require broader Techno virtual-slot audit; next-step-if-pursued: synthesize with TechnoClass vtable layout docs)
- `[DEFERRED] OQ-17 - Full crashable cargo behavior for jumpjet/aircraft-style transports.` (category: `out-of-scope`; reason: BFRT is non-crashable and this target is BFRT-like OpenTopped unit death; next-step-if-pursued: crashable transport cargo death trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fatal `UnitClass::ReceiveDamage` clears OpenTopped passenger byte `+0x82` only after death result `4`, not on nonfatal damage. | `0x00737C90`, `0x00737F80..0x00737F92` | Missing explicit per-passenger flag/timing | `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, future native lifecycle API | Add a per-passenger OpenTopped-contained/fire flag or equivalent state and clear it only in the fatal transport-destruction path before cargo ejection/death. | `open_topped_passenger_flag_not_cleared_on_nonfatal_transport_damage`; `open_topped_passenger_flag_cleared_before_bfrt_death_eject` | Do not clear OpenTopped firing state merely because the transport was hit. |
| `ClearAllInOpenTransport` only writes passenger `+0x82=0`; it does not unregister active logic or unhide/eject passengers. | `0x007104C0..0x007104E9` | Rust currently has no active-list-equivalent flag separation | `src/sim/world/mod.rs`, `src/sim/passenger.rs` | Keep OpenTopped passengers live-active while contained; clearing firing/contained-open flag must be separate from active-vector membership and separate from reveal/despawn. | `clear_all_open_topped_flags_keeps_passengers_live_until_eject_or_death` | Do not implement this helper as `PassengerRole::None`, `despawn`, or live-vector unregister. |
| After the clear, native attempts to eject cargo alive; passengers die only when ejection/reveal fails. | `0x00737FC4..0x007381B6` | Mismatch: Rust kills all non-garrison transport riders on transport destruction. | `src/sim/combat/mod.rs:925..930`, passenger ejection helpers in `src/sim/passenger.rs`, occupancy placement | On BFRT/transport death, pop cargo in native order, attempt native-like reveal/eject placement near the transport, and only mark a passenger dying if placement fails. | `bfrt_destroyed_with_open_adjacent_cell_ejects_passenger_alive`; `bfrt_destroyed_with_blocked_eject_cells_kills_passenger` | Do not preserve the current blanket "kill riders" behavior for BFRT-like transport destruction. |
| Successful ejection clears passenger transport pointer before post-eject mission/order handling; OpenTopped different-owner passengers get an extra `+0x3C8(0)` call. | `0x007380FA`, `0x00738104..0x0073812A` | Unchecked/missing | `src/sim/passenger.rs`, order/target cleanup fields on `GameEntity` | On successful death-eject, clear transport role before post-eject order assignment and handle target/order cleanup for cross-owner OpenTopped passengers. | `bfrt_death_eject_clears_inside_role_before_scatter_order`; `captured_or_mixed_owner_bfrt_death_clears_passenger_target_state` | Do not leave passenger logically inside transport while issuing the scatter/eject mission. |
| The dying transport's own final `+0xF8` cleanup occurs after cargo processing in the ordinary non-crashable path. | `0x007381AE..0x007381B6`, `0x00738493..0x007384A5` | Current Rust death handling snapshots cargo then marks riders before/despite despawn; native ordering not modeled | `src/sim/combat/mod.rs`, future pending-delete queue/lifecycle API | Preserve order: death explosion/visuals, clear OpenTopped flags, passenger cargo processing, then transport uninit/pending-delete. | `bfrt_death_order_clears_and_ejects_cargo_before_transport_uninit_event` | Do not despawn/remove the transport before cargo logic has the native view of its coordinates/type/owner. |
| BFRT activates this path in standard YR. | `ini/rulesmd.ini:6931..6933`; `0x00737F80..0x00737F92` | Mismatch affects stock Allied late-game unit | `src/rules/object_type.rs`, `src/sim/passenger.rs`, `src/sim/combat/mod.rs` | Treat this as stock gameplay, not a mod-only edge case. | `stock_bfrt_open_topped_death_uses_open_topped_cargo_clear_path` | Do not defer under "mods only"; BFRT is standard YR content. |

## 10. Stale Docs / Follow-up Docs

- `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` should replace "Caller of `ClearAllInOpenTransport`: `UnitClass__ReceiveDamage` (when the open-topped transport takes damage - possibly during destruction or area damage that hits passengers)" with: "`CargoClass::ClearAllInOpenTransport @ 0x007104C0` is directly called only by `UnitClass::ReceiveDamage @ 0x00737C90`, and in the verified caller it runs only on fatal result `4`, gated by the dying unit type's `OpenTopped` byte at `+0x5E4`. It clears passenger `Techno+0x82` before cargo ejection/death processing; it does not run on nonfatal damage."
- `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md` should replace "transport damaged" wording with "transport destroyed/fatal damage" for this helper.
- `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md`, if it describes OpenTopped transport death as "Hide / RemoveFromMap-ish" or blanket rider death, should use: "fatal OpenTopped unit damage first clears passenger `+0x82`, then native cargo processing attempts alive ejection; failed ejection reaches passenger limbo/uninit."
- Current Rust comments at `src/sim/combat/mod.rs:894..925` already flag transport rider death as a parity gap; this report supplies the native BFRT mechanism for closing it.

## Sources

- Live Ghidra decompile: `TechnoClass::ClearInOpenTransport @ 0x007104A0`, `CargoClass::ClearAllInOpenTransport @ 0x007104C0`, `UnitClass::ReceiveDamage @ 0x00737C90`, `FootClass::ReceiveDamage @ 0x004D7330`, `FootClass::EMPPassengers @ 0x00707CB0`, `FUN_00473450 @ 0x00473450`, `FUN_00473430 @ 0x00473430`, `FUN_004DE710 @ 0x004DE710`, `UnitClass::Death_Explosion @ 0x00738680`.
- Live Ghidra caller/callee evidence: `get_function_callers(0x007104C0)`, `get_function_callers(0x007104A0)`, `get_function_callees(0x00737C90)`.
- Live Ghidra assembly contexts: `0x007104A0..0x007104E9`, `0x00737F74..0x00737FAB`, `0x00737FC4..0x007381B6`, `0x00738457..0x007384A5`.
- Research index/doc context: `IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md`, `SET_IN_OPEN_TRANSPORT_VTABLE_3D0_RESWARM_20260528.md`, `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md`, `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`, `units/AUDIT_INDEX.md`.
- INI checked: `ini/rulesmd.ini` `[BFRT]`, `[CombatDamage]`.
- Rust source scanned: `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, `src/sim/combat/combat_weapon.rs`, `src/rules/object_type.rs`, `src/rules/weapon_type.rs`.
