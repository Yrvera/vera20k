# Techno Eject Passenger Virtual Slots - Re-Swarm Research Report

**Address(es):** `0x00737C90` (`UnitClass::ReceiveDamage` fatal cargo slice), passenger virtual calls at `0x007380EC`, `0x0073812A`, `0x0073813D`, `0x00738191`, `0x0073819B`, transport virtual call at `0x00737F7A`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Identify the Techno/Foot virtual slots used around passenger ejection and failed-ejection death in the ordinary non-crashable `UnitClass::ReceiveDamage` cargo path, especially `+0x174`, `+0x3C8`, `+0x124`, `+0xD8`, `+0xE0`, and `+0xF8`.  
**Non-Scope:** full fatal transport damage timing, normal manual unload, full crashable/jumpjet cargo behavior, all scatter internals, and full Techno vtable taxonomy outside the named slots.  
**Confidence:** High for call ordering, arguments, Unit/Infantry bindings, and Rust handoff. Medium for the friendly semantic names of unlabelled `0x0051DFF0` and some decompiler-recovered stack parameter names.  
**Active in YR:** Yes. Standard `[BFRT]` has `Passengers=5` and `OpenTopped=yes`; the verified path is the stock `UnitClass::ReceiveDamage` fatal result `4` cargo handling path.

## 1. Overview

The fatal cargo path uses six relevant virtual slots, but they do not all dispatch on the same object. The earlier `+0x124` call is on the dying transport and resolves to `TechnoClass::DoCloak(0)` for Unit/Infantry/Foot-family vtables; it runs before OpenTopped flag clearing and before passenger ejection.

The passenger loop then pops each cargo item and tries to place it back on the map through passenger `+0xD8` (`Unlimbo`). Successful ejection clears the passenger transport link, optionally clears target/archive state through `+0x3C8(0)` for mixed-owner OpenTopped cargo, calls passenger `+0x174(DAT_00B1CFE8, 1, 0)` scatter, then assigns follow-up mission/order. Failed ejection records kill/cell side effects through passenger `+0xE0(killer/source)` and then calls passenger `+0xF8`, which resolves to `FootClass::UnInit` for Unit/Infantry passengers.

## 2. Slot Map / Key Offsets

| Slot / field | Receiver in this path | Unit binding | Infantry binding | Verified semantics in this cargo path | Evidence |
|---|---|---|---|---|---|
| vtable `+0x124` | dying transport (`ESI`) | `0x004D3780` `TechnoClass::DoCloak` | same | Called as `DoCloak(0)` before OpenTopped clear/cargo loop; if mark succeeds and visual state is ground-like, removes from multicell occupancy for cloak/visibility transition. | `0x00737F74..0x00737F7A`; vtable reads `0x007F5D94`, `0x007EB17C`; decompile `0x004D3780` |
| vtable `+0xD8` | popped passenger (`EDI`) | `0x00737BA0` `UnitClass::Unlimbo` | `0x0051DFF0` unlabelled Infantry unlimbo body | Attempts to reveal/unlimbo passenger at transport coords, with a byte argument derived from `RateTimer::Current()`. Return `AL==0` sends passenger to failure/death path. | `0x007380D0..0x007380F4`; vtable reads `0x007F5D48`, `0x007EB130`; decompile `0x00737BA0`; bytes `0x0051DFF0` |
| passenger `+0x11C` | popped passenger | field write | field write | Transport/link pointer is cleared only after `+0xD8` succeeds. | `0x007380FA` |
| vtable `+0x3C8` | popped passenger (`EDI`) | `0x006FCDB0` `TechnoClass::Set_ArchiveTarget` | `0x0051B1F0` Infantry wrapper then archive-target clear | Called as `+0x3C8(0)` only when dying transport is OpenTopped and passenger owner differs from transport owner. Clears archive/target state; Infantry wrapper also clears firing/sequence state before delegating. | `0x00738104..0x0073812A`; vtable reads `0x007F6038`, `0x007EB420`; decompile `0x006FCDB0`, `0x0051B1F0` |
| vtable `+0x174` | popped passenger (`EDI`) | `0x00743A50` `UnitClass::Scatter` | `0x0051D0D0` `InfantryClass::Scatter` | Called as `Scatter(&DAT_00B1CFE8, 1, 0)` after successful `+0xD8`/transport-link clear/optional target clear. Can queue movement mission `2` and set destination; may consume scenario RNG. | `0x00738130..0x0073813D`; vtable reads `0x007F5DE4`, `0x007EB1CC`; decompile scatter functions; prior scatter docs |
| vtable `+0xE0` | popped passenger (`EDI`) | `0x00744720` `UnitClass::OnEnterCell_Triggers` wrapper ending in `TechnoClass::RecordKill` | `0x00702D40` `TechnoClass::RecordKill` | Failed-ejection path calls `+0xE0(killer/source_arg)` before uninit. For Unit, wrapper optionally runs cell actions, then records kill. For Infantry, direct `RecordKill`. | `0x00738188..0x00738191`; vtable reads `0x007F5D50`, `0x007EB138`; decompile `0x00744720`, `0x00702D40` |
| vtable `+0xF8` | popped passenger (`EDI`) | `0x004DE5D0` `FootClass::UnInit` | same | Failed-ejection path finalizes passenger through Foot/Object cleanup and pending-delete enqueue; not scalar destructor directly. | `0x00738197..0x0073819B`; vtable reads `0x007F5D68`, `0x007EB150`; decompile `0x004DE5D0` |
| `Techno+0x21C` | transport/passenger | owner pointer | owner pointer | Owner compare gates the optional OpenTopped mixed-owner `+0x3C8(0)` call. | `0x00738114..0x00738122` |
| passenger `+0x30` / cargo head | cargo chain | next pointer | next pointer | Cargo pop removes head and clears popped passenger next pointer before ejection attempt. | `FUN_004DE710`, `FUN_00473430` |

