# Garrison Occupant Death Removal via PenetratesBunker - Ghidra Research Report

**Address(es):** `0x00701900` (`TechnoClass::ReceiveDamage`), `0x00522910` (`BuildingClass::AddGarrisonOccupant`), `0x00457DE0` (`BuildingClass::SellBuilding`), `0x006FDD50` (`TechnoClass::Fire_At`), `0x00702D40` (`TechnoClass::RecordKill`), `0x006CE2D0` (`DynamicVectorClass::Remove`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** whether a live `CanBeOccupied` building garrison occupant killed directly by a `PenetratesBunker`-style warhead is removed from `BuildingClass+0x684`, and what happens to count/fire index/pips/fire/kill-credit state.
**Non-Scope:** generic transports, Battle Fortress/open-topped passengers, tank-bunker lifecycle, and full fire-index/XP parity except where directly needed to answer stale-vector effects.
**Confidence:** High for the negative `CanBeOccupied` finding; Medium for artificial direct-damage stale-pointer consequences because no standard YR caller was found that damages limbo garrison occupants directly.
**Active in YR:** Conditional. `PenetratesBunker` is active for bunker/shelter damage routing, but not for `CanBeOccupied` garrison occupant-vector removal.

## Working Notes Gate

- Target question: Does a `PenetratesBunker`-style direct kill remove a garrison occupant from `BuildingClass+0x684`, update `+0x694` count and `+0x69C` fire index, and change pips/fire eligibility/kill credit immediately?
- Non-goals: Do not investigate generic passenger ejection, bunker entry/exit, transport passengers, or full garrison fire-index/XP design beyond direct consequences of occupant death.
- Evidence needed to mark COMPLETE: binary proof of where `PenetratesBunker` is read, binary proof of how `CanBeOccupied` occupants are stored, binary proof for any individual-vector removal or absence of such removal, and Rust-facing handoff.
- Stop conditions: stop after proving the live YR branch either reaches or cannot reach `BuildingClass+0x684` for this target, and after all direct state consequences are resolved or explicitly deferred.

## 1. Overview

`PenetratesBunker` does not implement a `CanBeOccupied` garrison occupant-removal path in `gamemd.exe`. The flag is read by the live `TechnoClass::ReceiveDamage` bunker/shelter branch keyed by `TechnoClass+0x2E4`; `BuildingClass::AddGarrisonOccupant` for civilian/building garrisons instead limbos infantry and appends the infantry pointer to the building's `DynamicVectorClass` at `+0x684`, without setting that `+0x2E4` shelter pointer.

If some out-of-band caller directly damages and kills a limbo garrison occupant object, this slice found no binary path that removes that pointer from the building's occupant vector immediately. Standard YR `PenetratesBunker` does not create that situation for `CanBeOccupied` garrisons.

## 2. Class Layout / Key Offsets

| Owner | Offset | Type | Purpose | Active in YR |
|---|---:|---|---|---|
| `BuildingClass` | `+0x684` | `DynamicVectorClass<InfantryClass*>` vtable | garrison occupant vector object | Yes, `AddGarrisonOccupant` appends here at `0x00522910` |
| `BuildingClass` | `+0x688` | `InfantryClass**` | occupant items buffer | Yes, read by `GetWeapon`, `SellBuilding`, `RecordKill` |
| `BuildingClass` | `+0x694` | `int` | occupant count | Yes, `GetOccupantCount` returns this at `0x004581F0` |
| `BuildingClass` | `+0x69C` | `int` | current garrison fire index | Yes, `GetWeapon`, `GetFireCoords`, `Fire_At`, `SellBuilding`, `RecordKill` |
| `TechnoClass` | `+0x2E4` | pointer/id | bunker/shelter building link used by `PenetratesBunker` damage routing | Conditional, live for bunker/shelter paths, not set by `AddGarrisonOccupant` |
| `WarheadTypeClass` | `+0x146` | bool | `PenetratesBunker` | Conditional, read in `TechnoClass::ReceiveDamage` bunker branch |

`BuildingClass+0x664` was not confirmed as the current garrison fire index in this slice. The live garrison fire index evidence points to `+0x69C`.

## 3. Core Logic

### 3.1 PenetratesBunker read is a bunker/shelter gate, not garrison-vector removal

`WarheadTypeClass::ReadINI_Body` reads `PenetratesBunker` into `WarheadTypeClass+0x146` at `0x0075D53C`. Stock `rulesmd.ini` sets it on `[Parasite]`, `[Super]`, `[Crush]`, `[DemobombWH]`, and `[DiskWH]`.

`TechnoClass::ReceiveDamage` reads `warhead+0x146` only in the `this+0x2E4 != 0` branch. The assembly context around `0x00701BBE` reads `MOV AL, byte ptr [EBP + 0x146]`; for non-building technos in a bunker/shelter, `PenetratesBunker=no` compares the looked-up building in the target cell against `this+0x2E4` and zeroes damage when the shelter matches. For building technos on this same branch, `PenetratesBunker=yes` zeroes damage and returns.

Active in YR: Conditional. The branch is live for `TechnoClass+0x2E4` bunker/shelter relationships. This report found no evidence that `CanBeOccupied` garrison occupants use that relationship.

### 3.2 Garrison entry does not install the `+0x2E4` shelter back-pointer

`BuildingClass::AddGarrisonOccupant` at `0x00522910` checks `InfantryTypeClass+0xEB4` (`Occupier`), calls the infantry's vtable `+0xD4` limbo method, then appends the infantry pointer into the building's `+0x684` dynamic vector by writing `Items[count] = infantry` and incrementing `+0x694`. In the decompiled body, the only post-append writes to the infantry are two house/human-player state bytes at infantry offsets `+0x691` and `+0x690`; no building pointer is written to the infantry's `TechnoClass+0x2E4`.

Active in YR: Yes. This is the standard Occupier=yes `CanBeOccupied` entry path.

### 3.3 No immediate individual removal from `+0x684` was found

`DynamicVectorClass::Remove` at `0x006CE2D0` finds an item, decrements vector count, and shifts later items down. Xrefs to this helper are limited to unrelated vector users (`0x006CB920`, `0x006CB560`, `SuperClass::Launch`) and not `BuildingClass`, `InfantryClass::ReceiveDamage`, `FootClass::ReceiveDamage`, `TechnoClass::ReceiveDamage`, or `ObjectClass::ReceiveDamage`.

The direct garrison occupant clear observed in this slice is whole-vector clearing in `BuildingClass::SellBuilding` at `0x00457DE0`: it first writes `BuildingClass+0x69C = 0`, then if `GetOccupantCount() != 0`, iterates occupants backward through `+0x688`, unlimbos or destroys each, then calls vector clear/resize and recalculates building threat/power. That is sell/destruction ejection, not individual occupant-death cleanup.

Active in YR: Yes for sell/destruction clear; No evidence found for individual direct-death removal from live `CanBeOccupied` garrisons.

### 3.4 Count, fire index, pips, and fire eligibility consequences

Because no immediate individual removal path was found, a hypothetical directly killed limbo garrison occupant would not decrement `BuildingClass+0x694` or compact `+0x688` in this slice. `BuildingClass::IsOccupied` is count-based (`CanBeOccupied && CanOccupyFire && GetOccupantCount() > 0`), so a stale occupant pointer would keep the building fire-eligible until the vector is cleared by another path.

`TechnoClass::Fire_At` advances `BuildingClass+0x69C` only after a garrisoned building fires and after bullet/effect creation succeeds: `fire_index = (fire_index + 1) % GetOccupantCount()`. There is no death-removal normalization observed for the index. `BuildingClass::SellBuilding` resets `+0x69C` to zero before ejection.

The app-facing pip implication for Rust is straightforward: pips are driven by the passenger list/count. Matching the binary means a normal `PenetratesBunker` blast must not remove a `CanBeOccupied` occupant pip; only sell/destruction/explicit ejection should.

Active in YR: Yes for count-based fire eligibility and fire-index advance. Conditional for stale-pointer consequences, because the direct-kill caller was not found in standard garrison play.

### 3.5 Kill credit implication

`TechnoClass::RecordKill` at `0x00702D40` reads `BuildingClass+0x688 + CurrentFireIdx*4` (`+0x69C`) when the credited killer is an occupied building. It does not validate that the selected occupant is alive in this slice. If an impossible/stale occupant remained in the vector and the current index selected it, the kill-credit path would still consult that occupant pointer's type data.

Active in YR: Yes for occupied-building kill-credit lookup; Conditional for stale-dead occupant involvement because no standard direct-death path was found.

## 4. INI Keys

| Key | Source | Default / stock use | Effect in this slice | Active in YR |
|---|---|---|---|---|
| `PenetratesBunker` | `rulesmd.ini`; read at `0x0075D53C` into `WarheadTypeClass+0x146` | default false; stock true on `[Parasite]`, `[Super]`, `[Crush]`, `[DemobombWH]`, `[DiskWH]` | gates bunker/shelter damage routing in `TechnoClass::ReceiveDamage`; no garrison-vector removal | Conditional |
| `CanBeOccupied` | building type | stock civilian buildings | enables building garrison path | Yes |
| `CanOccupyFire` | building type | stock YR occupied buildings commonly set yes | fire eligibility with count > 0 | Yes |
| `MaxNumberOccupants` | building type | stock values vary, common 10 | capacity and pip slots | Yes |
| `ShowOccupantPips` | building type | default true in current Rust; binary docs place at type `+0x1584` | visual pips; not involved in removal | Yes |

## 5. Integration Points

| Function | Role | Evidence | Active in YR |
|---|---|---|---|
| `WarheadTypeClass::ReadINI_Body` `0x0075D3A0` | parses `PenetratesBunker` | string xref `0x00847E08`, store to `+0x146` | Yes |
| `TechnoClass::ReceiveDamage` `0x00701900` | reads `PenetratesBunker` for `+0x2E4` bunker/shelter damage routing | decompile plus assembly around `0x00701BBE` | Conditional |
| `BuildingClass::AddGarrisonOccupant` `0x00522910` | limbo + append to `+0x684`; no `+0x2E4` write | decompile | Yes |
| `DynamicVectorClass::Remove` `0x006CE2D0` | individual vector remove helper | decompile plus xrefs; no garrison/damage callers found | No for target path |
| `BuildingClass::SellBuilding` `0x00457DE0` | resets `+0x69C`, backward ejection, whole-vector clear | decompile plus assembly around `0x00457DEB` | Yes |
| `TechnoClass::Fire_At` `0x006FDD50` | advances `+0x69C` after successful garrison shot | decompile | Yes |
| `TechnoClass::RecordKill` `0x00702D40` | reads occupant at `+0x688/+0x69C` for occupied-building kill credit | decompile plus assembly around `0x00702FC5` | Yes |

## 6. Current Rust Implementation Status

Rust models garrison occupants as `PassengerCargo.passengers` plus `PassengerCargo.garrison_fire_index` in `src/sim/passenger.rs`. The comment already matches the verified binary offset `BuildingClass+0x69C`, not `+0x664`.

`src/sim/combat/mod.rs` selects garrison shooters from `cargo.passengers[garrison_fire_index % count]` and advances the index after shots. It also skips `passenger_role.is_inside_transport()` as normal targets, matching the fact that limbo garrison occupants are not area-damage cell occupants.

`src/rules/warhead_type.rs` currently does not parse `PenetratesBunker`; this is a bunker/shelter parity gap, but not a reason to remove civilian garrison occupants on direct blast. `src/app_ui_overlays.rs` draws occupant pips from `cargo.passengers.len()` and occupant type data.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `PenetratesBunker` INI read | verified | `0x0075D53C`, string `0x00847E08`, `rulesmd.ini` stock keys | none |
| `TechnoClass::ReceiveDamage` `+0x2E4` bunker branch | verified | `0x00701900`, assembly around `0x00701BBE` | full bunker lifecycle out of scope |
| `CanBeOccupied` add path | verified | `0x00522910` | none |
| Individual `+0x684` occupant removal on direct death | verified-negative | `DynamicVectorClass::Remove` xrefs; damage function decompiles; no caller found | runtime-only artificial direct-damage test could validate stale pointer behavior |
| Sell/destruction vector clear | verified | `0x00457DE0` | exact ejection placement remains slot 4 scope |
| Count/fire-index state after individual death | verified-negative/conditional | no individual removal; `GetOccupantCount` `0x004581F0`; `Fire_At` `0x006FDD50` | direct artificial caller not found |
| Pips | touched-not-exhausted | count/list-driven Rust surface; binary ShowOccupantPips already documented elsewhere | binary draw path not re-investigated here |
| Kill credit stale-pointer consequence | touched-not-exhausted | `RecordKill` `0x00702D40` reads `+0x688/+0x69C` | full shooter-vs-current-index timing belongs to slot 3 |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Where is PenetratesBunker parsed? -> WarheadTypeClass+0x146 at ReadINI body.` (evidence: `0x0075D53C`, string `0x00847E08`)
- `[RESOLVED] OQ2 - Where is PenetratesBunker used? -> TechnoClass::ReceiveDamage reads +0x146 only under the +0x2E4 bunker/shelter branch in this slice.` (evidence: `0x00701900`, assembly around `0x00701BBE`)
- `[RESOLVED] OQ3 - Does AddGarrisonOccupant set the +0x2E4 shelter pointer on infantry? -> No observed write; it limbos, appends to +0x684, and touches only unrelated infantry bytes after append.` (evidence: `0x00522910`)
- `[RESOLVED] OQ4 - Does individual death call DynamicVectorClass::Remove on the building occupant vector? -> No garrison/damage xrefs found for the remove helper.` (evidence: `0x006CE2D0` xrefs)
- `[RESOLVED] OQ5 - What updates +0x694 on normal garrison add/clear? -> add increments at `0x00522910`; sell/destruction whole-clear clears/resizes vector at `0x00457DE0`.` (evidence: `0x00522910`, `0x00457DE0`)
- `[RESOLVED] OQ6 - What updates current fire index? -> `Fire_At` advances `+0x69C` after successful shot; `SellBuilding` resets it to 0 before ejection.` (evidence: `0x006FDD50`, `0x00457DEB`)
- `[RESOLVED] OQ7 - Is +0x664 the current fire index? -> Not in this evidence; `+0x69C` is current fire index for garrison fire.` (evidence: `0x00452742`, `0x00453867`, `0x006FDD50`, `0x00702FC5`)
- `[RESOLVED] OQ8 - Do normal area detonations enumerate limbo garrison occupants? -> Prior warhead docs show area damage walks cell object lists; AddGarrisonOccupant limbos occupants before adding to vector.` (evidence: `WARHEAD_DETONATE_GHIDRA_REPORT.md`, `0x00522910`)
- `[RESOLVED] OQ9 - Does kill credit validate selected garrison occupant liveness? -> No liveness check observed before reading occupant type via `+0x688/+0x69C`.` (evidence: `0x00702D40`, assembly around `0x00702FC5`)
- `[DEFERRED] OQ10 - What happens if a debugger directly calls ReceiveDamage on a limbo garrison occupant pointer and then the building fires?` (category: needs-runtime-debugger; reason: no standard YR caller was found; next-step-if-pursued: runtime harness with artificial direct damage to a limbo occupant)
- `[DEFERRED] OQ11 - Exact bunker occupant +0x2E4 lifecycle.` (category: out-of-scope; reason: separate bunker slot; next-step-if-pursued: trace bunker entry/exit/clear)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `PenetratesBunker` does not remove `CanBeOccupied` garrison occupants from `BuildingClass+0x684`; the flag is a `TechnoClass+0x2E4` bunker/shelter damage gate. | `0x00701900`, `0x00701BBE`, `0x00522910` | Rust has no `PenetratesBunker` field yet; garrison target skipping already prevents normal inside-occupant hits | `src/rules/warhead_type.rs`, `src/sim/combat/mod.rs`, future bunker code | Do not add blast-driven removal from `PassengerCargo.passengers` for civilian/building garrisons. Future `PenetratesBunker` support should target bunker/shelter routing, not garrison pips/count. | A garrisoned civilian building hit by stock `[DiskWH]` or `[DemobombWH]` keeps the same occupant count and pips unless the building itself is destroyed/sold. Proposed test: `penetrates_bunker_warhead_does_not_remove_canbeoccupied_garrison_occupant` | High risk of falsely killing infantry/pips if the INI comment is applied to all garrisons. |
| Individual occupant death was not found to compact the building occupant vector or normalize current fire index. | `DynamicVectorClass::Remove` `0x006CE2D0` xrefs; `0x00457DE0` whole-clear only; `0x006FDD50` index advance only | Current Rust can remove passengers via `PassengerCargo::disembark`, but normal combat does not target inside passengers | `src/sim/passenger.rs`, `src/sim/combat/mod.rs` | Keep removal tied to explicit unload/sell/destruction paths unless a separate verified path is found. If an artificial direct-damage API kills an inside occupant, do not silently compact garrison cargo as a parity assumption. | Direct combat scan should skip inside occupants and leave `cargo.passengers` unchanged. Proposed test: `garrison_inside_occupant_is_not_area_damage_target` | Do not add a global "on infantry death remove from every cargo vec" cleanup without binary evidence; it changes pips/fire eligibility. |
| `+0x69C`, not `+0x664`, is the current garrison fire index used by weapon selection, muzzle port selection, firing advance, sell reset, and kill-credit lookup. | `0x00452742`, `0x00453867`, `0x00457DEB`, `0x006FDD50`, `0x00702FC5` | Rust comment already says `+0x69C`; older doc wording is stale | `src/sim/passenger.rs`, docs | Keep `garrison_fire_index` semantics as `(idx + 1) % count` after successful shot; do not wire behavior to a separate `+0x664` concept. | Three occupants fire in round-robin order and index resets only on sell/destruction clear. Proposed test: `garrison_fire_index_uses_69c_round_robin_and_sell_reset` | Wrong offset leads to bogus stale-doc implementation and incorrect kill-credit/pip reasoning. |

### Negative Facts / Do Not Do

- Do not implement `PenetratesBunker=yes` as "damage all infantry in any occupied building." Evidence: binary reads `WarheadTypeClass+0x146` only in the `TechnoClass+0x2E4` bunker/shelter branch in this slice (`0x00701900`).
- Do not remove or compact `BuildingClass+0x684` on ordinary building damage. Evidence: `BuildingClass::ReceiveDamage` only calls `SellBuilding` on building destruction for `CanBeOccupied`; no normal-damage vector removal path found (`0x00442230`, `0x00457DE0`).
- Do not use `BuildingClass+0x664` as the current garrison fire index. Evidence: weapon/fire-coord/fire/kill-credit code reads `+0x69C`.
- Do not assume `DynamicVectorClass::Remove` is used for live garrison occupant death. Evidence: xrefs to `0x006CE2D0` are unrelated to `BuildingClass`/damage in this slice.
- Do not make pips disappear for `PenetratesBunker` hits unless the building is sold/destroyed or an explicitly verified unload/removal path runs. Evidence: pips/count are vector/list based; no target-path decrement was found.

### Stale Docs / Follow-up Docs

- `docs/research/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`
  - Replace line 19 wording: "`0x664` ... (probable) Index for garrison fire rotation" with "`0x69C` is the verified current garrison fire index; `0x664` was not confirmed as this index."
  - Replace open question 2 with: "Resolved by `GARRISON_OCCUPANT_DEATH_REMOVAL_PENETRATESBUNKER_GHIDRA_REPORT.md`: no standard YR `PenetratesBunker` path removes `CanBeOccupied` garrison occupants from `BuildingClass+0x684`; `PenetratesBunker` gates `TechnoClass+0x2E4` bunker/shelter damage routing."
  - Replace open question 3 with: "Resolved by later garrison reports: current garrison fire index is `BuildingClass+0x69C`; weapon selection, fire coords, `Fire_At` advance, sell reset, and kill-credit lookup use that field."

## Sources

- Ghidra: `0x0075D3A0`, `0x00701900`, `0x00522910`, `0x006CE2D0`, `0x00442230`, `0x00457DE0`, `0x006FDD50`, `0x004526F0`, `0x00453840`, `0x00702D40`
- Docs: `GARRISON_SYSTEM_GHIDRA_REPORT.md`, `GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md`, `WARHEAD_DETONATE_GHIDRA_REPORT.md`, `WARHEADTYPECLASS_REINVESTIGATION_GHIDRA_REPORT.md`
- INI: `ini/rulesmd.ini` `[Parasite]`, `[Super]`, `[Crush]`, `[DemobombWH]`, `[DiskWH]`
- Rust scan: `src/sim/passenger.rs`, `src/sim/combat/mod.rs`, `src/rules/warhead_type.rs`, `src/app_ui_overlays.rs`