## 3. Core Logic

### 3.1 Ordered fatal cargo slot sequence

Relevant order after fatal `FootClass::ReceiveDamage` result `4` reaches the common cargo branch:

```text
dying_transport->vtable[0x124](0)
if dying_transport.Type.OpenTopped:
    CargoClass::ClearAllInOpenTransport()      ; clears passenger +0x82 only
if dying_transport->vtable[0x1C8]() > 0xD0:
    FootClass::EMPPassengers()
if dying_transport.Type.Crashable == false:
    while cargo head exists:
        passenger = CargoClass::PopFirst()
        if passenger == null:
            continue

        cell = CellClass::Get_Cell_At(transport coords)
        can_enter = passenger->vtable[0x1AC](cell, -1, -1, 0)
        accepted = (can_enter == 0 || can_enter == 2)

        g_MapEditorMode += 1
        passenger.OnBridge = transport.OnBridge
        build unlimbo coordinate from transport cell/bridge state

        if accepted and not transport byte +0x8F and not transport IsABomb:
            facing_arg = ((RateTimer::Current() >> 7) + 1) >> 1 & 0xFF
            if passenger->vtable[0xD8](&coord, facing_arg) succeeds:
                passenger[0x11C] = 0
                if transport.Type.OpenTopped and transport.Owner != passenger.Owner:
                    passenger->vtable[0x3C8](0)
                passenger->vtable[0x174](&DAT_00B1CFE8, 1, 0)
                run AI/player follow-up mission/selection handling
            else:
                passenger->vtable[0xE0](source_arg)
                passenger->vtable[0xF8]()
        else:
            passenger->vtable[0xE0](source_arg)
            passenger->vtable[0xF8]()

        g_MapEditorMode -= 1
```

Assembly anchors:

- `0x00737F74..0x00737F7A`: `PUSH 0`, then call dying transport `+0x124`.
- `0x007380D0..0x007380EC`: `RateTimer::Current`, derive byte argument with `SHR 7`, `INC`, `SHR 1`, `AND 0xFF`, push it, push coord pointer, call passenger `+0xD8`.
- `0x007380F2..0x007380FA`: zero return from `+0xD8` branches to failure; success writes `passenger+0x11C = 0`.
- `0x00738104..0x0073812A`: OpenTopped byte and owner compare gate passenger `+0x3C8(0)`.
- `0x00738130..0x0073813D`: `PUSH 0`, `PUSH 1`, `PUSH 0x00B1CFE8`, call passenger `+0x174`.
- `0x00738188..0x0073819B`: failure path pushes `[ESP+0x54]`, calls passenger `+0xE0`, then passenger `+0xF8`.
- `0x007381A1..0x007381B6`: decrements `g_MapEditorMode`, reloads cargo head, loops.

### 3.2 `+0x124` is transport-side `TechnoClass::DoCloak(0)`

The dying transport call at `0x00737F7A` is not an ejected-passenger callback. For UnitClass vtable base `0x007F5C70`, slot address `0x007F5D94` contains `0x004D3780`; Infantry/Foot-family vtables also bind their `+0x124` slot to `0x004D3780`.

`TechnoClass::DoCloak @ 0x004D3780`:

- returns `1` immediately if argument is `2`;
- otherwise calls `TechnoClass::ProcessCloakAndNotify(arg)`;
- if that succeeds and visual state virtual `+0x78` returns `2`, gets cell coords through `+0x1B8`;
- with argument `0`, calls `TechnoClass::ExitCell_RemoveFromMultiCells`;
- with argument `1` or `3`, calls `TechnoClass::EnterCell_AddToMultiCells`.

In this path the argument is literal `0`, so the transport executes the exit/remove side of this helper before OpenTopped cargo flag clearing and passenger ejection.

### 3.3 `+0xD8` is passenger unlimbo/reveal

The ejection path only proceeds to post-eject scatter if passenger `+0xD8` returns nonzero. UnitClass binds this slot to `UnitClass::Unlimbo @ 0x00737BA0`, which calls `FootClass::Unlimbo`, updates facing from the byte argument, and initializes Unit movement/unload fields (`+0xF8`, `+0x100..+0x10C`) depending on type flags `+0xE18/+0xE19`.

InfantryClass binds the same slot to `0x0051DFF0`. Ghidra did not expose a named function boundary there, but vtable memory and raw bytes show a real function body beginning with a stack frame and a call with `ECX=0x87F7E8` followed by coordinate work. In this cargo path it is the Infantry unlimbo/reveal equivalent; the semantic claim is grounded by the shared `+0xD8` slot role and success/failure branch at `0x007380F2`.

Tiny details:

- The second argument is not a random direction draw. It is derived from `RateTimer::Current()` as `((current >> 7) + 1) >> 1 & 0xFF`.
- `g_MapEditorMode` is incremented before the unlimbo call and decremented after success/failure handling.
- `passenger.OnBridge` is copied from the dying transport before placement.
- The passenger transport/link pointer `+0x11C` is cleared only after `+0xD8` succeeds.

### 3.4 `+0x3C8(0)` is conditional target/archive-target clearing

The mixed-owner OpenTopped branch is narrower than a general post-eject cleanup:

```text
if transport.Type.OpenTopped != 0 and transport.Owner != passenger.Owner:
    passenger->vtable[0x3C8](0)
```

UnitClass binds `+0x3C8` to `TechnoClass::Set_ArchiveTarget @ 0x006FCDB0`. With argument `0`, the effect is archive/target clearing plus related spawn/attach cleanup when the old target changes.

InfantryClass binds `+0x3C8` to `0x0051B1F0`, an Infantry wrapper that can clear infantry firing/action state (`+0x68D`, sequence/action helpers, `+0x2A8` on a linked object) before calling `TechnoClass::Set_ArchiveTarget(0)`.

This means Rust must not clear passenger attack/archive state unconditionally for all successful ejections just because a passenger leaves a dying transport. The native extra clear is only for OpenTopped cargo whose owner differs from the dying transport owner.

### 3.5 `+0x174(DAT_00B1CFE8, 1, 0)` is class scatter

UnitClass binds `+0x174` to `UnitClass::Scatter @ 0x00743A50`; InfantryClass binds it to `InfantryClass::Scatter @ 0x0051D0D0`. Existing scatter reports verify this as the scatter virtual. In this cargo path the arguments are:

- first arg: pointer/global coord `DAT_00B1CFE8`;
- second arg: `1`;
- third arg: `0`.

For infantry, prior scatter research shows the direct scatter path can consume scenario RNG, scan nearby passable cells, queue mission `2`, and set a destination. For UnitClass, the same slot is the Unit scatter implementation with unit-specific locomotor/deploy gates.

The call happens after successful unlimbo and after `passenger+0x11C = 0`, so native ordering makes the passenger no longer logically inside the transport before scatter/order work.

### 3.6 Failed ejection uses `+0xE0(source)` then `+0xF8()`

Failure is reached when:

- `Can_Enter_Cell` result is neither `0` nor `2`;
- transport byte `+0x8F` branch blocks the success attempt;
- transport `IsABomb` blocks the success attempt;
- passenger `+0xD8` returns zero.

Then:

```text
source_arg = [ESP + 0x54] in UnitClass::ReceiveDamage stack frame
passenger->vtable[0xE0](source_arg)
passenger->vtable[0xF8]()
```

`+0xE0` class bindings:

- UnitClass: `0x00744720` `UnitClass::OnEnterCell_Triggers`, which conditionally runs `TechnoClass::ProcessCellAction` cases `7`, `0x30`, and `0x1D`, then calls `TechnoClass::RecordKill(source_arg)`.
- InfantryClass: `0x00702D40` `TechnoClass::RecordKill(source_arg)` directly.

`+0xF8` class bindings:

- UnitClass: `0x004DE5D0` `FootClass::UnInit`.
- InfantryClass: `0x004DE5D0` `FootClass::UnInit`.

`FootClass::UnInit` is not a scalar destructor call. It runs Foot cleanup (capture manager, chrono/deploy hook, passenger/transport link cleanup through `FUN_006EA870` when `+0x5D4` is non-null) and then calls `ObjectClass::UnInit`, which queues pending delete.

## 4. INI Keys

| Key | Scope | Stock value relevant here | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `Passengers=` | transport type | `[BFRT] Passengers=5` | Makes the cargo loop reachable. | Yes |
| `OpenTopped=` | transport type | `[BFRT] OpenTopped=yes` | Gates earlier `ClearAllInOpenTransport`; also gates the optional mixed-owner `+0x3C8(0)` check in the ejection loop. | Yes |
| `Crashable=` | unit type | BFRT has no visible override; default false | If true at type `+0xD95`, skips the ordinary cargo eject loop. | Conditional; BFRT ordinary path is non-crashable |
| `OpenTransportWeapon=` | passenger TechnoType | several infantry set `0` or `1` in `rulesmd.ini` | Not read by the slot calls directly; relevant because BFRT passengers remain active/open-topped before fatal ejection. | Conditional by passenger type |
| `Gunner=` | transport type | IFV uses it; BFRT does not | Cargo pop helper `FUN_004DE710` can notify gunner state via `+0x4D8`; not one of the scoped ejection/death slots. | Conditional |

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Fatal damage owner | `UnitClass::ReceiveDamage` reaches this cargo loop only after death result `4`; this report consumes the prior fatal timing report instead of re-proving all gates. | `0x00737C90`; prior `CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md` | Yes |
| Cargo pop | `FUN_004DE710` calls `FUN_00473430`, removing cargo head, clearing popped passenger `+0x30`, decrementing cargo count; gunner path may call `+0x4D8`. | decompile `0x004DE710`, `0x00473430` | Yes |
| Placement gate | Passenger `+0x1AC(cell,-1,-1,0)` must return `0` or `2` before the success branch attempts `+0xD8`. | `0x00737FF0..0x00738013` decompile | Yes |
| Transport visibility/occupancy hook | Dying transport `+0x124(0)` occurs before OpenTopped clear and passenger ejection. | `0x00737F74..0x00737F80`; `0x004D3780` | Yes |
| Successful ejection | `+0xD8` success -> clear `+0x11C` -> optional `+0x3C8(0)` -> `+0x174(DAT,1,0)` -> mission/selection follow-up. | `0x007380EC..0x00738180` | Yes |
| Failed ejection | `+0xE0(source)` -> `+0xF8()` while `g_MapEditorMode` remains incremented; decrement happens afterward. | `0x00738188..0x007381A8` | Yes |

## 6. Current Rust Implementation Status

Relevant current Rust surfaces:

- `src/sim/passenger.rs` models transport/garrison cargo through `PassengerRole::{Transport, Boarding, Inside}` and unload helpers.
- Boarding sets `PassengerRole::Inside`, clears movement/attack/order state, and puts `WeaponOverride::OpenTransport` on the transport for open-topped cases (`src/sim/passenger.rs:478..493`, `src/sim/passenger.rs:730..748`).
- `src/sim/combat/mod.rs:925..930` currently blanket-kills non-garrison transport riders on transport death by setting `health=0`, `dying=true`, and `PassengerRole::None`.
- `src/sim/combat/mod.rs:1370..1371` skips all `PassengerRole::Inside` attackers, with OpenTopped explicitly deferred.
- `src/sim/game_entity.rs` has movement, attack, radio, rally, order intent, and dying fields that correspond to pieces touched by native `+0x3C8`, `+0x174`, `+0xE0`, and `+0xF8`, but no virtual-slot/lifecycle layer.
- `src/sim/world/mod.rs` has `live_object_order` and `despawn_entity`, but no native pending-delete/class-finalizer split for passenger failed ejection.

Main deltas:

- Rust does not attempt native alive ejection first for non-garrison transport cargo death.
- Rust does not model the native order "unlimbo succeeds, clear transport link, optional target clear, scatter, then mission follow-up."
- Rust has no class-dispatched `RecordKill`/Unit cell-trigger wrapper before failed-ejection `UnInit`.
- Rust generic death/despawn is not equivalent to passenger `+0xF8 -> FootClass::UnInit -> ObjectClass::UnInit -> pending-delete`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `UnitClass::ReceiveDamage` fatal cargo slot order | verified | decompile `0x00737C90`; assembly contexts `0x00737F74`, `0x007380EC`, `0x0073812A`, `0x0073813D`, `0x00738191`, `0x0073819B` | none for named slots |
| `+0x124` transport binding | verified | vtable reads `0x007F5D94`, `0x007EB17C`; decompile `TechnoClass::DoCloak @ 0x004D3780` | full visual/cloak side effects outside scope |
| `+0xD8` Unit binding | verified | vtable read `0x007F5D48 -> 0x00737BA0`; decompile `UnitClass::Unlimbo` | none for slot identity |
| `+0xD8` Infantry binding | touched-not-exhausted | vtable read `0x007EB130 -> 0x0051DFF0`; bytes at `0x0051DFF0`; shared slot role and cargo success branch | Ghidra has no function boundary/name; full Infantry unlimbo body remains deferred |
| `+0x3C8` Unit binding | verified | vtable read `0x007F6038 -> 0x006FCDB0`; decompile `TechnoClass::Set_ArchiveTarget` | none for arg `0` semantics |
| `+0x3C8` Infantry binding | verified | vtable read `0x007EB420 -> 0x0051B1F0`; decompile wrapper | exact labels for all infantry action bytes outside scope |
| `+0x174` Unit binding | verified | vtable read `0x007F5DE4 -> 0x00743A50`; decompile and prior scatter doc | full Unit scatter internals outside scope |
| `+0x174` Infantry binding | verified | vtable read `0x007EB1CC -> 0x0051D0D0`; decompile and prior scatter doc | full Infantry scatter internals already covered elsewhere |
| `+0xE0` Unit binding | verified | vtable read `0x007F5D50 -> 0x00744720`; decompile wrapper | exact ProcessCellAction case user-facing names outside scope |
| `+0xE0` Infantry binding | verified | vtable read `0x007EB138 -> 0x00702D40`; decompile `TechnoClass::RecordKill` | full kill-stat side effects beyond cargo failure outside scope |
| `+0xF8` Unit/Infantry binding | verified | vtable reads `0x007F5D68`, `0x007EB150 -> 0x004DE5D0`; decompile `FootClass::UnInit` | pending-delete drain already covered by separate report |
| Cargo pop helper | verified | decompile `FUN_004DE710`, `FUN_00473430` | gunner `+0x4D8` details outside named-slot scope |
| Rust passenger/combat surfaces | verified-by-scan | `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, `src/sim/game_entity.rs`, `src/sim/world/mod.rs` | implementation not performed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which object receives `+0x124` in this path? -> The dying transport, not a passenger.` (evidence: `0x00737F74..0x00737F7A`, receiver in `ESI`)
- `[RESOLVED] OQ-02 - What does `+0x124` bind to for Unit/Infantry/Foot-family objects? -> `TechnoClass::DoCloak @ 0x004D3780`.` (evidence: vtable reads `0x007F5D94`, `0x007EB17C`; decompile `0x004D3780`)
- `[RESOLVED] OQ-03 - What argument does the cargo path pass to transport `+0x124`? -> literal `0`.` (evidence: `0x00737F76 PUSH 0`)
- `[RESOLVED] OQ-04 - Which object receives `+0xD8`? -> The popped passenger in `EDI`.` (evidence: `0x007380D7 MOV EDX,[EDI]`; `0x007380EC CALL [EDX+0xD8]`)
- `[RESOLVED] OQ-05 - What are `+0xD8` arguments? -> coord pointer and byte `((RateTimer::Current() >> 7) + 1) >> 1 & 0xFF`.` (evidence: `0x007380D0..0x007380EC`)
- `[RESOLVED] OQ-06 - What does `+0xD8` mean for Unit passengers? -> `UnitClass::Unlimbo`.` (evidence: vtable read `0x007F5D48 -> 0x00737BA0`; decompile `UnitClass::Unlimbo`)
- `[RESOLVED] OQ-07 - What does `+0xD8` mean for Infantry passengers? -> Infantry unlimbo-equivalent body at `0x0051DFF0`, but Ghidra has no function boundary/name.` (evidence: vtable read `0x007EB130 -> 0x0051DFF0`; raw bytes at `0x0051DFF0`; shared cargo branch semantics)
- `[RESOLVED] OQ-08 - When is passenger `+0x11C` cleared? -> Only after `+0xD8` succeeds.` (evidence: `0x007380F2..0x007380FA`)
- `[RESOLVED] OQ-09 - What gates passenger `+0x3C8(0)`? -> Dying transport type `OpenTopped` byte is nonzero and transport owner differs from passenger owner.` (evidence: `0x00738104..0x0073812A`)
- `[RESOLVED] OQ-10 - What does Unit passenger `+0x3C8` bind to? -> `TechnoClass::Set_ArchiveTarget @ 0x006FCDB0`.` (evidence: vtable read `0x007F6038`; decompile `0x006FCDB0`)
- `[RESOLVED] OQ-11 - What does Infantry passenger `+0x3C8` bind to? -> Infantry wrapper `0x0051B1F0`, which can clear infantry action/firing state then delegates to archive-target clear.` (evidence: vtable read `0x007EB420`; decompile `0x0051B1F0`)
- `[RESOLVED] OQ-12 - What arguments does passenger `+0x174` receive? -> `DAT_00B1CFE8`, `1`, `0`.` (evidence: `0x00738130..0x0073813D`)
- `[RESOLVED] OQ-13 - What does passenger `+0x174` bind to? -> Unit/Infantry class scatter implementations.` (evidence: vtable reads `0x007F5DE4`, `0x007EB1CC`; decompile `UnitClass::Scatter`, `InfantryClass::Scatter`)
- `[RESOLVED] OQ-14 - What happens on failed ejection before uninit? -> passenger `+0xE0(source_arg)` runs before `+0xF8`.` (evidence: `0x00738188..0x0073819B`)
- `[RESOLVED] OQ-15 - What does `+0xE0` bind to? -> Unit uses `UnitClass::OnEnterCell_Triggers` wrapper ending in `RecordKill`; Infantry uses `TechnoClass::RecordKill` directly.` (evidence: vtable reads `0x007F5D50`, `0x007EB138`; decompile `0x00744720`, `0x00702D40`)
- `[RESOLVED] OQ-16 - What does passenger `+0xF8` bind to? -> `FootClass::UnInit` for Unit and Infantry passengers.` (evidence: vtable reads `0x007F5D68`, `0x007EB150`; decompile `0x004DE5D0`)
- `[RESOLVED] OQ-17 - Is `+0xF8` an immediate scalar destructor here? -> No; it is `FootClass::UnInit`, which reaches `ObjectClass::UnInit` and pending delete.` (evidence: decompile `0x004DE5D0`; prior pending-delete reports)
- `[RESOLVED] OQ-18 - Is this stock-live? -> Yes for BFRT/OpenTopped fatal cargo because `[BFRT] Passengers=5` and `OpenTopped=yes`.` (evidence: `ini/rulesmd.ini:6931..6932`; prior fatal cargo timing report)
- `[RESOLVED] OQ-19 - Does current Rust model these virtual slot boundaries? -> No direct virtual/lifecycle layer exists; transport death currently blanket-kills non-garrison riders.` (evidence: `src/sim/combat/mod.rs:925..930`; `src/sim/passenger.rs` scan)
- `[DEFERRED] OQ-20 - Full `0x0051DFF0` Infantry unlimbo body and best canonical name.` (category: `requires-different-system-context`; reason: Ghidra lacks a function boundary at the vtable target; slot identity is enough for this cargo virtual-slot report; next-step-if-pursued: dedicated Infantry unlimbo function-boundary audit without mutating Ghidra)
- `[DEFERRED] OQ-21 - Full Unit/Infantry scatter internals from this specific call's `DAT_00B1CFE8,1,0` arguments.` (category: `out-of-scope`; reason: scatter has separate reports and this target only needs slot semantics/order; next-step-if-pursued: trace cargo-eject scatter as a dedicated action trace)
- `[DEFERRED] OQ-22 - Crashable transport cargo virtual-slot behavior.` (category: `out-of-scope`; reason: BFRT ordinary cargo loop is non-crashable; next-step-if-pursued: crashable/jumpjet cargo death investigation)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Fatal cargo handling calls dying transport `+0x124(0)` before OpenTopped clear and passenger loop; this is `TechnoClass::DoCloak(0)`, not passenger cleanup. | `0x00737F74..0x00737F80`; `0x004D3780` | missing | future lifecycle API; `src/sim/combat/mod.rs` transport death path; occupancy/multicell surfaces | Preserve a transport-side pre-cargo visibility/occupancy hook before passenger clear/eject handling. | `transport_death_runs_visibility_exit_before_cargo_ejection` | Do not model `+0x124` as a passenger ejection callback or skip it as cosmetic. |
| Successful cargo ejection requires passenger `+0xD8` unlimbo success before clearing transport link. | `0x007380EC..0x007380FA`; vtable reads `Unit +0xD8`, `Inf +0xD8` | mismatch: Rust transport death does not attempt alive ejection for non-garrison cargo | `src/sim/combat/mod.rs`, `src/sim/passenger.rs`, placement/occupancy helpers | Pop each cargo passenger, attempt native-like unlimbo/reveal placement, and clear inside/transport link only after placement succeeds. | `bfrt_death_open_adjacent_cell_unlimbos_passenger_before_clearing_inside_role` | Do not set `PassengerRole::None` before placement success is known. |
| `+0xD8` facing/placement byte is derived from `RateTimer::Current`, not raw RNG. | `0x007380D0..0x007380E6` | missing | future ejection placement/scatter helper | Use the verified frame/timer-derived byte when modeling ejection orientation/placement. | `fatal_transport_eject_orientation_uses_rate_timer_derived_byte` | Do not consume scenario RNG for this argument. |
| Mixed-owner OpenTopped successful ejection calls passenger `+0x3C8(0)`; same-owner or non-OpenTopped ejection skips it. | `0x00738104..0x0073812A`; `0x006FCDB0`; `0x0051B1F0` | missing/unchecked | `src/sim/passenger.rs`, `src/sim/game_entity.rs` target/order fields | Clear archive/attack/infantry firing state only for the native mixed-owner OpenTopped condition. | `mixed_owner_bfrt_death_eject_clears_passenger_target_once`; `same_owner_bfrt_eject_preserves_target_until_scatter_or_mission_changes_it` | Do not clear all passenger targets unconditionally on ejection. |
| Successful ejection calls class scatter `+0x174(DAT_00B1CFE8,1,0)` after link clear and optional target clear. | `0x00738130..0x0073813D`; scatter function vtable reads | partial/mismatch: Rust uses approximate unload/eject movement helpers | `src/sim/passenger.rs`, movement/order-intent surfaces | Dispatch Unit/Infantry-specific scatter-equivalent behavior after successful ejection, preserving native order relative to link clear and mission follow-up. | `bfrt_death_ejected_infantry_runs_scatter_before_guard_or_mission_followup` | Do not replace scatter with a generic first-free-cell move if exact scatter is required. |
| Failed ejection records kill/cell side effects through `+0xE0(source_arg)` before `+0xF8()`. | `0x00738188..0x0073819B`; `0x00744720`; `0x00702D40` | missing | `src/sim/combat/mod.rs`, kill credit/stat/event surfaces | Before marking failed-eject passenger for cleanup, run class-specific kill-record/cell-trigger semantics. | `blocked_bfrt_death_eject_records_passenger_kill_before_uninit` | Do not simply set `dying=true` without kill attribution/cell trigger side effects. |
| Failed-eject `+0xF8` resolves to `FootClass::UnInit`, not immediate removal/destructor. | `0x00738197..0x0073819B`; vtable reads; decompile `0x004DE5D0`; pending-delete reports | mismatch: generic `despawn_entity`/dying path lacks native class finalizer timing | `src/sim/world/mod.rs`, future pending-delete queue, passenger death cleanup | Failed ejection should go through Foot/Object uninit and pending-delete timing, preserving class cleanup order. | `blocked_bfrt_passenger_enters_pending_delete_after_recordkill_not_immediate_remove` | Do not call generic entity removal at the failure point. |
| `g_MapEditorMode` remains incremented across both successful and failed passenger handling and is decremented after slot calls. | `0x007380EC..0x007381A8` | missing | future ejection/reveal placement context | Preserve any map-editor-silencing/context behavior around unlimbo/eject/death if modeled. | `fatal_cargo_eject_suppresses_map_editor_side_effects_until_after_passenger_handling` | Do not decrement context before failed-ejection `+0xE0/+0xF8`. |

### Stale Docs / Follow-up Docs

- `CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md` deferred exact names for these slots. Replacement wording: "`+0x124` in the fatal cargo slice is a dying-transport call to `TechnoClass::DoCloak(0)`. Passenger `+0xD8` is Unlimbo/reveal; `+0x3C8(0)` is conditional mixed-owner OpenTopped target/archive clearing; `+0x174(DAT_00B1CFE8,1,0)` is class Scatter; failed ejection calls passenger `+0xE0(source)` for RecordKill/cell-trigger side effects and then `+0xF8`, which resolves to `FootClass::UnInit` for Unit/Infantry passengers."
- Older prose that says failed cargo ejection directly destroys passengers should be replaced with: "failed ejection records kill/cell effects through class virtual `+0xE0`, then calls `FootClass::UnInit` through `+0xF8`; actual destructor/free is later through pending-delete drain."

## Sources

- Ghidra decompile/read-only:
  - `UnitClass::ReceiveDamage @ 0x00737C90`
  - `TechnoClass::DoCloak @ 0x004D3780`
  - `UnitClass::Unlimbo @ 0x00737BA0`
  - `TechnoClass::Set_ArchiveTarget @ 0x006FCDB0`
  - `FUN_0051B1F0` Infantry `+0x3C8` wrapper
  - `UnitClass::Scatter @ 0x00743A50`
  - `InfantryClass::Scatter @ 0x0051D0D0`
  - `UnitClass::OnEnterCell_Triggers @ 0x00744720`
  - `TechnoClass::RecordKill @ 0x00702D40`
  - `FootClass::UnInit @ 0x004DE5D0`
  - `FUN_004DE710`, `FUN_00473430`, `FUN_00473450`
- Ghidra assembly/read-memory evidence:
  - Call contexts `0x00737F74..0x00737F80`, `0x007380D0..0x007380F4`, `0x00738104..0x0073813D`, `0x00738188..0x0073819B`
  - Unit vtable reads: `0x007F5D48`, `0x007F5D50`, `0x007F5D68`, `0x007F5D94`, `0x007F5DE4`, `0x007F6038`
  - Infantry vtable reads: `0x007EB130`, `0x007EB138`, `0x007EB150`, `0x007EB17C`, `0x007EB1CC`, `0x007EB420`
  - Raw bytes at `0x0051DFF0` for the unlabelled Infantry `+0xD8` target
- Prior research/docs referenced:
  - `CARGO_CLEAR_ALL_IN_OPEN_TRANSPORT_DAMAGE_TIMING_RESWARM_20260528.md`
  - `SET_IN_OPEN_TRANSPORT_VTABLE_3D0_RESWARM_20260528.md`
  - `UNIT_CLASS_SCATTER_GHIDRA_REPORT.md`
  - `GARRISON_EJECTED_INFANTRY_SCATTER_GHIDRA_REPORT.md`
  - `OBJECTCLASS_UNINIT_DEATH_CLEANUP_ORDERING_RESWARM_20260528.md`
  - `PENDING_DELETE_DRAIN_DESTRUCTOR_TIMING_RESWARM_20260528.md`
- INI checked:
  - `ini/rulesmd.ini` `[BFRT]`, `OpenTransportWeapon`, `Crashable`, `Gunner`
- Rust scanned:
  - `src/sim/passenger.rs`
  - `src/sim/combat/mod.rs`
  - `src/sim/game_entity.rs`
  - `src/sim/world/mod.rs`
  - `src/rules/object_type.rs`
